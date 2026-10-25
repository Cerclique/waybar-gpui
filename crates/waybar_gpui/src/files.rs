//! What a program owns on disk, and how a daemon takes and gives up that
//! ownership.
//!
//! | File | Path |
//! |---|---|
//! | socket | `$XDG_RUNTIME_DIR/waybar-gpui/<id>.sock` (the system temp directory if `XDG_RUNTIME_DIR` is unset) |
//! | startup lock | `<id>.lock` next to the socket |
//! | log | `$XDG_STATE_HOME/waybar-gpui/<id>.log`, `$XDG_STATE_HOME` defaulting to `~/.local/state` |
//!
//! Every program's socket lives in the same directory so runtimes can close
//! each other's popover. This module is the only place that reads those
//! environment variables.

use std::{
    env,
    ffi::OsString,
    fs::{self, File},
    io,
    os::{
        fd::BorrowedFd,
        unix::{
            fs::MetadataExt,
            net::{UnixListener, UnixStream},
        },
    },
    path::{Path, PathBuf},
};

use crate::ipc::{self, Command, ExeId};

/// Everything a program owns on disk, resolved once.
#[derive(Debug, Clone)]
pub struct ProgramFiles {
    id: String,
    dir: PathBuf,
    socket: PathBuf,
    lock: PathBuf,
    log: Option<PathBuf>,
}

impl ProgramFiles {
    /// The files of program `id`, from `XDG_RUNTIME_DIR`, `XDG_STATE_HOME`
    /// and `HOME`.
    pub fn from_env(id: &str) -> Self {
        let runtime = env::var_os("XDG_RUNTIME_DIR").map_or_else(env::temp_dir, PathBuf::from);
        Self::new(id, &runtime, state_home(env::var_os("XDG_STATE_HOME"), env::var_os("HOME")))
    }

    /// The files of program `id` with `root` standing in for both
    /// `XDG_RUNTIME_DIR` and `XDG_STATE_HOME`.
    #[cfg(test)]
    pub fn under(root: &Path, id: &str) -> Self {
        Self::new(id, root, Some(root.to_owned()))
    }

    fn new(id: &str, runtime: &Path, state_home: Option<PathBuf>) -> Self {
        let dir = runtime.join("waybar-gpui");
        Self {
            id: id.to_owned(),
            socket: dir.join(format!("{id}.sock")),
            lock: dir.join(format!("{id}.lock")),
            log: state_home.map(|state| state.join("waybar-gpui").join(format!("{id}.log"))),
            dir,
        }
    }

    /// The program ID, which prefixes every log line.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The directory every program's socket lives in.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn socket(&self) -> &Path {
        &self.socket
    }

    /// Where a spawned daemon's stderr goes; `None` without `XDG_STATE_HOME`
    /// or `HOME`.
    pub fn log(&self) -> Option<&Path> {
        self.log.as_deref()
    }

    /// Makes this process the program's daemon: binds the socket unless
    /// another daemon answers on it (`daemon already running`, kind
    /// `AddrInUse`), claims the log, and forwards every command it receives to
    /// `tx` from a background thread.
    ///
    /// The check, removing a stale socket file and binding happen under an
    /// advisory lock on the lock file, so daemons starting at the same moment
    /// take turns: the first binds, the others find it answering. The lock is
    /// released once bound, so a daemon being replaced holds nothing its
    /// successor waits for, and the kernel releases it if the process dies.
    ///
    /// Claiming the log empties it if `stderr` is that very file, so the log
    /// covers this daemon only; a daemon run by hand logs elsewhere and
    /// truncates nothing.
    pub fn claim(&self, stderr: BorrowedFd, tx: async_channel::Sender<Command>) -> io::Result<Ownership> {
        fs::create_dir_all(&self.dir)?;
        let listener = self.bind_unless_running()?;
        if let Some(log) = &self.log
            && let Err(e) = truncate_if_same(stderr, log)
        {
            eprintln!("{}: cannot truncate log {}: {e}", self.id, log.display());
        }
        ipc::serve(self.id.clone(), listener, ExeId::running()?, tx);
        Ok(Ownership { socket: self.socket.clone() })
    }

    fn bind_unless_running(&self) -> io::Result<UnixListener> {
        let already_running = || io::Error::new(io::ErrorKind::AddrInUse, "daemon already running");
        let lock = File::options().create(true).truncate(false).write(true).open(&self.lock)?;
        lock.lock()?;
        if UnixStream::connect(&self.socket).is_ok() {
            return Err(already_running());
        }
        let _ = fs::remove_file(&self.socket);
        // Only a daemon that skips the lock (one from before it) can win here.
        UnixListener::bind(&self.socket)
            .map_err(|e| if e.kind() == io::ErrorKind::AddrInUse { already_running() } else { e })
    }
}

/// A daemon's hold on its program's socket, from [`ProgramFiles::claim`].
#[must_use = "the socket stays until ownership is given up"]
pub struct Ownership {
    socket: PathBuf,
}

impl Ownership {
    /// Removes the socket, so clients find no daemon at once instead of one
    /// that is quitting.
    pub fn give_up(self) {
        let _ = fs::remove_file(&self.socket);
    }
}

/// `$XDG_STATE_HOME`, defaulting to `~/.local/state`; empty values count as
/// unset.
fn state_home(state_home: Option<OsString>, home: Option<OsString>) -> Option<PathBuf> {
    match state_home.filter(|dir| !dir.is_empty()) {
        Some(dir) => Some(PathBuf::from(dir)),
        None => Some(PathBuf::from(home.filter(|dir| !dir.is_empty())?).join(".local/state")),
    }
}

/// Truncates the file open on `fd` if it is the file at `path`. Writes
/// through an append-mode `fd` then start at the beginning.
fn truncate_if_same(fd: BorrowedFd, path: &Path) -> io::Result<()> {
    let at_path = match fs::metadata(path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        result => result?,
    };
    let file = File::from(fd.try_clone_to_owned()?);
    let open = file.metadata()?;
    if (open.dev(), open.ino()) == (at_path.dev(), at_path.ino()) {
        file.set_len(0)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        io::Write as _,
        os::fd::AsFd,
        process,
        sync::Barrier,
        thread,
    };

    use super::*;
    use crate::ipc::{Connection, Target, no_daemon};

    /// A fresh directory standing in for the runtime and state directories.
    struct Root(PathBuf);

    impl Root {
        fn new(name: &str) -> Self {
            let path = env::temp_dir().join(format!("waybar-gpui-{name}-{}", process::id()));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for Root {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// A descriptor that isn't the log, for claims that shouldn't touch it.
    fn elsewhere(root: &Root) -> File {
        File::create(root.0.join("elsewhere")).unwrap()
    }

    #[test]
    fn state_home_follows_xdg_state_home() {
        let some = |s: &str| Some(OsString::from(s));
        assert_eq!(state_home(some("/state"), some("/home/me")), Some(PathBuf::from("/state")));
        assert_eq!(state_home(None, some("/home/me")), Some(PathBuf::from("/home/me/.local/state")));
        assert_eq!(state_home(some(""), some("/home/me")), Some(PathBuf::from("/home/me/.local/state")));
        assert_eq!(state_home(None, None), None);
    }

    #[test]
    fn files_are_named_by_the_program_id() {
        let files = ProgramFiles::new("my-panels", Path::new("/run/user/1000"), Some("/state".into()));
        assert_eq!(files.dir(), Path::new("/run/user/1000/waybar-gpui"));
        assert_eq!(files.socket(), Path::new("/run/user/1000/waybar-gpui/my-panels.sock"));
        assert_eq!(files.lock, Path::new("/run/user/1000/waybar-gpui/my-panels.lock"));
        assert_eq!(files.log(), Some(Path::new("/state/waybar-gpui/my-panels.log")));
        assert_eq!(ProgramFiles::new("my-panels", Path::new("/run"), None).log(), None);
    }

    /// The daemon side of the exchange over the claimed socket: a client
    /// reading the identity, then one writing without reading.
    #[test]
    fn claimed_socket_answers_identity_then_forwards_commands() {
        let root = Root::new("claim-serve");
        let files = ProgramFiles::under(&root.0, "test");
        let (tx, rx) = async_channel::unbounded();
        let ownership = files.claim(elsewhere(&root).as_fd(), tx).unwrap();

        let mut client = Connection::open(&files).unwrap();
        assert_eq!(client.identity(), Some(ExeId::of(&env::current_exe().unwrap()).unwrap()));
        let clock = Target { panel: "clock".into(), args: vec!["Paris".into()] };
        client.send(&Command::Toggle(clock.clone())).unwrap();
        assert_eq!(rx.recv_blocking(), Ok(Command::Toggle(clock)));

        ipc::send(&files, &Command::Close).unwrap();
        assert_eq!(rx.recv_blocking(), Ok(Command::Close));
        ownership.give_up();
    }

    /// Daemons starting at the same moment: exactly one claims, every other
    /// one finds it answering. A stale socket file nobody answers on doesn't
    /// count.
    #[test]
    fn only_one_of_simultaneous_claims_wins() {
        let root = Root::new("claim-race");
        let files = ProgramFiles::under(&root.0, "race");
        fs::create_dir_all(files.dir()).unwrap();
        let stderr = elsewhere(&root);
        for _ in 0..20 {
            // Left by a crashed daemon: a file with no listener behind it.
            drop(UnixListener::bind(files.socket()).unwrap());
            let start = Barrier::new(4);
            let results: Vec<_> = thread::scope(|s| {
                let handles: Vec<_> = (0..4)
                    .map(|_| {
                        s.spawn(|| {
                            start.wait();
                            files.claim(stderr.as_fd(), async_channel::unbounded().0)
                        })
                    })
                    .collect();
                handles.into_iter().map(|h| h.join().unwrap()).collect()
            });
            let (won, lost): (Vec<_>, Vec<_>) = results.into_iter().partition(Result::is_ok);
            assert_eq!(won.len(), 1);
            for e in lost.into_iter().map(Result::err).map(Option::unwrap) {
                assert_eq!(e.kind(), io::ErrorKind::AddrInUse);
                assert_eq!(e.to_string(), "daemon already running");
            }
            assert!(Connection::open(&files).is_ok());
            won.into_iter().for_each(|o| o.unwrap().give_up());
        }
    }

    #[test]
    fn claim_replaces_a_stale_socket() {
        let root = Root::new("claim-stale");
        let files = ProgramFiles::under(&root.0, "stale");
        fs::create_dir_all(files.dir()).unwrap();
        drop(UnixListener::bind(files.socket()).unwrap());
        assert!(Connection::open(&files).is_err_and(|e| no_daemon(&e)));

        let ownership = files.claim(elsewhere(&root).as_fd(), async_channel::unbounded().0).unwrap();
        assert!(Connection::open(&files).is_ok());
        ownership.give_up();
    }

    #[test]
    fn giving_up_removes_the_socket() {
        let root = Root::new("claim-give-up");
        let files = ProgramFiles::under(&root.0, "quit");
        let ownership = files.claim(elsewhere(&root).as_fd(), async_channel::unbounded().0).unwrap();
        assert!(files.socket().exists());
        ownership.give_up();
        assert!(!files.socket().exists());
        assert!(Connection::open(&files).is_err_and(|e| no_daemon(&e)));
        // The next daemon claims without a stale file to remove.
        files.claim(elsewhere(&root).as_fd(), async_channel::unbounded().0).unwrap().give_up();
    }

    /// The log is emptied only when stderr is that very file, and later
    /// appends through stderr start from the beginning.
    #[test]
    fn claim_empties_the_log_only_when_stderr_is_the_log() {
        let root = Root::new("claim-log");
        let files = ProgramFiles::under(&root.0, "log");
        let path = files.log().unwrap();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut log = File::options().create(true).append(true).open(path).unwrap();
        writeln!(log, "previous daemon, longer line").unwrap();

        files.claim(elsewhere(&root).as_fd(), async_channel::unbounded().0).unwrap().give_up();
        assert_eq!(fs::read_to_string(path).unwrap(), "previous daemon, longer line\n");

        let won = files.claim(log.as_fd(), async_channel::unbounded().0).unwrap();
        writeln!(log, "this daemon").unwrap();
        assert_eq!(fs::read_to_string(path).unwrap(), "this daemon\n");

        // A daemon that lost the race leaves the winner's log alone.
        let lost = files.claim(log.as_fd(), async_channel::unbounded().0);
        assert!(lost.is_err());
        assert_eq!(fs::read_to_string(path).unwrap(), "this daemon\n");
        won.give_up();
    }

    #[test]
    fn claim_without_a_log_file_creates_none() {
        let root = Root::new("claim-no-log");
        let files = ProgramFiles::under(&root.0, "nolog");
        files.claim(elsewhere(&root).as_fd(), async_channel::unbounded().0).unwrap().give_up();
        assert!(!files.log().unwrap().exists());
    }
}
