//! The daemon: one full-output transparent layer surface per open popover,
//! the panel drawn inside it through `anchored()` (measured and placed in the
//! same frame). A click that reaches the backdrop around the card dismisses.

use std::{cell::Cell, error::Error, io, rc::Rc};

use gpui::{
    AnyView, AnyWindowHandle, App, AppContext as _, Bounds, Context, DisplayId, FocusHandle,
    Global, IntoElement, QuitMode, Render, Size, Window, WindowBackgroundAppearance, WindowBounds,
    WindowKind, WindowOptions, Edges, anchored, div, layer_shell::*, point, prelude::*, px,
};
use uuid::Uuid;

use crate::{
    Build,
    ipc::{self, Command},
    placement::{self, Edge, Placement},
};

struct Registry {
    id: String,
    panels: Vec<(&'static str, Build)>,
    /// Set in `preview`: closing the popover quits, and a failed open is
    /// recorded here for the exit status.
    preview_failed: Option<Rc<Cell<bool>>>,
}

/// What the process runs as.
pub enum Mode {
    /// Listens on the program's socket until `quit`, optionally opening a
    /// panel (name + arguments) at once.
    Daemon(Option<Vec<String>>),
    /// Opens one panel without the socket and exits when it closes.
    Preview(Vec<String>),
}

impl Global for Registry {}

#[derive(Default)]
struct OpenPopover(Option<(Vec<String>, AnyWindowHandle)>);

impl Global for OpenPopover {}

/// Closes this program's popover once the current update finishes. Call it
/// from the panel's own event handlers.
pub fn close(cx: &mut App) {
    cx.defer(|cx| {
        close_now("panel", cx);
    });
}

pub fn run(app: crate::App, mode: Mode) -> io::Result<()> {
    let (tx, rx) = async_channel::unbounded();
    let (initial, preview_failed) = match mode {
        Mode::Daemon(initial) => {
            ipc::listen(&app.id, tx.clone())?;
            (initial, None)
        }
        Mode::Preview(target) => (Some(target), Some(Rc::new(Cell::new(false)))),
    };
    if let Some(target) = initial {
        let _ = tx.try_send(Command::Toggle(target));
    }
    let crate::App { id, theme, panels } = app;
    let failed = preview_failed.clone();

    let application = gpui_platform::application().with_quit_mode(QuitMode::Explicit);
    #[cfg(feature = "components")]
    let application = application.with_assets(gpui_kit_assets::Assets);
    application.run(move |cx: &mut App| {
        #[cfg(feature = "components")]
        {
            gpui_component::init(cx);
            crate::theme::apply_to_components(&theme, cx);
        }
        cx.set_global(theme);
        cx.set_global(Registry { id, panels, preview_failed });
        cx.set_global(OpenPopover::default());
        cx.spawn(async move |cx| {
            while let Ok(command) = rx.recv().await {
                cx.update(|cx| handle(command, cx));
            }
        })
        .detach();
    });
    match failed {
        Some(failed) if failed.get() => Err(io::Error::other("preview failed")),
        _ => Ok(()),
    }
}

fn handle(command: Command, cx: &mut App) {
    match command {
        Command::Toggle(target) => {
            // Same panel with the same arguments: close. Anything else: switch.
            if close_now("toggle", cx).as_ref() == Some(&target) {
                return;
            }
            if let Err(e) = open(target.clone(), cx) {
                let registry = cx.global::<Registry>();
                eprintln!("{}: cannot open {target:?}: {e}", registry.id);
                if let Some(failed) = &registry.preview_failed {
                    failed.set(true);
                    cx.quit();
                }
            }
        }
        Command::Close => {
            close_now("close command", cx);
        }
        Command::Quit => {
            close_now("quit", cx);
            let _ = std::fs::remove_file(ipc::socket_path(&cx.global::<Registry>().id));
            cx.quit();
        }
    }
}

fn close_now(reason: &str, cx: &mut App) -> Option<Vec<String>> {
    let (target, handle) = cx.global_mut::<OpenPopover>().0.take()?;
    let _ = handle.update(cx, |_, window, _| window.remove_window());
    let registry = cx.global::<Registry>();
    eprintln!("{}: closed {target:?} ({reason})", registry.id);
    if registry.preview_failed.is_some() {
        cx.quit();
    }
    Some(target)
}

fn open(target: Vec<String>, cx: &mut App) -> Result<(), Box<dyn Error>> {
    let registry = cx.global::<Registry>();
    let id = registry.id.clone();
    let preview = registry.preview_failed.is_some();
    let build = registry
        .panels
        .iter()
        .find(|(name, _)| *name == target[0])
        .map(|(_, build)| *build)
        .ok_or_else(|| format!("unknown panel {:?}", target[0]))?;

    // A preview also closes its own program's daemon popover: one popover at a time.
    ipc::close_others(&id, preview);

    let cursor: hypr::CursorPos = hypr::request("cursorpos")?;
    let monitors: Vec<hypr::Monitor> = hypr::request("monitors")?;
    let placement = placement::place(cursor, &monitors).ok_or("no monitor under the cursor")?;

    let (width, height) = placement.monitor_size;
    let options = WindowOptions {
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(width), px(height)),
        })),
        app_id: Some(format!("waybar-gpui-{id}")),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: format!("waybar-gpui:{id}"),
            layer: Layer::Overlay,
            anchor: Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
            exclusive_zone: Some(px(-1.)),
            keyboard_interactivity: KeyboardInteractivity::Exclusive,
            ..Default::default()
        }),
        focus: true,
        display_id: display_for_monitor(&placement.monitor, cx),
        ..Default::default()
    };

    let args = target[1..].to_vec();
    let handle = cx.open_window(options, |window, cx| {
        let content = build(&args, window, cx);
        // No gpui-component `Root`, even with `components`: on Linux it always
        // wraps the window in `window_border()` (layer surfaces are forced to
        // client-side decorations), which grows the surface by 20 px per side
        // and paints a shadow over the whole output. Menus and popovers work
        // without it; dialogs and notifications don't.
        cx.new(|cx| Overlay::new(content, placement.clone(), window, cx))
    })?;

    cx.global_mut::<OpenPopover>().0 = Some((target.clone(), handle.into()));
    eprintln!("{id}: opened {target:?} on {} ({:?} bar)", placement.monitor, placement.edge);
    Ok(())
}

struct Overlay {
    content: AnyView,
    placement: Placement,
    focus: FocusHandle,
}

impl Overlay {
    fn new(content: AnyView, placement: Placement, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle();
        // The panel may have focused one of its own elements in `new`; keep
        // that, so its key handlers (Esc included) run before ours.
        if window.focused(cx).is_none() {
            window.focus(&focus, cx);
        }
        Self { content, placement, focus }
    }
}

impl Render for Overlay {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let p = &self.placement;
        let [left, top, right, bottom] = p.margins;
        let corner = match p.edge {
            Edge::Top => gpui::Anchor::TopCenter,
            Edge::Bottom => gpui::Anchor::BottomCenter,
            Edge::Left => gpui::Anchor::LeftCenter,
            Edge::Right => gpui::Anchor::RightCenter,
        };
        let (w, h) = p.monitor_size;

        let card = div()
            .occlude()
            // Never larger than the usable area; the panel scrolls past that.
            .max_w(px(w - left - right))
            .max_h(px(h - top - bottom))
            // Clicks on other monitors arrive with coordinates outside this
            // surface (Hyprland forces them to the exclusive layer); nothing
            // here is "hovered" for those, so test geometrically.
            .on_mouse_down_out(move |event, _, cx| {
                let inside_surface =
                    event.position.x >= px(0.) && event.position.y >= px(0.) && event.position.x < px(w) && event.position.y < px(h);
                if !inside_surface {
                    cx.defer(|cx| {
                        close_now("click on another monitor", cx);
                    })
                }
            })
            .child(self.content.clone());

        div()
            .size_full()
            // Only clicks nothing else handles reach the backdrop: the card
            // and widget overlays (menus that overflow the card) occlude it.
            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| {
                cx.defer(|cx| {
                    close_now("click outside", cx);
                })
            })
            .on_mouse_down(gpui::MouseButton::Right, |_, _, cx| {
                cx.defer(|cx| {
                    close_now("click outside", cx);
                })
            })
            .track_focus(&self.focus)
            .on_key_down(|event, _, cx| {
                if event.keystroke.key == "escape" {
                    cx.defer(|cx| {
                        close_now("escape", cx);
                    });
                }
            })
            .child(
                anchored()
                    .position(point(px(p.anchor.0), px(p.anchor.1)))
                    .anchor(corner)
                    .snap_to_window_with_margin(Edges { top: px(top), right: px(right), bottom: px(bottom), left: px(left) })
                    .child(card),
            )
    }
}

/// GPUI's Wayland displays derive their UUID from the `wl_output` name.
fn display_for_monitor(name: &str, cx: &App) -> Option<DisplayId> {
    let wanted = Uuid::new_v5(&Uuid::NAMESPACE_DNS, name.as_bytes());
    cx.displays()
        .into_iter()
        .find(|display| display.uuid().is_ok_and(|uuid| uuid == wanted))
        .map(|display| display.id())
}
