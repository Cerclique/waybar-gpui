//! A clock for a given UTC offset. Shows:
//! - panel arguments: `toggle clock <label> <utc-offset-hours>`
//! - live data from a timer, stopped on close because the panel owns the task
//! - a panel handling Esc itself: press `e` to enter edit mode; the first Esc
//!   leaves edit mode (`cx.stop_propagation()` keeps the popover open), the
//!   second closes the popover
//! - state that survives a close, in a GPUI `Global`: the open count

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use waybar_gpui::{
    Panel, Theme, card,
    gpui::{self, Context, FocusHandle, Global, IntoElement, Task, Window, div, prelude::*},
};

/// Lives in the daemon, not in the panel, so it outlives each popover.
#[derive(Default)]
struct Opens(u32);

impl Global for Opens {}

pub struct ClockPanel {
    label: String,
    offset_hours: i64,
    editing: bool,
    opens: u32,
    focus: FocusHandle,
    _tick: Task<()>,
}

impl Panel for ClockPanel {
    const NAME: &str = "clock";

    fn new(args: &[String], window: &mut Window, cx: &mut Context<Self>) -> Self {
        let label = args.first().cloned().unwrap_or_else(|| "UTC".into());
        let offset_hours = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);

        let opens = {
            let opens = cx.default_global::<Opens>();
            opens.0 += 1;
            opens.0
        };

        // Focused in `new`, so key presses (Esc included) reach the panel first.
        let focus = cx.focus_handle();
        window.focus(&focus, cx);

        // Dropped with the panel on close, which stops the timer.
        let _tick = cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                if this.update(cx, |_, cx| cx.notify()).is_err() {
                    return;
                }
            }
        });
        Self { label, offset_hours, editing: false, opens, focus, _tick }
    }
}

impl Render for ClockPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::get(cx);
        let secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64;
        let local = (secs + self.offset_hours * 3600).rem_euclid(86_400);
        let time = format!("{:02}:{:02}:{:02}", local / 3600, local / 60 % 60, local % 60);

        card(cx)
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                match event.keystroke.key.as_str() {
                    "e" => {
                        this.editing = true;
                        cx.notify();
                    }
                    "escape" if this.editing => {
                        this.editing = false;
                        cx.notify();
                        // Keep the popover open: the runtime's Esc handler
                        // never sees this one.
                        cx.stop_propagation();
                    }
                    _ => {}
                }
            }))
            .child(div().text_color(theme.subtext).child(format!("{} (UTC{:+})", self.label, self.offset_hours)))
            .child(div().text_2xl().child(time))
            .child(div().text_xs().text_color(theme.subtext).child(format!(
                "opened {} times · {}",
                self.opens,
                if self.editing { "EDIT MODE: Esc leaves it" } else { "press e for edit mode" }
            )))
            .when(self.editing, |card| card.border_color(theme.accent))
    }
}
