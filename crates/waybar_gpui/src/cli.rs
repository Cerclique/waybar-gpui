//! The author's command line, owned entirely by the SDK.

use std::{
    env,
    ffi::OsString,
    fs::{self, File},
    io,
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::{self, ExitCode, Stdio},
    time::Duration,
};

use crate::{
    App,
    ipc::{self, Command, Decision, ExeId, Target},
    runtime::{self, Mode},
};

/// A parsed command line.
#[derive(Debug, PartialEq, Eq)]
enum Invocation {
    Toggle(Target),
    Close,
    Quit,
    Daemon(Option<Target>),
    Preview(Target),
    Help,
}

/// Why a command line was rejected; the usage text follows the message.
#[derive(Debug, PartialEq, Eq)]
struct UsageError(String);

fn parse(args: &[String], panels: &[&str]) -> Result<Invocation, UsageError> {
    let target = |rest: &[String]| match Target::from_words(rest.iter().cloned()) {
        None => Err(UsageError("missing panel name".into())),
        Some(t) if !panels.contains(&t.panel.as_str()) => Err(UsageError(format!("unknown panel {:?}", t.panel))),
        Some(t) => Ok(t),
    };
    let (command, rest) = match args.split_first() {
        None => return Ok(Invocation::Help),
        Some((command, rest)) => (command.as_str(), rest),
    };
    let no_args = |invocation| match rest {
        [] => Ok(invocation),
        _ => Err(UsageError(format!("{command} takes no arguments"))),
    };
    match command {
        "toggle" => target(rest).map(Invocation::Toggle),
        "preview" => target(rest).map(Invocation::Preview),
        "daemon" if rest.is_empty() => Ok(Invocation::Daemon(None)),
        "daemon" => target(rest).map(|t| Invocation::Daemon(Some(t))),
        "close" => no_args(Invocation::Close),
        "quit" => no_args(Invocation::Quit),
        "help" | "--help" | "-h" => no_args(Invocation::Help),
        _ => Err(UsageError(format!("unknown command {command:?}"))),
    }
}

/// The commands, the registered panels and a ready-to-paste `on-click` line
/// per panel, using the program's real path.
fn usage(exe: &Path, panels: &[&str]) -> String {
    let exe = exe.display();
    let mut text = format!(
        "usage: {exe} <command>\n\n\
         commands:\n\
         \x20 toggle <panel> [args…]    open <panel>, or close it if it is open with the same args\n\
         \x20 close                     close the open popover\n\
         \x20 quit                      stop the daemon\n\
         \x20 preview <panel> [args…]   open <panel> from this terminal; exits when it closes\n\
         \x20 daemon [<panel> [args…]]  run the daemon in the foreground\n\
         \x20 help                      show this text\n\n\
         panels and their Waybar on-click lines:\n"
    );
    for panel in panels {
        text.push_str(&format!("  {panel:<12} \"on-click\": \"{exe} toggle {panel}\"\n"));
    }
    text
}

/// How long `toggle` waits for a daemon from another executable to quit.
const REPLACE_TIMEOUT: Duration = Duration::from_secs(2);

/// Where a spawned daemon's stderr goes: `$XDG_STATE_HOME/waybar-gpui/<id>.log`,
/// `$XDG_STATE_HOME` defaulting to `~/.local/state`.
fn log_path(id: &str, state_home: Option<OsString>, home: Option<OsString>) -> Option<PathBuf> {
    let state_home = match state_home.filter(|dir| !dir.is_empty()) {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(home.filter(|dir| !dir.is_empty())?).join(".local/state"),
    };
    Some(state_home.join("waybar-gpui").join(format!("{id}.log")))
}

/// Creates the log's directory and truncates the log.
fn open_log(path: &Path) -> io::Result<File> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    File::create(path)
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

fn spawn_daemon(id: &str, target: &Target) -> io::Result<()> {
    let log = log_path(id, env::var_os("XDG_STATE_HOME"), env::var_os("HOME")).and_then(|path| {
        open_log(&path).map_err(|e| eprintln!("{id}: cannot open log {}: {e}", path.display())).ok()
    });
    let args: Vec<String> = ["daemon"].into_iter().chain(target.words()).map(str::to_owned).collect();
    spawn_detached(&env::current_exe()?, &args, log).map(drop)
}

/// Sends the toggle to the running daemon if it runs this very executable;
/// otherwise replaces it, so a rebuilt or reinstalled program shows its own
/// panels.
fn toggle(id: &str, target: Target) -> io::Result<()> {
    let mut daemon = match ipc::Connection::open(id) {
        Err(e) if ipc::no_daemon(&e) => return spawn_daemon(id, &target),
        result => result?,
    };
    let own = env::current_exe().and_then(|exe| ExeId::of(&exe)).ok();
    match Decision::new(daemon.identity(), own) {
        Decision::Send => daemon.send(&Command::Toggle(target)),
        Decision::Replace => {
            eprintln!("{id}: replacing the daemon of another executable");
            daemon.send(&Command::Quit)?;
            ipc::wait_gone(id, REPLACE_TIMEOUT)?;
            spawn_daemon(id, &target)
        }
    }
}

/// `close` and `quit` have nothing to do without a daemon.
fn send_if_running(id: &str, command: &Command) -> io::Result<()> {
    match ipc::send(id, command) {
        Err(e) if ipc::no_daemon(&e) => Ok(()),
        result => result,
    }
}

fn run(app: App, invocation: Invocation, usage_text: String) -> io::Result<()> {
    let id = app.id.clone();
    match invocation {
        Invocation::Toggle(target) => toggle(&id, target),
        Invocation::Close => send_if_running(&id, &Command::Close),
        Invocation::Quit => send_if_running(&id, &Command::Quit),
        Invocation::Daemon(initial) => runtime::run(app, Mode::Daemon(initial)),
        Invocation::Preview(target) => runtime::run(app, Mode::Preview(target)),
        Invocation::Help => {
            print!("{usage_text}");
            Ok(())
        }
    }
}

pub fn main(app: App) -> ExitCode {
    let exe = env::current_exe().unwrap_or_else(|_| env::args().next().unwrap_or_default().into());
    let names: Vec<&str> = app.panels.iter().map(|(name, _)| *name).collect();
    let args: Vec<String> = env::args().skip(1).collect();
    let usage_text = usage(&exe, &names);
    let invocation = match parse(&args, &names) {
        Ok(invocation) => invocation,
        Err(UsageError(message)) => {
            eprint!("{}: {message}\n\n{usage_text}", app.id);
            return ExitCode::from(2);
        }
    };
    let id = app.id.clone();
    match run(app, invocation, usage_text) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{id}: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PANELS: &[&str] = &["workspace", "clock"];

    fn parse_words(line: &str) -> Result<Invocation, UsageError> {
        let args: Vec<String> = line.split_whitespace().map(str::to_owned).collect();
        parse(&args, PANELS)
    }

    fn target(panel: &str, args: &[&str]) -> Target {
        Target { panel: panel.into(), args: args.iter().map(|a| a.to_string()).collect() }
    }

    #[test]
    fn parses_every_command() {
        assert_eq!(parse_words(""), Ok(Invocation::Help));
        assert_eq!(parse_words("help"), Ok(Invocation::Help));
        assert_eq!(parse_words("toggle clock Paris 2"), Ok(Invocation::Toggle(target("clock", &["Paris", "2"]))));
        assert_eq!(parse_words("preview workspace"), Ok(Invocation::Preview(target("workspace", &[]))));
        assert_eq!(parse_words("daemon"), Ok(Invocation::Daemon(None)));
        assert_eq!(parse_words("daemon clock Tokyo"), Ok(Invocation::Daemon(Some(target("clock", &["Tokyo"])))));
        assert_eq!(parse_words("close"), Ok(Invocation::Close));
        assert_eq!(parse_words("quit"), Ok(Invocation::Quit));
    }

    #[test]
    fn rejects_bad_command_lines() {
        assert_eq!(parse_words("open clock"), Err(UsageError("unknown command \"open\"".into())));
        assert_eq!(parse_words("toggle"), Err(UsageError("missing panel name".into())));
        assert_eq!(parse_words("preview"), Err(UsageError("missing panel name".into())));
        assert_eq!(parse_words("toggle calendar"), Err(UsageError("unknown panel \"calendar\"".into())));
        assert_eq!(parse_words("daemon calendar"), Err(UsageError("unknown panel \"calendar\"".into())));
        assert_eq!(parse_words("quit now"), Err(UsageError("quit takes no arguments".into())));
    }

    #[test]
    fn usage_lists_panels_with_on_click_lines() {
        let text = usage(Path::new("/home/me/.cargo/bin/my-panels"), PANELS);
        assert!(text.starts_with("usage: /home/me/.cargo/bin/my-panels <command>\n"));
        assert!(text.contains("  workspace    \"on-click\": \"/home/me/.cargo/bin/my-panels toggle workspace\"\n"));
        assert!(text.contains("  clock        \"on-click\": \"/home/me/.cargo/bin/my-panels toggle clock\"\n"));
    }

    #[test]
    fn log_path_follows_xdg_state_home() {
        let some = |s: &str| Some(OsString::from(s));
        assert_eq!(
            log_path("my-panels", some("/state"), some("/home/me")),
            Some(PathBuf::from("/state/waybar-gpui/my-panels.log"))
        );
        assert_eq!(
            log_path("my-panels", None, some("/home/me")),
            Some(PathBuf::from("/home/me/.local/state/waybar-gpui/my-panels.log"))
        );
        assert_eq!(
            log_path("my-panels", some(""), some("/home/me")),
            Some(PathBuf::from("/home/me/.local/state/waybar-gpui/my-panels.log"))
        );
        assert_eq!(log_path("my-panels", None, None), None);
    }

    /// The spawn path with a stand-in program: the log directory is created,
    /// stderr lands in the log, and each spawn truncates it.
    #[test]
    fn spawned_stderr_goes_to_a_truncated_log() {
        let state = env::temp_dir().join(format!("waybar-gpui-log-test-{}", process::id()));
        let path = log_path("test", Some(state.clone().into()), None).unwrap();
        let spawn_and_wait = |message: &str| {
            let args = ["-c".to_owned(), format!("echo {message} >&2; echo stdout")];
            let status = spawn_detached(Path::new("/bin/sh"), &args, Some(open_log(&path).unwrap())).unwrap().wait();
            assert!(status.unwrap().success());
        };
        spawn_and_wait("first run, longer line");
        assert_eq!(fs::read_to_string(&path).unwrap(), "first run, longer line\n");
        spawn_and_wait("second");
        assert_eq!(fs::read_to_string(&path).unwrap(), "second\n");
        fs::remove_dir_all(&state).unwrap();
    }
}
