//! The long-running GPUI process. Keeping it alive means the GPU device and
//! fonts stay warm, so opening a popover after the first one is near-instant.
//!
//! An open popover is a single transparent layer surface covering the whole
//! output (bar included), with exclusive keyboard focus. The panel is drawn as
//! a card inside it at the computed position. Clicks outside the card land on
//! the same surface and close it, and Esc always reaches it. This doesn't
//! depend on focus (Hyprland's `follow_mouse` moves it) or on how the
//! compositor routes pointer input while an exclusive-keyboard layer is open.

use std::{error::Error, io};

use gpui::{
    AnyView, AnyWindowHandle, App, AppContext as _, Bounds, Context, DisplayId, FocusHandle,
    Global, IntoElement, QuitMode, Render, Size, Window, WindowBackgroundAppearance, WindowBounds,
    WindowKind, WindowOptions, div, layer_shell::*, point, prelude::*, px,
};
use uuid::Uuid;

use crate::{
    ipc::{self, Command},
    panel::{PanelKind, WorkspacePanel},
    placement::{self, Placement},
};

/// Space between the popover and the bar, in logical pixels.
const GAP: f32 = 6.0;

#[derive(Default)]
struct PopupState {
    open: Option<(PanelKind, AnyWindowHandle)>,
}

impl Global for PopupState {}

/// Closes the popover once the current update finishes. Use this from inside
/// the popover's own event handlers, where its window can't be updated.
fn close_deferred(reason: &'static str, cx: &mut App) {
    cx.defer(move |cx| {
        close(reason, cx);
    });
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
            if close("toggle", cx) == Some(kind) {
                return;
            }
            if let Err(e) = open(kind, cx) {
                eprintln!("waybar-popup: cannot open {name}: {e}");
            }
        }
        Command::Close => {
            close("close command", cx);
        }
        Command::Quit => {
            close("quit", cx);
            let _ = std::fs::remove_file(ipc::socket_path());
            cx.quit();
        }
    }
}

/// Closes the open popover, returning which panel it was.
fn close(reason: &str, cx: &mut App) -> Option<PanelKind> {
    let (kind, handle) = cx.global_mut::<PopupState>().open.take()?;
    let _ = handle.update(cx, |_, window, _| window.remove_window());
    eprintln!("waybar-popup: closed {kind:?} ({reason})");
    Some(kind)
}

fn open(kind: PanelKind, cx: &mut App) -> Result<(), Box<dyn Error>> {
    let card_size = kind.size();
    let cursor: hypr::CursorPos = hypr::request("cursorpos")?;
    let monitors: Vec<hypr::Monitor> = hypr::request("monitors")?;
    let placement = placement::place(cursor, &monitors, card_size, GAP)
        .ok_or("no monitor under the cursor")?;

    let (width, height) = placement.monitor_size;
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
            anchor: Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
            // -1: cover the whole output, bar included, so clicking the
            // module again also counts as "outside".
            exclusive_zone: Some(px(-1.)),
            keyboard_interactivity: KeyboardInteractivity::Exclusive,
            ..Default::default()
        }),
        focus: true,
        display_id: display_for_monitor(&placement.monitor, cx),
        ..Default::default()
    };

    let handle = cx.open_window(options, |window, cx| {
        let content: AnyView = match kind {
            PanelKind::Workspace => cx.new(WorkspacePanel::new).into(),
        };
        cx.new(|cx| Overlay::new(content, &placement, card_size, window, cx))
    })?;

    cx.global_mut::<PopupState>().open = Some((kind, handle.into()));
    eprintln!("waybar-popup: opened {kind:?} on {} at {:?}", placement.monitor, placement.origin);
    Ok(())
}

/// Full-output root view: transparent everywhere except the panel card.
struct Overlay {
    content: AnyView,
    origin: (f32, f32),
    size: (f32, f32),
    focus: FocusHandle,
}

impl Overlay {
    fn new(
        content: AnyView,
        placement: &Placement,
        size: (f32, f32),
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        Self { content, origin: placement.origin, size, focus }
    }
}

impl Render for Overlay {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let card = div()
            .absolute()
            .left(px(self.origin.0))
            .top(px(self.origin.1))
            .w(px(self.size.0))
            .h(px(self.size.1))
            // Clicks on the card stay in the card.
            .occlude()
            .child(self.content.clone());

        div()
            .size_full()
            .track_focus(&self.focus)
            .on_key_down(|event, _, cx| {
                if event.keystroke.key == "escape" {
                    close_deferred("escape", cx);
                }
            })
            .on_any_mouse_down(|_, _, cx| close_deferred("click outside", cx))
            .child(card)
    }
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
