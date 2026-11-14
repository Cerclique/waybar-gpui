//! The guide's Rust snippets: one module per page of `docs/guide/`, compiled
//! and linted with the workspace but never run, so they keep up with the SDK
//! and the pinned GPUI.
//!
//! A page shows the lines between `// ANCHOR: <name>` and
//! `// ANCHOR_END: <name>` with `{{#include snippets/src/<module>.rs:<name>}}`
//! inside a code block. The rest of a module is the context the snippet needs
//! to compile (imports, stand-in panels), which the page leaves out. A snippet
//! from inside a function is indented here; the book's preprocessor
//! (`scripts/book-preprocessor.py`) strips that on the site.

// The snippets are what an author would write; nothing here calls them.
#![allow(dead_code)]

/// Empty panels for the snippets that name a panel the page doesn't define:
/// `stand_in!(Clock = "clock");`.
macro_rules! stand_in {
    ($($panel:ident = $name:literal),* $(,)?) => {$(
        pub struct $panel;

        impl waybar_gpui::Panel for $panel {
            const NAME: &'static str = $name;
            fn new(
                _: &[String],
                _: &mut waybar_gpui::gpui::Window,
                _: &mut waybar_gpui::gpui::Context<Self>,
            ) -> Self {
                $panel
            }
        }

        impl waybar_gpui::gpui::Render for $panel {
            fn render(
                &mut self,
                _: &mut waybar_gpui::gpui::Window,
                cx: &mut waybar_gpui::gpui::Context<Self>,
            ) -> impl waybar_gpui::gpui::IntoElement {
                waybar_gpui::card(cx)
            }
        }
    )*};
}

mod closing_and_esc;
#[cfg(feature = "components")]
mod components;
mod live_data_and_state;
mod panel_arguments;
mod theming;
mod writing_a_panel;
