//! docs/guide/writing-a-panel.md

// ANCHOR: program
use waybar_gpui::{card, gpui::{prelude::*, Context, Window}, Panel};

struct Hello;

impl Panel for Hello {
    const NAME: &'static str = "hello";
    fn new(_args: &[String], _window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Hello
    }
}

impl Render for Hello {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        card(cx).child("Hello")
    }
}

fn main() -> std::process::ExitCode {
    waybar_gpui::Program::new("my-panels").panel::<Hello>().run()
}
// ANCHOR_END: program

// ANCHOR: derive
use waybar_gpui::gpui;
// ANCHOR_END: derive

use std::process::ExitCode;

use waybar_gpui::gpui::{App, Div, div};

stand_in!(Clock = "clock", Volume = "volume");

fn size(rows: Vec<Div>, cx: &App) -> impl IntoElement {
    // ANCHOR: size
    card(cx)
        .w(gpui::px(340.))
        .max_h(gpui::px(420.))
        .child(div().id("rows").flex().flex_col().overflow_y_scroll().children(rows))
    // ANCHOR_END: size
}

fn several() -> ExitCode {
    // ANCHOR: several
    waybar_gpui::Program::new("my-panels")
        .panel::<Clock>()
        .panel::<Volume>()
        .run()
    // ANCHOR_END: several
}
