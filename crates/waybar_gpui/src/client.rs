//! The click client: makes sure a daemon of *this* program receives the
//! command. `toggle` sends it to a daemon running this very executable,
//! replaces a daemon of another executable (`quit`, wait for it to go, start
//! ours), and starts one when none runs; `close` and `quit` have nothing to do
//! without a daemon.

use std::{
    env,
    fs::{self, File, OpenOptions},
    io,
    os::unix::{net::UnixStream, process::CommandExt},
    path::Path,
    process::{self, Stdio},
    thread,
    time::Duration,
};

use crate::{
    files::ProgramFiles,
    ipc::{Command, Connection, ExeId, Target},
};

/// How long `toggle` waits for a daemon from another executable to quit.
const REPLACE_TIMEOUT: Duration = Duration::from_secs(2);

/// The executable whose daemon the client wants: who it is, and how to start
/// its daemon. [`CurrentExe`] in production; tests start fake daemons.
pub trait Executable {
    /// The identity its daemon would report; `None` if it can't be known.
    fn identity(&self) -> Option<ExeId>;

    /// Starts its daemon, opening `target` at once.
    fn start_daemon(&self, files: &ProgramFiles, target: &Target) -> io::Result<()>;
}

/// This process's own executable, its daemon re-run as `current_exe() daemon
/// <target>`, detached, stderr appended to the log.
pub struct CurrentExe;

impl Executable for CurrentExe {
    fn identity(&self) -> Option<ExeId> {
        env::current_exe().and_then(|exe| ExeId::of(&exe)).ok()
    }

    fn start_daemon(&self, files: &ProgramFiles, target: &Target) -> io::Result<()> {
        let log = files.log().and_then(|path| {
            open_log(path).map_err(|e| eprintln!("{}: cannot open log {}: {e}", files.id(), path.display())).ok()
        });
        let args: Vec<String> = ["daemon"].into_iter().chain(target.words()).map(str::to_owned).collect();
        spawn_detached(&env::current_exe()?, &args, log).map(drop)
    }
}

/// Sends the toggle to the running daemon if it runs `exe`; otherwise
/// replaces it, so a rebuilt or reinstalled program shows its own panels.
/// Starts `exe`'s daemon with `target` when none runs.
pub fn toggle(files: &ProgramFiles, target: Target, exe: &impl Executable) -> io::Result<()> {
    let mut daemon = match Connection::open(files) {
        Err(e) if no_daemon(&e) => return exe.start_daemon(files, &target),
        result => result?,
    };
    match Decision::new(daemon.identity(), exe.identity()) {
        Decision::Send => daemon.send(&Command::Toggle(target)),
        Decision::Replace => {
            eprintln!("{}: replacing the daemon of another executable", files.id());
            daemon.send(&Command::Quit)?;
            wait_gone(files.socket(), REPLACE_TIMEOUT)?;
            exe.start_daemon(files, &target)
        }
    }
}

/// Closes the daemon's popover; nothing to do without a daemon.
pub fn close(files: &ProgramFiles) -> io::Result<()> {
    send_if_running(files, &Command::Close)
}

/// Stops the daemon; nothing to do without a daemon.
pub fn quit(files: &ProgramFiles) -> io::Result<()> {
    send_if_running(files, &Command::Quit)
}

fn send_if_running(files: &ProgramFiles, command: &Command) -> io::Result<()> {
    match Connection::open(files) {
        Err(e) if no_daemon(&e) => Ok(()),
        result => result?.send(command),
    }
}

/// What `toggle` does with the daemon it reached.
#[derive(Debug, PartialEq, Eq)]
enum Decision {
    /// Same executable: send it the command.
    Send,
    /// Another executable, or one too old to say: `quit` it and start ours.
    Replace,
}

impl Decision {
    /// `daemon` is the identity the daemon replied with, `own` the one ours
    /// would. Without our own identity there is nothing to compare, so the
    /// daemon stays.
    fn new(daemon: Option<ExeId>, own: Option<ExeId>) -> Self {
        match (daemon, own) {
            (_, None) => Self::Send,
            (Some(daemon), Some(own)) if daemon == own => Self::Send,
            _ => Self::Replace,
        }
    }
}

/// A connect error meaning no daemon listens: nothing is open and nothing runs.
fn no_daemon(e: &io::Error) -> bool {
    matches!(e.kind(), io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused)
}

/// Polls until nothing answers on `socket`, for at most `timeout`. The probe
/// writes nothing, which a daemon ignores.
fn wait_gone(socket: &Path, timeout: Duration) -> io::Result<()> {
    const STEP: Duration = Duration::from_millis(20);
    let mut waited = Duration::ZERO;
    while UnixStream::connect(socket).is_ok() {
        if waited >= timeout {
            return Err(io::Error::new(io::ErrorKind::TimedOut, "the old daemon did not quit"));
        }
        thread::sleep(STEP);
        waited += STEP;
    }
    Ok(())
}

/// Creates the log's directory and opens the log for appending. `toggle`
/// doesn't truncate it: a second `toggle` racing this one would wipe the log
/// of the daemon that wins. The winner empties it in [`ProgramFiles::claim`].
fn open_log(path: &Path) -> io::Result<File> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    OpenOptions::new().create(true).append(true).open(path)
}

/// Starts `exe args…` detached, stderr into `log` (or discarded).
fn spawn_detached(exe: &Path, args: &[String], log: Option<File>) -> io::Result<process::Child> {
    process::Command::new(exe)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(log.map_or_else(Stdio::null, Stdio::from))
        // Own process group, so it outlives Waybar reloads killing on-click children.
        .process_group(0)
        .spawn()
}

#[cfg(test)]
mod tests {
    use std::{
        io::{BufRead as _, BufReader},
        os::unix::net::UnixListener,
        sync::{Mutex, mpsc},
        time::Instant,
    };

    use std::path::PathBuf;

    use super::*;
    use crate::ipc;

    /// A fresh directory standing in for the runtime and state directories.
    struct Root(PathBuf);

    impl Root {
        fn new(name: &str) -> Self {
            let path = env::temp_dir().join(format!("waybar-gpui-client-{name}-{}", process::id()));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn files(&self) -> ProgramFiles {
            let files = ProgramFiles::under(&self.0, "test");
            fs::create_dir_all(files.dir()).unwrap();
            files
        }
    }

    impl Drop for Root {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    const OURS: ExeId = ExeId::new(1, 10);
    const REBUILT: ExeId = ExeId::new(1, 11);

    /// A daemon on `files`' socket replying with `identity`, or never replying
    /// (one from before the identity reply). Returns the commands it receives.
    /// On `quit` it removes its socket, as a real daemon does.
    fn fake_daemon(files: &ProgramFiles, identity: Option<ExeId>) -> mpsc::Receiver<Command> {
        let listener = UnixListener::bind(files.socket()).unwrap();
        let socket = files.socket().to_owned();
        let (received, rx) = mpsc::channel();
        let (tx, commands) = async_channel::unbounded();
        match identity {
            Some(identity) => ipc::serve("fake".into(), listener, identity, tx),
            None => {
                thread::spawn(move || {
                    for stream in listener.incoming().flatten() {
                        let mut line = String::new();
                        let _ = BufReader::new(stream).read_line(&mut line);
                        match line.as_str() {
                            "" => continue,
                            "quit\n" => tx.send_blocking(Command::Quit).unwrap(),
                            _ => panic!("unexpected command {line:?}"),
                        }
                    }
                });
            }
        }
        thread::spawn(move || {
            while let Ok(command) = commands.recv_blocking() {
                if command == Command::Quit {
                    let _ = fs::remove_file(&socket);
                }
                if received.send(command).is_err() {
                    break;
                }
            }
        });
        rx
    }

    /// An executable with a chosen identity that records the daemons it
    /// starts, and whether anything answered on the socket at that moment.
    struct FakeExe {
        identity: Option<ExeId>,
        started: Mutex<Vec<(Target, bool)>>,
    }

    impl FakeExe {
        fn new(identity: Option<ExeId>) -> Self {
            Self { identity, started: Mutex::default() }
        }

        fn started(&self) -> Vec<(Target, bool)> {
            self.started.lock().unwrap().clone()
        }
    }

    impl Executable for FakeExe {
        fn identity(&self) -> Option<ExeId> {
            self.identity
        }

        fn start_daemon(&self, files: &ProgramFiles, target: &Target) -> io::Result<()> {
            let answering = UnixStream::connect(files.socket()).is_ok();
            self.started.lock().unwrap().push((target.clone(), answering));
            Ok(())
        }
    }

    fn clock() -> Target {
        Target { panel: "clock".into(), args: vec!["Paris".into(), "2".into()] }
    }

    #[test]
    fn toggle_without_a_daemon_starts_one_with_the_target() {
        let root = Root::new("none");
        let files = root.files();
        let exe = FakeExe::new(Some(OURS));
        toggle(&files, clock(), &exe).unwrap();
        assert_eq!(exe.started(), [(clock(), false)]);
    }

    #[test]
    fn toggle_sends_to_a_daemon_of_the_same_executable() {
        let root = Root::new("same");
        let files = root.files();
        let received = fake_daemon(&files, Some(OURS));
        let exe = FakeExe::new(Some(OURS));
        toggle(&files, clock(), &exe).unwrap();
        assert_eq!(received.recv(), Ok(Command::Toggle(clock())));
        assert_eq!(exe.started(), []);
    }

    /// Without its own identity the client has nothing to compare: it keeps
    /// whatever daemon runs.
    #[test]
    fn toggle_without_its_own_identity_sends() {
        let root = Root::new("unknown");
        let files = root.files();
        let received = fake_daemon(&files, Some(REBUILT));
        let exe = FakeExe::new(None);
        toggle(&files, clock(), &exe).unwrap();
        assert_eq!(received.recv(), Ok(Command::Toggle(clock())));
        assert_eq!(exe.started(), []);
    }

    /// The old daemon gets `quit`, and ours starts only once it is gone.
    #[test]
    fn toggle_replaces_a_daemon_of_another_executable() {
        let root = Root::new("other");
        let files = root.files();
        let received = fake_daemon(&files, Some(REBUILT));
        let exe = FakeExe::new(Some(OURS));
        toggle(&files, clock(), &exe).unwrap();
        assert_eq!(received.recv(), Ok(Command::Quit));
        assert_eq!(exe.started(), [(clock(), false)]);
        assert!(received.try_recv().is_err());
    }

    #[test]
    fn toggle_replaces_a_daemon_that_never_says_who_it_is() {
        let root = Root::new("silent");
        let files = root.files();
        let received = fake_daemon(&files, None);
        let exe = FakeExe::new(Some(OURS));
        toggle(&files, clock(), &exe).unwrap();
        assert_eq!(received.recv(), Ok(Command::Quit));
        assert_eq!(exe.started(), [(clock(), false)]);
    }

    /// A daemon that takes `quit` but keeps answering: the client gives up
    /// after [`REPLACE_TIMEOUT`] and starts nothing.
    #[test]
    fn toggle_fails_when_the_old_daemon_does_not_go() {
        let root = Root::new("stuck");
        let files = root.files();
        let listener = UnixListener::bind(files.socket()).unwrap();
        // Kept, so the serving thread keeps answering after `quit`.
        let (tx, _commands) = async_channel::unbounded();
        ipc::serve("stuck".into(), listener, REBUILT, tx);
        let exe = FakeExe::new(Some(OURS));
        let start = Instant::now();
        let e = toggle(&files, clock(), &exe).unwrap_err();
        assert_eq!(e.kind(), io::ErrorKind::TimedOut);
        assert!(start.elapsed() >= REPLACE_TIMEOUT);
        assert_eq!(exe.started(), []);
    }

    #[test]
    fn close_and_quit_reach_a_running_daemon() {
        let root = Root::new("close");
        let files = root.files();
        let received = fake_daemon(&files, Some(OURS));
        close(&files).unwrap();
        assert_eq!(received.recv(), Ok(Command::Close));
        quit(&files).unwrap();
        assert_eq!(received.recv(), Ok(Command::Quit));
    }

    /// No socket file, or one nobody answers on (left by a crashed daemon).
    #[test]
    fn close_and_quit_without_a_daemon_do_nothing() {
        let root = Root::new("nothing");
        let files = root.files();
        close(&files).unwrap();
        quit(&files).unwrap();
        drop(UnixListener::bind(files.socket()).unwrap());
        close(&files).unwrap();
        quit(&files).unwrap();
    }

    /// The spawn path with a stand-in program: the log directory is created,
    /// stderr lands in the log, and spawning appends instead of truncating.
    #[test]
    fn spawned_stderr_is_appended_to_the_log() {
        let root = Root::new("log");
        let files = ProgramFiles::under(&root.0, "test");
        let path = files.log().unwrap();
        let spawn_and_wait = |message: &str| {
            let args = ["-c".to_owned(), format!("echo {message} >&2; echo stdout")];
            let status = spawn_detached(Path::new("/bin/sh"), &args, Some(open_log(path).unwrap())).unwrap().wait();
            assert!(status.unwrap().success());
        };
        spawn_and_wait("first run");
        assert_eq!(fs::read_to_string(path).unwrap(), "first run\n");
        spawn_and_wait("second");
        assert_eq!(fs::read_to_string(path).unwrap(), "first run\nsecond\n");
    }
}
