//! docs/guide/components.md, with the `components` feature.

use waybar_gpui::{
    Panel, card, close,
    gpui::{Context, Window, prelude::*},
};

pub struct ControlsPanel {
    wifi: bool,
}

impl Panel for ControlsPanel {
    const NAME: &'static str = "controls";

    fn new(_args: &[String], _window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self { wifi: true }
    }
}

impl Render for ControlsPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // ANCHOR: widgets
        use waybar_gpui::components::{button::{Button, ButtonVariants as _}, switch::Switch};

        card(cx)
            .child(Switch::new("wifi").checked(self.wifi).on_click(cx.listener(|this, on, _, cx| {
                this.wifi = *on;
                cx.notify();
            })))
            .child(Button::new("done").primary().label("Done").on_click(|_, _, cx| close(cx)))
        // ANCHOR_END: widgets
    }
}
