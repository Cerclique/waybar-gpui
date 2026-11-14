//! docs/guide/closing-and-esc.md

use waybar_gpui::{
    Panel, card, close,
    gpui::{self, Context, FocusHandle, IntoElement, Render, Stateful, Window, div, prelude::*},
};

fn row() -> Stateful<gpui::Div> {
    // ANCHOR: close
    div()
        .id("row")
        .on_click(move |_, _, cx| {
            std::process::Command::new("hyprctl").args(["dispatch", "…"]).spawn().ok();
            close(cx);
        })
    // ANCHOR_END: close
}

pub struct EditPanel {
    focus: FocusHandle,
    editing: bool,
}

impl Panel for EditPanel {
    const NAME: &'static str = "edit";

    // ANCHOR: focus
    fn new(_args: &[String], window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        Self { focus, editing: false }
    }
    // ANCHOR_END: focus
}

impl Render for EditPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // ANCHOR: keys
        card(cx)
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                if event.keystroke.key == "escape" && this.editing {
                    this.editing = false;
                    cx.notify();
                    cx.stop_propagation(); // first Esc leaves edit mode, the popover stays
                }
            }))
        // ANCHOR_END: keys
    }
}
