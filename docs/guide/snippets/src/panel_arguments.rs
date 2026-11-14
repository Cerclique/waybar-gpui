//! docs/guide/panel-arguments.md

use waybar_gpui::{
    Panel, card,
    gpui::{Context, Window, prelude::*},
};

pub struct ClockPanel {
    label: String,
    offset_hours: i64,
}

// ANCHOR: new
impl Panel for ClockPanel {
    const NAME: &'static str = "clock";

    fn new(args: &[String], _window: &mut Window, _cx: &mut Context<Self>) -> Self {
        let label = args.first().cloned().unwrap_or_else(|| "UTC".into());
        let offset_hours = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
        Self { label, offset_hours }
    }
}
// ANCHOR_END: new

impl Render for ClockPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        card(cx).child(format!("{} (UTC{:+})", self.label, self.offset_hours))
    }
}
