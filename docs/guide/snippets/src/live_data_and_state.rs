//! docs/guide/live-data-and-state.md

use std::time::Duration;

use waybar_gpui::{
    Panel, card,
    gpui::{Context, Global, Task, Window, prelude::*},
};

// ANCHOR: tick_field
pub struct ClockPanel {
    _tick: Task<()>,
    // …
}
// ANCHOR_END: tick_field

impl Panel for ClockPanel {
    const NAME: &'static str = "clock";

    fn new(_args: &[String], _window: &mut Window, cx: &mut Context<Self>) -> Self {
        // ANCHOR: tick
        let _tick = cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(Duration::from_secs(1)).await;
            if this.update(cx, |_, cx| cx.notify()).is_err() {
                return; // the panel is gone
            }
        });
        // ANCHOR_END: tick
        Self { _tick }
    }
}

/// A stand-in for example-panels' Hyprland client (examples/example-panels/src/hypr.rs),
/// with the same shape: a blocking iterator over the event socket.
mod hypr {
    use std::io;

    pub struct Events;

    impl Events {
        pub fn connect() -> io::Result<Self> {
            Ok(Events)
        }
    }

    impl Iterator for Events {
        type Item = io::Result<String>;

        fn next(&mut self) -> Option<Self::Item> {
            None
        }
    }
}

pub struct WorkspacePanel {
    _events: Task<()>,
}

impl WorkspacePanel {
    fn refresh(&mut self) {}
}

impl Panel for WorkspacePanel {
    const NAME: &'static str = "workspaces";

    fn new(_args: &[String], _window: &mut Window, cx: &mut Context<Self>) -> Self {
        // ANCHOR: events
        let _events = cx.spawn(async move |this, cx| {
            let Ok(mut events) = hypr::Events::connect() else { return };
            loop {
                let (back, event) = cx
                    .background_executor()
                    .spawn(async move { let event = events.next(); (events, event) })
                    .await;
                events = back;
                let Some(Ok(_event)) = event else { return };
                if this.update(cx, |panel, cx| { panel.refresh(); cx.notify(); }).is_err() {
                    return;
                }
            }
        });
        // ANCHOR_END: events
        Self { _events }
    }
}

// ANCHOR: global
#[derive(Default)]
struct Opens(u32);

impl Global for Opens {}
// ANCHOR_END: global

pub struct CounterPanel {
    opens: u32,
}

impl Panel for CounterPanel {
    const NAME: &'static str = "counter";

    fn new(_args: &[String], _window: &mut Window, cx: &mut Context<Self>) -> Self {
        // ANCHOR: opens
        let opens = {
            let opens = cx.default_global::<Opens>();
            opens.0 += 1;
            opens.0
        };
        // ANCHOR_END: opens
        Self { opens }
    }
}

impl Render for ClockPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        card(cx)
    }
}

impl Render for WorkspacePanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        card(cx)
    }
}

impl Render for CounterPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        card(cx).child(format!("opened {} times", self.opens))
    }
}
