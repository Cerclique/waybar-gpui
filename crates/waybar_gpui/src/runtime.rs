//! The daemon: one full-output transparent layer surface per open popover,
//! the panel drawn inside it through `anchored()` (measured and placed in the
//! same frame). A click that reaches the backdrop around the card dismisses.

use std::{cell::Cell, error::Error, io, os::fd::AsFd, rc::Rc};

use gpui::{
    AnyView, AnyWindowHandle, App, AppContext as _, Bounds, Context, DisplayId, Edges, FocusHandle, Global,
    IntoElement, QuitMode, Render, Size, Window, WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions,
    anchored, div, layer_shell::*, point, prelude::*, px,
};
use uuid::Uuid;

use crate::{
    files::{Ownership, ProgramFiles},
    ipc::{Command, Target},
    panels::Panels,
    placement::{self, Placement},
    popovers::{Effect, Popovers},
};

struct Registry {
    panels: Panels,
    session: Session,
}

/// What the running process holds for its [`Mode`].
enum Session {
    /// The daemon's socket, given up on `quit` (`None` once given up).
    Daemon { ownership: Option<Ownership> },
    /// Set when the preview's panel can't open, for the exit status.
    Preview { failed: Rc<Cell<bool>> },
}

impl Session {
    fn is_preview(&self) -> bool {
        matches!(self, Self::Preview { .. })
    }
}

/// What the process runs as.
pub enum Mode {
    /// Listens on the program's socket until `quit`, optionally opening a
    /// panel at once.
    Daemon(Option<Target>),
    /// Opens one panel without the socket and exits when it closes.
    Preview(Target),
}

impl Global for Registry {}

impl Global for Popovers<AnyWindowHandle> {}

/// Closes this program's popover once the current update finishes. Call it
/// from the panel's own event handlers.
pub fn close(cx: &mut App) {
    defer_close("panel", cx);
}

/// Closes the popover after the current event, so nothing removes the window
/// during its own dispatch.
fn defer_close(reason: &'static str, cx: &mut App) {
    cx.defer(move |cx| {
        let effects = cx.global_mut::<Popovers<AnyWindowHandle>>().close(reason);
        apply(effects, cx);
    });
}

impl Global for ProgramFiles {}

pub fn run(app: crate::App, files: ProgramFiles, mode: Mode) -> io::Result<()> {
    let (tx, rx) = async_channel::unbounded();
    let failed = Rc::new(Cell::new(false));
    let (initial, session) = match mode {
        Mode::Daemon(initial) => {
            (initial, Session::Daemon { ownership: Some(files.claim(io::stderr().as_fd(), tx.clone())?) })
        }
        Mode::Preview(target) => (Some(target), Session::Preview { failed: failed.clone() }),
    };
    if let Some(target) = initial {
        let _ = tx.try_send(Command::Toggle(target));
    }
    // The program ID is `files`' from here on.
    let crate::App { theme, panels, .. } = app;

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
        cx.set_global(Popovers::<AnyWindowHandle>::new(session.is_preview()));
        cx.set_global(Registry { panels, session });
        cx.set_global(files);
        cx.spawn(async move |cx| {
            while let Ok(command) = rx.recv().await {
                cx.update(|cx| {
                    let effects = cx.global_mut::<Popovers<AnyWindowHandle>>().on(command);
                    apply(effects, cx);
                });
            }
        })
        .detach();
    });
    if failed.get() {
        return Err(io::Error::other("preview failed"));
    }
    Ok(())
}

/// Carries out the popover rules' effects, reporting each open's outcome back.
/// No decisions here: those are in `Popovers`.
fn apply(effects: Vec<Effect<AnyWindowHandle>>, cx: &mut App) {
    for effect in effects {
        match effect {
            Effect::Close { target, window, reason } => {
                let _ = window.update(cx, |_, window, _| window.remove_window());
                eprintln!("{}: closed {target} ({reason})", cx.global::<ProgramFiles>().id());
            }
            Effect::Open(target) => match open(&target, cx) {
                Ok(window) => cx.global_mut::<Popovers<AnyWindowHandle>>().opened(target, window),
                Err(e) => {
                    eprintln!("{}: cannot open {target}: {e}", cx.global::<ProgramFiles>().id());
                    let effects = cx.global_mut::<Popovers<AnyWindowHandle>>().open_failed();
                    apply(effects, cx);
                }
            },
            Effect::RemoveSocket => {
                // Before the event loop stops: clients find no daemon at once.
                if let Session::Daemon { ownership } = &mut cx.global_mut::<Registry>().session
                    && let Some(ownership) = ownership.take()
                {
                    ownership.give_up();
                }
            }
            Effect::Quit { failed } => {
                if failed && let Session::Preview { failed } = &cx.global::<Registry>().session {
                    failed.set(true);
                }
                cx.quit();
            }
        }
    }
}

fn open(target: &Target, cx: &mut App) -> Result<AnyWindowHandle, Box<dyn Error>> {
    let registry = cx.global::<Registry>();
    let build = registry.panels.get(&target.panel).ok_or_else(|| format!("unknown panel {:?}", target.panel))?;

    // A preview also closes its own program's daemon popover: one popover at a time.
    let files = cx.global::<ProgramFiles>();
    files.close_others(registry.session.is_preview());
    let id = files.id().to_owned();

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

    let handle = cx.open_window(options, |window, cx| {
        let content = build(&target.args, window, cx);
        // No gpui-component `Root`, even with `components`: on Linux it always
        // wraps the window in `window_border()` (layer surfaces are forced to
        // client-side decorations), which grows the surface by 20 px per side
        // and paints a shadow over the whole output. Menus and popovers work
        // without it; dialogs and notifications don't.
        cx.new(|cx| Popover::new(content, placement.clone(), window, cx))
    })?;

    eprintln!("{id}: opened {target} on {} ({:?} bar)", placement.monitor, placement.edge);
    Ok(handle.into())
}

/// The popover's root view: the full-monitor backdrop, with the panel's card
/// placed inside it.
struct Popover {
    content: AnyView,
    placement: Placement,
    focus: FocusHandle,
}

impl Popover {
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

impl Render for Popover {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let p = &self.placement;
        let [left, top, right, bottom] = p.margins;
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
                let inside_surface = event.position.x >= px(0.)
                    && event.position.y >= px(0.)
                    && event.position.x < px(w)
                    && event.position.y < px(h);
                if !inside_surface {
                    defer_close("click on another monitor", cx);
                }
            })
            .child(self.content.clone());

        div()
            .size_full()
            // Only clicks nothing else handles reach the backdrop: the card
            // and widget overlays (menus that overflow the card) occlude it.
            .on_any_mouse_down(|_, _, cx| defer_close("click outside", cx))
            .track_focus(&self.focus)
            .on_key_down(|event, _, cx| {
                if event.keystroke.key == "escape" {
                    defer_close("escape", cx);
                }
            })
            .child(
                anchored()
                    .position(point(px(p.anchor.0), px(p.anchor.1)))
                    .anchor(p.corner)
                    .snap_to_window_with_margin(Edges {
                        top: px(top),
                        right: px(right),
                        bottom: px(bottom),
                        left: px(left),
                    })
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
