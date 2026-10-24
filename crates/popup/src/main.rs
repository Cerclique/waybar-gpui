//! GPUI popover for Waybar modules.
//!
//! Waybar's `on-click` runs `waybar-popup toggle <panel>`. If a daemon is
//! running, the command is forwarded to it over a Unix socket. Otherwise a
//! daemon is started in the background, so the first click is slower (GPU and
//! font init) and the next ones are instant.

mod daemon;
mod ipc;
mod panel;
mod placement;

use std::{
    env, io,
    os::unix::process::CommandExt,
    process::{self, ExitCode, Stdio},
};

use ipc::Command;

const USAGE: &str = "\
usage: waybar-popup toggle <panel>   open <panel>, or close it if open
       waybar-popup close            close the open panel
       waybar-popup quit             stop the daemon
       waybar-popup daemon [<panel>] run the daemon in the foreground

panels: workspace";

fn spawn_daemon(panel: &str) -> io::Result<()> {
    process::Command::new(env::current_exe()?)
        .args(["daemon", panel])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        // Own process group, so it outlives Waybar reloads killing on-click children.
        .process_group(0)
        .spawn()
        .map(drop)
}

fn run(args: &[String]) -> io::Result<()> {
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        ["toggle", panel] => match ipc::send(&Command::Toggle((*panel).to_owned())) {
            Err(e) if matches!(e.kind(), io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused) => {
                spawn_daemon(panel)
            }
            result => result,
        },
        ["close"] => ipc::send(&Command::Close),
        ["quit"] => ipc::send(&Command::Quit),
        ["daemon"] => daemon::run(None),
        ["daemon", panel] => daemon::run(Some((*panel).to_owned())),
        _ => Err(io::Error::new(io::ErrorKind::InvalidInput, USAGE)),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("waybar-popup: {e}");
            ExitCode::FAILURE
        }
    }
}
