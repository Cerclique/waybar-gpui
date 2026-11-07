//! The author's command line, owned entirely by the SDK.

use std::{borrow::Cow, env, ffi::OsString, fmt::Write as _, io, path::Path, process::ExitCode};

use crate::{
    Program,
    client::{self, CurrentExe},
    files::ProgramFiles,
    ipc::Target,
    panels::Panels,
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
#[derive(Debug, Clone, PartialEq, Eq)]
struct UsageError(String);

/// The command line as UTF-8 strings; any other argument is a usage error.
fn utf8_args(args: impl Iterator<Item = OsString>) -> Result<Vec<String>, UsageError> {
    args.map(OsString::into_string).collect::<Result<_, _>>().map_err(|_| UsageError("arguments must be UTF-8".into()))
}

fn parse(args: &[String], panels: &Panels) -> Result<Invocation, UsageError> {
    let target = |rest: &[String]| match Target::from_words(rest.iter().cloned()) {
        None => Err(UsageError("missing panel name".into())),
        Some(t) if panels.get(&t.panel).is_none() => Err(UsageError(format!("unknown panel {:?}", t.panel))),
        // The socket's line format can't carry them (see `ipc`).
        Some(t) if t.args.iter().any(|a| a.contains(['\t', '\n'])) => {
            Err(UsageError("panel arguments can't contain tabs or newlines".into()))
        }
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

/// Quotes `word` for `sh`, which runs Waybar's `on-click`, unless it only
/// holds characters the shell takes literally.
fn shell_quote(word: &str) -> Cow<'_, str> {
    let literal = |c: char| c.is_ascii_alphanumeric() || "/._-+,:@%=".contains(c);
    if !word.is_empty() && word.chars().all(literal) {
        Cow::Borrowed(word)
    } else {
        Cow::Owned(format!("'{}'", word.replace('\'', r"'\''")))
    }
}

/// A Waybar `on-click` entry toggling `panel`: valid JSON, and one shell word
/// for the program's path, whatever characters the path holds.
fn on_click_line(exe: &Path, panel: &str) -> String {
    let command = format!("{} toggle {panel}", shell_quote(&exe.to_string_lossy()));
    format!("\"on-click\": {}", serde_json::Value::String(command))
}

/// The commands, the registered panels and a ready-to-paste `on-click` line
/// per panel, using the program's real path.
fn usage(exe: &Path, panels: &Panels) -> String {
    let on_click = |panel: &str| on_click_line(exe, panel);
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
    for panel in panels.names() {
        let _ = writeln!(text, "  {panel:<12} {}", on_click(panel));
    }
    text
}

fn run(app: Program, invocation: Invocation, usage_text: &str) -> io::Result<()> {
    let files = ProgramFiles::from_env(&app.id);
    match invocation {
        Invocation::Toggle(target) => client::toggle(&files, target, &CurrentExe),
        Invocation::Close => client::close(&files),
        Invocation::Quit => client::quit(&files),
        Invocation::Daemon(initial) => runtime::run(app, files, Mode::Daemon(initial)),
        Invocation::Preview(target) => runtime::run(app, files, Mode::Preview(target)),
        Invocation::Help => {
            print!("{usage_text}");
            Ok(())
        }
    }
}

pub fn main(app: Program) -> ExitCode {
    let exe = env::current_exe().unwrap_or_else(|_| env::args_os().next().unwrap_or_default().into());
    let usage_text = usage(&exe, &app.panels);
    let invocation = match utf8_args(env::args_os().skip(1)).and_then(|args| parse(&args, &app.panels)) {
        Ok(invocation) => invocation,
        Err(UsageError(message)) => {
            eprint!("{}: {message}\n\n{usage_text}", app.id);
            return ExitCode::from(2);
        }
    };
    let id = app.id.clone();
    match run(app, invocation, &usage_text) {
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

    crate::panels::test_panel!(Workspace, "workspace");
    crate::panels::test_panel!(Clock, "clock");

    fn panels() -> Panels {
        let mut panels = Panels::default();
        panels.add::<Workspace>();
        panels.add::<Clock>();
        panels
    }

    fn parse_words(line: &str) -> Result<Invocation, UsageError> {
        let args: Vec<String> = line.split_whitespace().map(str::to_owned).collect();
        parse(&args, &panels())
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
        let control = UsageError("panel arguments can't contain tabs or newlines".into());
        for command in ["toggle", "preview", "daemon"] {
            for arg in ["New\tYork", "New\nYork"] {
                let args = [command, "clock", arg].map(str::to_owned);
                assert_eq!(parse(&args, &panels()), Err(control.clone()), "{command} {arg:?}");
            }
        }
    }

    #[test]
    fn rejects_non_utf8_arguments() {
        use std::os::unix::ffi::OsStringExt;
        let args = [OsString::from("toggle"), OsString::from("clock"), OsString::from_vec(vec![0xff])];
        assert_eq!(utf8_args(args.into_iter()), Err(UsageError("arguments must be UTF-8".into())));
        let args = [OsString::from("toggle"), OsString::from("clock")];
        assert_eq!(utf8_args(args.into_iter()), Ok(vec!["toggle".to_owned(), "clock".to_owned()]));
    }

    #[test]
    fn usage_lists_panels_with_on_click_lines() {
        let text = usage(Path::new("/home/me/.cargo/bin/my-panels"), &panels());
        assert!(text.starts_with("usage: /home/me/.cargo/bin/my-panels <command>\n"));
        assert!(text.contains("  workspace    \"on-click\": \"/home/me/.cargo/bin/my-panels toggle workspace\"\n"));
        assert!(text.contains("  clock        \"on-click\": \"/home/me/.cargo/bin/my-panels toggle clock\"\n"));

        // A space, quotes or a backslash: still valid JSON, still one shell word.
        let exe = r#"/home/me/my "best" panels\bin/it's"#;
        let text = usage(Path::new(exe), &panels());
        let line = text.lines().find(|l| l.trim_start().starts_with("clock")).unwrap();
        let entry = line.trim_start().trim_start_matches("clock").trim_start();
        let json: serde_json::Value = serde_json::from_str(&format!("{{{entry}}}")).unwrap();
        assert_eq!(json["on-click"], r#"'/home/me/my "best" panels\bin/it'\''s' toggle clock"#);
    }
}
