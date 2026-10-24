//! Write a GPUI panel, get a Waybar popover on Hyprland.
//!
//! An author writes panels and a `main`:
//!
//! ```no_run
//! use waybar_gpui::{card, gpui::{self, prelude::*, Context, Window}, Panel};
//!
//! struct Hello;
//!
//! impl Panel for Hello {
//!     const NAME: &'static str = "hello";
//!     fn new(_args: &[String], _: &mut Window, _: &mut Context<Self>) -> Self {
//!         Hello
//!     }
//! }
//!
//! impl Render for Hello {
//!     fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
//!         card(cx).child("Hello")
//!     }
//! }
//!
//! fn main() -> std::process::ExitCode {
//!     waybar_gpui::App::new("my-panels").panel::<Hello>().run()
//! }
//! ```
//!
//! Waybar's `on-click` then runs `my-panels toggle hello [args…]`.

mod cli;
mod ipc;
mod placement;
mod runtime;
mod theme;

use std::process::ExitCode;

use gpui::{AnyView, AppContext as _, Context, Render, Window};

/// GPUI, re-exported so authors never pin it themselves. GPUI's derive macros
/// emit `gpui::…` paths, so write `use waybar_gpui::gpui;` in modules that use
/// them.
pub use gpui;

pub use runtime::close;
pub use theme::{Theme, card};

/// A popover's contents. Built fresh every time its popover opens and dropped
/// when it closes, so tasks stored in its fields stop on close.
///
/// The card fits the panel's content; give the root element a `max_h(..)` plus
/// `overflow_y_scroll()` if it can grow. Esc reaches the focused element first:
/// call `cx.stop_propagation()` to keep the popover open. Focus something in
/// `new` to receive keys.
pub trait Panel: Render + Sized {
    /// The name `on-click` passes to `toggle`.
    const NAME: &'static str;

    /// Builds the panel as its popover opens. `args` are the words after the
    /// panel name in `toggle <panel> [args…]`.
    fn new(args: &[String], window: &mut Window, cx: &mut Context<Self>) -> Self;
}

type Build = fn(&[String], &mut Window, &mut gpui::App) -> AnyView;

fn build<P: Panel>(args: &[String], window: &mut Window, cx: &mut gpui::App) -> AnyView {
    cx.new(|cx| P::new(args, window, cx)).into()
}

/// The author's program: an ID, a theme and its panels.
pub struct App {
    id: String,
    theme: Theme,
    panels: Vec<(&'static str, Build)>,
}

impl App {
    /// Starts a program. `id` names its socket
    /// (`$XDG_RUNTIME_DIR/waybar-gpui/<id>.sock`) and its Hyprland layer
    /// namespace (`waybar-gpui:<id>`), so keep it stable.
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into(), theme: Theme::default(), panels: Vec::new() }
    }

    /// Replaces the default theme (Catppuccin Mocha).
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Registers a panel under its [`Panel::NAME`].
    pub fn panel<P: Panel>(mut self) -> Self {
        self.panels.push((P::NAME, build::<P>));
        self
    }

    /// Parses the command line (`toggle`, `close`, `quit`, `daemon`) and runs it.
    pub fn run(self) -> ExitCode {
        cli::main(self)
    }
}
