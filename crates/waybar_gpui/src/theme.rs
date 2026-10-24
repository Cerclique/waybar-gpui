//! The SDK's single theme.

use gpui::{App, Div, Global, Rgba, div, prelude::*, rgb, rgba};

/// Colours shared by [`card`] and panels, stored by the runtime as a GPUI
/// global. Defaults: Catppuccin Mocha.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    /// Card background (translucent).
    pub base: Rgba,
    /// Raised rows, selected items.
    pub surface: Rgba,
    /// Borders, hover.
    pub overlay: Rgba,
    /// Main text.
    pub text: Rgba,
    /// Secondary text.
    pub subtext: Rgba,
    /// Highlights, focus rings.
    pub accent: Rgba,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            base: rgba(0x1e1e2ef2),
            surface: rgb(0x313244),
            overlay: rgb(0x45475a),
            text: rgb(0xcdd6f4),
            subtext: rgb(0xa6adc8),
            accent: rgb(0x89b4fa),
        }
    }
}

impl Global for Theme {}

impl Theme {
    /// The program's theme, as given to [`App::theme`](crate::App::theme).
    pub fn get(cx: &App) -> Self {
        *cx.global::<Self>()
    }
}

/// Standard card chrome: rounded, bordered, themed. Opt-in; a panel may draw
/// its own shape instead.
pub fn card(cx: &App) -> Div {
    let theme = Theme::get(cx);
    div()
        .flex()
        .flex_col()
        .gap_2()
        .p_3()
        .rounded_xl()
        .border_1()
        .border_color(theme.overlay)
        .bg(theme.base)
        .text_color(theme.text)
        .text_sm()
}
