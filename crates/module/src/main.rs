//! Waybar `custom/*` module: shows the active Hyprland workspace.
//!
//! Runs as a *continuous* script (no `interval` in the Waybar config): prints
//! one JSON object per line on startup and again whenever a relevant Hyprland
//! event arrives. Waybar re-renders on every line.

mod output;

use std::{
    io::{self, Write},
    process::ExitCode,
};

use output::Output;

/// Events after which the module re-queries Hyprland. Treat events as
/// "something changed" rather than tracking state from their payloads.
const RELEVANT_EVENTS: &[&str] =
    &["workspacev2", "focusedmonv2", "openwindow", "closewindow", "movewindowv2", "fullscreen"];

fn render() -> io::Result<Output> {
    let ws: hypr::Workspace = hypr::request("activeworkspace")?;
    let mut class = Vec::new();
    if ws.has_fullscreen {
        class.push("fullscreen".to_owned());
    }
    if ws.windows == 0 {
        class.push("empty".to_owned());
    }
    Ok(Output {
        text: ws.name,
        alt: Some(if ws.has_fullscreen { "fullscreen" } else { "normal" }.into()),
        tooltip: Some(format!("{} window(s) on {}", ws.windows, ws.monitor)),
        class,
        percentage: Some((ws.windows * 20).min(100) as u8),
    })
}

fn emit(out: &mut impl Write) -> io::Result<()> {
    let line = serde_json::to_string(&render()?)?;
    // Waybar reads stdout through a pipe, so flush explicitly after each line.
    writeln!(out, "{line}")?;
    out.flush()
}

fn run() -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    emit(&mut stdout)?;
    for event in hypr::Events::connect()? {
        if RELEVANT_EVENTS.contains(&event?.name.as_str()) {
            emit(&mut stdout)?;
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        // Hyprland closed the event socket; let Waybar's restart-interval
        // bring us back.
        Ok(()) => ExitCode::SUCCESS,
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("waybar-hypr-ws: {e}");
            ExitCode::FAILURE
        }
    }
}
