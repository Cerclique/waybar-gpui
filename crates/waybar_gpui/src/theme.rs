//! The SDK's single theme.

use gpui::{App, Div, Global, Rgba, div, prelude::*, rgb, rgba};

/// Colours shared by [`card`] and panels, stored by the runtime as a GPUI
/// global. Defaults: Catppuccin Mocha.
///
/// Colours may be added without a breaking release, so a `Theme` is built by
/// changing the fields of [`Theme::default`]:
///
/// ```
/// use waybar_gpui::{gpui::{rgb, rgba}, Theme};
///
/// let mut theme = Theme::default();
/// theme.base = rgba(0x2e3440f0);
/// theme.accent = rgb(0x88c0d0);
/// ```
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
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
    /// The program's theme, as given to [`Program::theme`](crate::Program::theme).
    ///
    /// # Panics
    ///
    /// Outside a program started by [`Program::run`](crate::Program::run).
    pub fn get(cx: &App) -> Self {
        *cx.global::<Self>()
    }
}

/// Standard card chrome: rounded, bordered, themed. Opt-in; a panel may draw
/// its own shape instead.
///
/// # Panics
///
/// Outside a program started by [`Program::run`](crate::Program::run).
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
    // (ThemeConfig is #[non_exhaustive] too.)
    let mut config = ThemeConfig::default();
    config.name = "waybar-gpui".into();
    config.mode = ThemeMode::Dark;
    config.colors = colors;
    ComponentTheme::global_mut(cx).apply_config(&Rc::new(config));
}

#[cfg(test)]
mod tests {
    use gpui::TestAppContext;

    use super::*;

    /// A theme whose every colour differs from the default's.
    fn nord() -> Theme {
        Theme {
            base: rgba(0x2e3440f0),
            surface: rgb(0x3b4252),
            overlay: rgb(0x434c5e),
            text: rgb(0xeceff4),
            subtext: rgb(0xd8dee9),
            accent: rgb(0x88c0d0),
        }
    }

    fn colours(t: Theme) -> [Rgba; 6] {
        [t.base, t.surface, t.overlay, t.text, t.subtext, t.accent]
    }

    #[gpui::test]
    fn get_returns_the_set_theme(cx: &mut TestAppContext) {
        cx.set_global(Theme::default());
        assert!(cx.read(|cx| colours(Theme::get(cx)) == colours(Theme::default())));
        cx.set_global(nord());
        assert!(cx.read(|cx| colours(Theme::get(cx)) == colours(nord())));
    }

    #[gpui::test]
    #[should_panic(expected = "no state of type waybar_gpui::theme::Theme exists")]
    fn get_panics_without_a_theme(cx: &mut TestAppContext) {
        cx.read(Theme::get);
    }

    /// gpui-component keeps `Hsla`: compare as `0xrrggbbaa`, past rounding.
    #[cfg(feature = "components")]
    fn hex(c: impl Into<Rgba>) -> u32 {
        let c = c.into();
        u32::from_be_bytes([c.r, c.g, c.b, c.a].map(|v| (v * 255.0).round() as u8))
    }

    #[cfg(feature = "components")]
    #[gpui::test]
    fn apply_to_components_maps_our_colours(cx: &mut TestAppContext) {
        use gpui_component::theme::Theme as ComponentTheme;

        let theme = nord();
        cx.update(|cx| {
            gpui_component::init(cx);
            apply_to_components(&theme, cx);
        });
        cx.read(|cx| {
            let colors = &ComponentTheme::global(cx).colors;
            assert_eq!(hex(colors.background), hex(theme.base));
            assert_eq!(hex(colors.foreground), hex(theme.text));
            assert_eq!(hex(colors.border), hex(theme.overlay));
            assert_eq!(hex(colors.muted), hex(theme.surface));
            assert_eq!(hex(colors.muted_foreground), hex(theme.subtext));
            assert_eq!(hex(colors.primary), hex(theme.accent));
            assert_eq!(hex(colors.primary_foreground), hex(Rgba { a: 1.0, ..theme.base }));
            assert_eq!(hex(colors.popover), hex(theme.surface));
            assert_eq!(hex(colors.ring), hex(theme.accent));
        });
    }
}
