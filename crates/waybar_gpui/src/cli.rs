//! The author's command line, owned entirely by the SDK.

use std::{
    env, io,
    os::unix::process::CommandExt,
    process::{self, ExitCode, Stdio},
};

use crate::{
    App,
    ipc::{self, Command},
    runtime,
};

fn usage(app: &App) -> String {
    let names: Vec<&str> = app.panels.iter().map(|(name, _)| *name).collect();
    format!(
        "usage: <program> toggle <panel> [args…]   open <panel>, or close it if open\n\
         \x20      <program> close                    close the open popover\n\
         \x20      <program> quit                     stop the daemon\n\
         \x20      <program> daemon [<panel> [args…]] run the daemon in the foreground\n\n\
         panels: {}",
        names.join(", ")
    )
}

fn spawn_daemon(panel: &[String]) -> io::Result<()> {
    process::Command::new(env::current_exe()?)
        .arg("daemon")
        .args(panel)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        // Own process group, so it outlives Waybar reloads killing on-click children.
        .process_group(0)
        .spawn()
        .map(drop)
}

fn run(app: App, args: Vec<String>) -> io::Result<()> {
    let id = app.id.clone();
    match args.first().map(String::as_str) {
        Some("toggle") if args.len() >= 2 => {
            let target = args[1..].to_vec();
            match ipc::send(&id, &Command::Toggle(target.clone())) {
                Err(e) if matches!(e.kind(), io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused) => {
                    spawn_daemon(&target)
                }
                result => result,
            }
        }
        Some("close") => ipc::send(&id, &Command::Close),
        Some("quit") => ipc::send(&id, &Command::Quit),
        Some("daemon") => {
            let initial = (args.len() >= 2).then(|| args[1..].to_vec());
            runtime::run(app, initial)
        }
        _ => Err(io::Error::new(io::ErrorKind::InvalidInput, usage(&app))),
    }
}

pub fn main(app: App) -> ExitCode {
    let id = app.id.clone();
    match run(app, env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{id}: {e}");
            ExitCode::FAILURE
        }
    }
}
