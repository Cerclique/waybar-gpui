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
//! Waybar's `on-click` then runs `my-panels toggle hello [args…]`; run the
//! program with no arguments for the full command line and an `on-click` line
//! per panel.
//!
//! - A [`Panel`] is built fresh each time its popover opens and dropped when it
//!   closes. Live data and state that survives a close are plain GPUI: tasks
//!   stored in the panel, and [`gpui::Global`]s.
//! - [`card`] and [`Theme`] are the optional standard look; [`close`] closes
//!   the popover from the panel's own handlers.
//! - GPUI is re-exported as [`gpui`]; don't depend on it yourself. Its derive
//!   macros expand to `gpui::…` paths, so modules using them need
//!   `use waybar_gpui::gpui;`.
//! - With the `components` feature, gpui-component's widgets are available as
//!   `waybar_gpui::components`, themed from [`Theme`].
//!
//! The guide covers each of these with examples:
//! <https://github.com/Cerclique/waybar-gpui/blob/v0.1.0/docs/guide/index.md>.

#![deny(missing_docs)]

mod cli;
mod client;
mod files;
mod ipc;
mod panels;
mod placement;
mod popovers;
mod runtime;
mod theme;

use std::process::ExitCode;

use gpui::{Context, Render, Window};

use panels::Panels;

/// GPUI, re-exported so authors never pin it themselves. GPUI's derive macros
/// emit `gpui::…` paths, so write `use waybar_gpui::gpui;` in modules that use
/// them.
pub use gpui;

/// gpui-component's widgets, with the `components` feature. The runtime
/// initialises the library and copies [`Theme`] into its theme at startup.
/// Menus and in-window popovers work; dialogs and notifications don't, because
/// they need gpui-component's `Root`, which breaks layer surfaces.
#[cfg(feature = "components")]
pub use gpui_component as components;

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

/// The author's program: an ID, a theme and its panels.
pub struct App {
    id: String,
    theme: Theme,
    panels: Panels,
}

impl App {
    /// Starts a program. `id` names its socket
    /// (`$XDG_RUNTIME_DIR/waybar-gpui/<id>.sock`), its Hyprland layer
    /// namespace (`waybar-gpui:<id>`) and its log, so keep it stable.
    ///
    /// # Panics
    ///
    /// If `id` isn't lowercase ASCII letters, digits and dashes, starting with
    /// a letter or digit.
    pub fn new(id: impl Into<String>) -> Self {
        let id = id.into();
        assert!(valid_id(&id), "waybar_gpui: invalid program ID {id:?}: use a-z, 0-9 and '-', not starting with '-'");
        Self { id, theme: Theme::default(), panels: Panels::default() }
    }

    /// Replaces the default theme (Catppuccin Mocha).
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Registers a panel under its [`Panel::NAME`].
    ///
    /// # Panics
    ///
    /// If a panel with the same name is already registered.
    pub fn panel<P: Panel>(mut self) -> Self {
        self.panels.add::<P>();
        self
    }

    /// Parses the command line and runs it: `toggle <panel> [args…]`, `close`,
    /// `quit`, `preview <panel> [args…]`, `daemon [<panel> [args…]]` or `help`.
    /// Without arguments it prints the usage text, including an `on-click`
    /// line per panel.
    pub fn run(self) -> ExitCode {
        cli::main(self)
    }
}

/// `^[a-z0-9][a-z0-9-]*$`: safe in a socket file name and a layer namespace.
fn valid_id(id: &str) -> bool {
    let mut chars = id.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_program_ids() {
        for id in ["my-panels", "a", "0clock", "panels-2"] {
            assert!(valid_id(id), "{id}");
        }
        for id in ["", "-panels", "My-Panels", "my_panels", "my panels", "my/panels", "café"] {
            assert!(!valid_id(id), "{id}");
        }
    }

    #[test]
    #[should_panic(expected = "invalid program ID")]
    fn new_rejects_invalid_id() {
        App::new("My Panels");
    }
}
