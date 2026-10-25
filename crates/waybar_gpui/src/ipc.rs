//! One exchange per connection, in `\n`-terminated lines of tab-separated words:
//!
//! 1. On accept the daemon writes its executable identity, `exe<TAB>dev<TAB>ino`
//!    (device and inode of `/proc/self/exe` recorded at startup, in decimal).
//! 2. The client writes one command, `toggle<TAB>panel<TAB>arg…`, `close` or
//!    `quit`, and hangs up.
//!
//! Clients that don't care who answers (`close`, `quit`, other programs closing
//! this one's popover) write their command without reading the identity.
//! `toggle` reads it first and, when it isn't its own executable's, writes `quit`
//! instead (see [`Decision`]).
//!
//! Every program's socket lives in one shared directory so runtimes can close
//! each other's popover.

use std::{
    env, fmt, fs,
    io::{self, BufRead, BufReader, Write},
    os::unix::{
        fs::MetadataExt,
        net::{UnixListener, UnixStream},
    },
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

/// How long a client waits for the daemon's identity line. A daemon from before
/// the identity reply never sends one.
const REPLY_TIMEOUT: Duration = Duration::from_secs(1);

/// A panel to open and the arguments its build function receives. Equal
/// targets are the same popover: toggling the open one closes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub panel: String,
    pub args: Vec<String>,
}

impl Target {
    /// The panel name followed by its arguments; `None` without a name.
    pub fn from_words(words: impl IntoIterator<Item = String>) -> Option<Self> {
        let mut words = words.into_iter();
        Some(Self { panel: words.next()?, args: words.collect() })
    }

    /// The inverse of [`Target::from_words`].
    pub fn words(&self) -> impl Iterator<Item = &str> {
        std::iter::once(&self.panel).chain(&self.args).map(String::as_str)
    }
}

/// For logs: `clock ["Paris", "2"]`, or `workspace` without arguments.
impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.args.as_slice() {
            [] => write!(f, "{}", self.panel),
            args => write!(f, "{} {args:?}", self.panel),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Toggle(Target),
    Close,
    Quit,
}

impl Command {
    fn parse(line: &str) -> Option<Self> {
        let mut words = line.trim_end_matches('\n').split('\t');
        match words.next()? {
            "toggle" => Target::from_words(words.map(str::to_owned)).map(Self::Toggle),
            "close" => Some(Self::Close),
            "quit" => Some(Self::Quit),
            _ => None,
        }
    }

    fn to_line(&self) -> String {
        match self {
            Self::Toggle(target) => format!("toggle\t{}\n", target.words().collect::<Vec<_>>().join("\t")),
            Self::Close => "close\n".into(),
            Self::Quit => "quit\n".into(),
        }
    }
}

/// An executable file's identity. Replacing the file (a rebuild, `cargo
/// install`) gives it a new inode; another copy is another file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExeId {
    dev: u64,
    ino: u64,
}

impl ExeId {
    /// The identity of the file at `path`, symlinks followed.
    pub fn of(path: &Path) -> io::Result<Self> {
        let meta = fs::metadata(path)?;
        Ok(Self { dev: meta.dev(), ino: meta.ino() })
    }

    /// The file this process started from, even once it is replaced on disk.
    fn running() -> io::Result<Self> {
        Self::of(Path::new("/proc/self/exe"))
    }

    fn parse(line: &str) -> Option<Self> {
        let mut words = line.trim_end_matches('\n').split('\t');
        let (Some("exe"), Some(dev), Some(ino), None) = (words.next(), words.next(), words.next(), words.next())
        else {
            return None;
        };
        Some(Self { dev: dev.parse().ok()?, ino: ino.parse().ok()? })
    }

    fn to_line(self) -> String {
        format!("exe\t{}\t{}\n", self.dev, self.ino)
    }
}

/// What `toggle` does with the daemon it reached.
#[derive(Debug, PartialEq, Eq)]
pub enum Decision {
    /// Same executable: send it the command.
    Send,
    /// Another executable, or one too old to say: `quit` it and spawn ourselves.
    Replace,
}

impl Decision {
    /// `daemon` is the identity the daemon replied with, `own` the file at the
    /// client's `current_exe()`. Without our own identity there is nothing to
    /// compare, so the daemon stays.
    pub fn new(daemon: Option<ExeId>, own: Option<ExeId>) -> Self {
        match (daemon, own) {
            (_, None) => Self::Send,
            (Some(daemon), Some(own)) if daemon == own => Self::Send,
            _ => Self::Replace,
        }
    }
}

fn dir() -> PathBuf {
    env::var_os("XDG_RUNTIME_DIR").map_or_else(env::temp_dir, PathBuf::from).join("waybar-gpui")
}

pub fn socket_path(id: &str) -> PathBuf {
    dir().join(format!("{id}.sock"))
}

/// A client's connection to a running daemon.
pub struct Connection(UnixStream);

impl Connection {
    /// Fails as [`no_daemon`] recognises when nothing listens.
    pub fn open(id: &str) -> io::Result<Self> {
        UnixStream::connect(socket_path(id)).map(Self)
    }

    /// The daemon's identity; `None` if its line is malformed or doesn't
    /// arrive within [`REPLY_TIMEOUT`].
    pub fn identity(&mut self) -> Option<ExeId> {
        self.0.set_read_timeout(Some(REPLY_TIMEOUT)).ok()?;
        let mut line = String::new();
        BufReader::new(&self.0).read_line(&mut line).ok()?;
        ExeId::parse(&line)
    }

    pub fn send(mut self, command: &Command) -> io::Result<()> {
        self.0.write_all(command.to_line().as_bytes())
    }
}

pub fn send(id: &str, command: &Command) -> io::Result<()> {
    Connection::open(id)?.send(command)
}

/// A connect error meaning no daemon listens: nothing is open and nothing runs.
pub fn no_daemon(e: &io::Error) -> bool {
    matches!(e.kind(), io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused)
}

/// Polls until no daemon answers on `id`'s socket, for at most `timeout`.
pub fn wait_gone(id: &str, timeout: Duration) -> io::Result<()> {
    const STEP: Duration = Duration::from_millis(20);
    let mut waited = Duration::ZERO;
    while UnixStream::connect(socket_path(id)).is_ok() {
        if waited >= timeout {
            return Err(io::Error::new(io::ErrorKind::TimedOut, "the old daemon did not quit"));
        }
        thread::sleep(STEP);
        waited += STEP;
    }
    Ok(())
}

/// Best effort: asks every other program's runtime to close its popover, and
/// deletes sockets nobody answers on (left by crashed programs). `include_own`
/// also asks this program's daemon (used by `preview`, which has no socket).
pub fn close_others(id: &str, include_own: bool) {
    let own = socket_path(id);
    let Ok(entries) = fs::read_dir(dir()) else { return };
    for path in entries.flatten().map(|e| e.path()) {
        if (path == own && !include_own) || path.extension().is_none_or(|ext| ext != "sock") {
            continue;
        }
        match UnixStream::connect(&path) {
            Ok(mut stream) => {
                let _ = stream.write_all(Command::Close.to_line().as_bytes());
            }
            Err(e) if e.kind() == io::ErrorKind::ConnectionRefused => {
                eprintln!("{id}: removing stale socket {}", path.display());
                let _ = fs::remove_file(&path);
            }
            Err(_) => {}
        }
    }
}

/// Binds `id`'s socket unless another daemon answers on it, then serves it.
pub fn listen(id: &str, tx: async_channel::Sender<Command>) -> io::Result<()> {
    fs::create_dir_all(dir())?;
    let listener = bind_unless_running(&socket_path(id), &dir().join(format!("{id}.lock")))?;
    // This daemon owns the program now, and with it the log.
    crate::cli::claim_log(id);
    serve(id.to_owned(), listener, ExeId::running()?, tx);
    Ok(())
}

/// Checks for a daemon on `socket`, removes a stale file and binds, holding an
/// advisory lock on `lock` for those three steps only, so daemons starting at
/// the same moment take turns: the first binds, the others find it answering.
/// The lock is released once bound, so a daemon being replaced holds nothing
/// its successor waits for, and the kernel releases it if the process dies.
fn bind_unless_running(socket: &Path, lock: &Path) -> io::Result<UnixListener> {
    let already_running = || io::Error::new(io::ErrorKind::AddrInUse, "daemon already running");
    let lock = fs::File::options().create(true).truncate(false).write(true).open(lock)?;
    lock.lock()?;
    if UnixStream::connect(socket).is_ok() {
        return Err(already_running());
    }
    let _ = fs::remove_file(socket);
    // Only a daemon that skips the lock (one from before it) can win here.
    UnixListener::bind(socket).map_err(|e| if e.kind() == io::ErrorKind::AddrInUse { already_running() } else { e })
}

/// Answers each connection with `identity`, then forwards its command.
fn serve(id: String, listener: UnixListener, identity: ExeId, tx: async_channel::Sender<Command>) {
    let identity = identity.to_line();
    thread::spawn(move || {
        for mut stream in listener.incoming().flatten() {
            // Fails if the client already hung up; its command is still readable.
            let _ = stream.write_all(identity.as_bytes());
            let mut line = String::new();
            // Nothing read: a client that only probed (`wait_gone`) or gave up.
            if BufReader::new(stream).read_line(&mut line).is_err() || line.is_empty() {
                continue;
            }
            match Command::parse(&line) {
                Some(command) => {
                    if tx.send_blocking(command).is_err() {
                        break;
                    }
                }
                None => eprintln!("{id}: ignoring unknown command {line:?}"),
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(words: &[&str]) -> Target {
        Target::from_words(words.iter().map(|w| w.to_string())).unwrap()
    }

    #[test]
    fn splits_target_words_into_panel_and_args() {
        assert_eq!(target(&["workspace"]), Target { panel: "workspace".into(), args: vec![] });
        let clock = target(&["clock", "Paris", "2"]);
        assert_eq!(clock, Target { panel: "clock".into(), args: vec!["Paris".into(), "2".into()] });
        assert_eq!(clock.words().collect::<Vec<_>>(), ["clock", "Paris", "2"]);
        assert_eq!(Target::from_words(Vec::new()), None);
    }

    #[test]
    fn displays_target_for_logs() {
        assert_eq!(target(&["workspace"]).to_string(), "workspace");
        assert_eq!(target(&["clock", "New York", "2"]).to_string(), r#"clock ["New York", "2"]"#);
    }

    #[test]
    fn round_trips_commands() {
        assert_eq!(Command::Toggle(target(&["clock", "New York"])).to_line(), "toggle\tclock\tNew York\n");
        for command in [
            Command::Toggle(target(&["workspace"])),
            Command::Toggle(target(&["clock", "Paris", "2"])),
            Command::Close,
            Command::Quit,
        ] {
            assert_eq!(Command::parse(&command.to_line()), Some(command));
        }
    }

    #[test]
    fn rejects_unknown_and_empty_toggle() {
        assert_eq!(Command::parse("bogus\n"), None);
        assert_eq!(Command::parse("toggle\n"), None);
    }

    #[test]
    fn round_trips_identity() {
        let id = ExeId { dev: 66306, ino: u64::MAX };
        assert_eq!(id.to_line(), "exe\t66306\t18446744073709551615\n");
        assert_eq!(ExeId::parse(&id.to_line()), Some(id));
    }

    #[test]
    fn rejects_malformed_identity() {
        for line in ["", "\n", "exe\t1\n", "exe\t1\t2\t3\n", "exe\t1\tx\n", "dev\t1\t2\n", "toggle\tclock\n"] {
            assert_eq!(ExeId::parse(line), None, "{line:?}");
        }
    }

    #[test]
    fn replaces_only_a_different_or_silent_daemon() {
        let own = Some(ExeId { dev: 1, ino: 10 });
        let rebuilt = Some(ExeId { dev: 1, ino: 11 });
        let elsewhere = Some(ExeId { dev: 2, ino: 10 });
        assert_eq!(Decision::new(own, own), Decision::Send);
        assert_eq!(Decision::new(rebuilt, own), Decision::Replace);
        assert_eq!(Decision::new(elsewhere, own), Decision::Replace);
        assert_eq!(Decision::new(None, own), Decision::Replace);
        assert_eq!(Decision::new(own, None), Decision::Send);
    }

    #[test]
    fn running_identity_is_the_current_exe() {
        let own = ExeId::of(&env::current_exe().unwrap()).unwrap();
        assert_eq!(ExeId::running().unwrap(), own);
    }

    /// The daemon side of the exchange over a real socket in a temp directory:
    /// a client reading the identity, then one writing without reading.
    #[test]
    fn serves_identity_then_reads_command() {
        let dir = env::temp_dir().join(format!("waybar-gpui-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.sock");
        let _ = fs::remove_file(&path);
        let identity = ExeId { dev: 7, ino: 42 };
        let (tx, rx) = async_channel::unbounded();
        serve("test".into(), UnixListener::bind(&path).unwrap(), identity, tx);

        let mut client = Connection(UnixStream::connect(&path).unwrap());
        assert_eq!(client.identity(), Some(identity));
        client.send(&Command::Toggle(target(&["clock", "Paris"]))).unwrap();
        assert_eq!(rx.recv_blocking(), Ok(Command::Toggle(target(&["clock", "Paris"]))));

        Connection(UnixStream::connect(&path).unwrap()).send(&Command::Close).unwrap();
        assert_eq!(rx.recv_blocking(), Ok(Command::Close));
        fs::remove_dir_all(&dir).unwrap();
    }

    /// Daemons starting at the same moment: exactly one binds, every other one
    /// finds it answering. A stale socket file nobody answers on doesn't count.
    #[test]
    fn only_one_of_simultaneous_daemons_binds() {
        let dir = env::temp_dir().join(format!("waybar-gpui-race-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let (socket, lock) = (dir.join("race.sock"), dir.join("race.lock"));
        for _ in 0..50 {
            // Left by a crashed daemon: a file with no listener behind it.
            drop(UnixListener::bind(&socket).unwrap());
            let start = std::sync::Barrier::new(4);
            let results: Vec<_> = thread::scope(|s| {
                let handles: Vec<_> = (0..4)
                    .map(|_| {
                        s.spawn(|| {
                            start.wait();
                            bind_unless_running(&socket, &lock)
                        })
                    })
                    .collect();
                handles.into_iter().map(|h| h.join().unwrap()).collect()
            });
            let (bound, lost): (Vec<_>, Vec<_>) = results.into_iter().partition(Result::is_ok);
            assert_eq!(bound.len(), 1);
            for e in lost.into_iter().map(Result::unwrap_err) {
                assert_eq!(e.to_string(), "daemon already running");
            }
            assert!(UnixStream::connect(&socket).is_ok());
            drop(bound);
            fs::remove_file(&socket).unwrap();
        }
        fs::remove_dir_all(&dir).unwrap();
    }
}
