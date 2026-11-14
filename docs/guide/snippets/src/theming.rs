//! docs/guide/theming.md

use std::process::ExitCode;

use waybar_gpui::gpui::{App, Div, div, prelude::*};

stand_in!(Hello = "hello");

fn main() -> ExitCode {
    // ANCHOR: custom
    use waybar_gpui::{gpui::{rgb, rgba}, Theme};

    let mut theme = Theme::default();
    theme.base = rgba(0x2e3440f0);
    theme.accent = rgb(0x88c0d0);
    waybar_gpui::Program::new("my-panels").theme(theme).panel::<Hello>().run()
    // ANCHOR_END: custom
}

fn secondary(cx: &App) -> Div {
    use waybar_gpui::Theme;

    // ANCHOR: use
    let theme = Theme::get(cx);
    div().text_color(theme.subtext).child("secondary")
    // ANCHOR_END: use
}
