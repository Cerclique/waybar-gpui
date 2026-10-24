//! The long-running GPUI process. Keeping it alive means the GPU device and
//! fonts stay warm, so opening a popover after the first one is near-instant.

use std::{
    error::Error,
    io,
    time::{Duration, Instant},
};

use gpui::{
    AnyWindowHandle, App, AppContext as _, Bounds, DisplayId, Global, QuitMode, Size,
    WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions, layer_shell::*, point, px,
};
use uuid::Uuid;

use crate::{
    ipc::{self, Command},
    panel::{PanelKind, WorkspacePanel},
    placement,
};

/// Space between the popover and the bar, in logical pixels.
const GAP: f32 = 6.0;

/// Clicking the Waybar icon while the popover is open can take keyboard focus
/// from it, so it dismisses itself and *then* receives the toggle. A toggle
/// this soon after a dismissal is that same click: don't reopen.
const REOPEN_GUARD: Duration = Duration::from_millis(300);

#[derive(Default)]
struct PopupState {
    open: Option<(PanelKind, AnyWindowHandle)>,
    dismissed: Option<(PanelKind, Instant)>,
}

impl Global for PopupState {}

/// Called by a panel that closed itself (Esc or focus loss).
pub fn note_dismissed(kind: PanelKind, cx: &mut App) {
    let state = cx.global_mut::<PopupState>();
    state.open = None;
    state.dismissed = Some((kind, Instant::now()));
}

pub fn run(initial_panel: Option<String>) -> io::Result<()> {
    let (tx, rx) = async_channel::unbounded();
    ipc::listen(tx.clone())?;
    if let Some(panel) = initial_panel {
        let _ = tx.try_send(Command::Toggle(panel));
    }

    gpui_platform::application()
        .with_quit_mode(QuitMode::Explicit)
        .run(move |cx: &mut App| {
            cx.set_global(PopupState::default());
            cx.spawn(async move |cx| {
                while let Ok(command) = rx.recv().await {
                    cx.update(|cx| handle(command, cx));
                }
            })
            .detach();
        });
    Ok(())
}

fn handle(command: Command, cx: &mut App) {
    match command {
        Command::Toggle(name) => {
            let Some(kind) = PanelKind::from_name(&name) else {
                eprintln!("waybar-popup: unknown panel {name:?}");
                return;
            };
            if close(cx) == Some(kind) {
                return;
            }
            if let Some((dismissed, at)) = cx.global::<PopupState>().dismissed
                && dismissed == kind
                && at.elapsed() < REOPEN_GUARD
            {
                return;
            }
            if let Err(e) = open(kind, cx) {
                eprintln!("waybar-popup: cannot open {name}: {e}");
            }
        }
        Command::Close => {
            close(cx);
        }
        Command::Quit => {
            let _ = std::fs::remove_file(ipc::socket_path());
            cx.quit();
        }
    }
}

/// Closes the open popover, returning which panel it was.
fn close(cx: &mut App) -> Option<PanelKind> {
    let (kind, handle) = cx.global_mut::<PopupState>().open.take()?;
    handle.update(cx, |_, window, _| window.remove_window()).ok().map(|()| kind)
}

fn open(kind: PanelKind, cx: &mut App) -> Result<(), Box<dyn Error>> {
    let (width, height) = kind.size();
    let cursor: hypr::CursorPos = hypr::request("cursorpos")?;
    let monitors: Vec<hypr::Monitor> = hypr::request("monitors")?;
    let placement = placement::place(cursor, &monitors, (width, height), GAP)
        .ok_or("no monitor under the cursor")?;

    let (top, right, bottom, left) = placement.margin;
    let edge = if placement.bottom { Anchor::BOTTOM } else { Anchor::TOP };
    let options = WindowOptions {
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)), // ignored for layer surfaces
            size: Size::new(px(width), px(height)),
        })),
        app_id: Some("waybar-popup".into()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            // Match this in Hyprland layer rules (see examples/hypr/).
            namespace: "waybar-popup".into(),
            layer: Layer::Overlay,
            anchor: edge | Anchor::LEFT,
            // -1: position from the output edge, ignoring Waybar's exclusive
            // zone; the margins already include it.
            exclusive_zone: Some(px(-1.)),
            margin: Some((px(top), px(right), px(bottom), px(left))),
            keyboard_interactivity: KeyboardInteractivity::OnDemand,
            ..Default::default()
        }),
        focus: true,
        display_id: display_for_monitor(&placement.monitor, cx),
        ..Default::default()
    };

    let handle: AnyWindowHandle = match kind {
        PanelKind::Workspace => cx
            .open_window(options, |window, cx| cx.new(|cx| WorkspacePanel::new(window, cx)))?
            .into(),
    };
    cx.global_mut::<PopupState>().open = Some((kind, handle));
    Ok(())
}

/// GPUI's Wayland displays derive their UUID from the `wl_output` name, which
/// is the same connector name Hyprland reports (e.g. `DP-1`).
fn display_for_monitor(name: &str, cx: &App) -> Option<DisplayId> {
    let wanted = Uuid::new_v5(&Uuid::NAMESPACE_DNS, name.as_bytes());
    cx.displays()
        .into_iter()
        .find(|display| display.uuid().is_ok_and(|uuid| uuid == wanted))
        .map(|display| display.id())
}
