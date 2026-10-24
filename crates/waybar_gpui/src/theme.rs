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

/// Copies the base colours into gpui-component's theme; it derives the rest.
#[cfg(feature = "components")]
pub(crate) fn apply_to_components(theme: &Theme, cx: &mut App) {
    use std::rc::Rc;

    use gpui_component::theme::{Theme as ComponentTheme, ThemeConfig, ThemeConfigColors, ThemeMode};

    fn hex(c: Rgba) -> Option<gpui::SharedString> {
        let [r, g, b, a] = [c.r, c.g, c.b, c.a].map(|v| (v * 255.0).round() as u8);
        Some(format!("#{r:02x}{g:02x}{b:02x}{a:02x}").into())
    }

    // Only the base colours; gpui-component derives the rest (button, input,
    // hover…) from these through its own fallbacks.
    // (ThemeConfigColors has private fields, so no struct literal.)
    let mut colors = ThemeConfigColors::default();
    colors.background = hex(theme.base);
    colors.foreground = hex(theme.text);
    colors.border = hex(theme.overlay);
    colors.muted = hex(theme.surface);
    colors.muted_foreground = hex(theme.subtext);
    colors.primary = hex(theme.accent);
    // Text on accent-coloured buttons: the card colour, opaque.
    colors.primary_foreground = hex(Rgba { a: 1.0, ..theme.base });
    colors.accent = hex(theme.overlay);
    colors.accent_foreground = hex(theme.text);
    colors.popover = hex(theme.surface);
    colors.popover_foreground = hex(theme.text);
    colors.secondary = hex(theme.surface);
    colors.secondary_foreground = hex(theme.text);
    colors.input = hex(theme.overlay);
    colors.ring = hex(theme.accent);
    let config = ThemeConfig { name: "waybar-gpui".into(), mode: ThemeMode::Dark, colors, ..Default::default() };
    ComponentTheme::global_mut(cx).apply_config(&Rc::new(config));
}
