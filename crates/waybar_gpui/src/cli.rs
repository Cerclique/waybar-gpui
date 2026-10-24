//! The author's command line, owned entirely by the SDK.

use std::{
    env, io,
    os::unix::process::CommandExt,
    path::Path,
    process::{self, ExitCode, Stdio},
};

use crate::{
    App,
    ipc::{self, Command},
    runtime::{self, Mode},
};

/// A parsed command line. Panel targets are the panel name followed by its
/// arguments.
#[derive(Debug, PartialEq, Eq)]
enum Invocation {
    Toggle(Vec<String>),
    Close,
    Quit,
    Daemon(Option<Vec<String>>),
    Preview(Vec<String>),
    Help,
}

/// Why a command line was rejected; the usage text follows the message.
#[derive(Debug, PartialEq, Eq)]
struct UsageError(String);

fn parse(args: &[String], panels: &[&str]) -> Result<Invocation, UsageError> {
    let target = |rest: &[String]| match rest.first() {
        None => Err(UsageError("missing panel name".into())),
        Some(name) if !panels.contains(&name.as_str()) => Err(UsageError(format!("unknown panel {name:?}"))),
        Some(_) => Ok(rest.to_vec()),
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

fn spawn_daemon(target: &[String]) -> io::Result<()> {
    process::Command::new(env::current_exe()?)
        .arg("daemon")
        .args(target)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        // Own process group, so it outlives Waybar reloads killing on-click children.
        .process_group(0)
        .spawn()
        .map(drop)
}

/// No daemon listening: nothing is open and nothing runs.
fn no_daemon(e: &io::Error) -> bool {
    matches!(e.kind(), io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused)
}

/// `close` and `quit` have nothing to do without a daemon.
fn send_if_running(id: &str, command: &Command) -> io::Result<()> {
    match ipc::send(id, command) {
        Err(e) if no_daemon(&e) => Ok(()),
        result => result,
    }
}

fn run(app: App, invocation: Invocation, usage_text: String) -> io::Result<()> {
    let id = app.id.clone();
    match invocation {
        Invocation::Toggle(target) => match ipc::send(&id, &Command::Toggle(target.clone())) {
            Err(e) if no_daemon(&e) => spawn_daemon(&target),
            result => result,
        },
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

    fn target(words: &str) -> Vec<String> {
        words.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn parses_every_command() {
        assert_eq!(parse_words(""), Ok(Invocation::Help));
        assert_eq!(parse_words("help"), Ok(Invocation::Help));
        assert_eq!(parse_words("toggle clock Paris 2"), Ok(Invocation::Toggle(target("clock Paris 2"))));
        assert_eq!(parse_words("preview workspace"), Ok(Invocation::Preview(target("workspace"))));
        assert_eq!(parse_words("daemon"), Ok(Invocation::Daemon(None)));
        assert_eq!(parse_words("daemon clock Tokyo"), Ok(Invocation::Daemon(Some(target("clock Tokyo")))));
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
}
