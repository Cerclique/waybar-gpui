//! Smoke tests: build each panel in a headless GPUI window, without the
//! runtime, and render it. They pass if nothing panics, so an SDK change that
//! breaks a panel at build or render time fails `cargo test`, not a click.
//!
//! The panels are listed by hand (`Program` doesn't expose its panels): a new
//! panel in `main` needs a new test here.

use std::time::Duration;

use gpui::{TestAppContext, WindowHandle};
use waybar_gpui::{Panel, Theme};

use crate::{clock::ClockPanel, workspace::WorkspacePanel};

/// Builds `P` with `args` in a new test window. Opening the window renders
/// the panel once.
fn open<P: Panel + 'static>(cx: &mut TestAppContext, args: &[&str]) -> WindowHandle<P> {
    // The runtime sets the `Theme` global before building a panel; `card`
    // and `Theme::get` panic without it.
    cx.update(|cx| cx.set_global(Theme::default()));
    let args: Vec<String> = args.iter().map(|&arg| arg.to_owned()).collect();
    cx.add_window(|window, cx| P::new(&args, window, cx))
}

#[gpui::test]
fn workspace_renders(cx: &mut TestAppContext) {
    // Renders "Hyprland unavailable" without Hyprland (as on CI) and the live
    // session, read-only, with it.
    let window = open::<WorkspacePanel>(cx, &[]);
    // The test executor runs background tasks on this thread, so with
    // Hyprland the panel's event task would block it on the event socket.
    // Close the window before the executor runs: dropping the panel cancels
    // the task unpolled.
    window.update(cx, |_, window, _| window.remove_window()).unwrap();
    cx.run_until_parked();
}

#[gpui::test]
fn clock_renders_without_arguments(cx: &mut TestAppContext) {
    open::<ClockPanel>(cx, &[]);
    cx.run_until_parked();
}

#[gpui::test]
fn clock_renders_with_label_and_offset(cx: &mut TestAppContext) {
    open::<ClockPanel>(cx, &["Paris", "1"]);
    cx.run_until_parked();
}

#[gpui::test]
fn clock_renders_with_malformed_offset(cx: &mut TestAppContext) {
    open::<ClockPanel>(cx, &["Paris", "x"]);
    cx.run_until_parked();
}

#[gpui::test]
fn clock_rerenders_on_tick(cx: &mut TestAppContext) {
    open::<ClockPanel>(cx, &[]);
    cx.run_until_parked();
    // Past the 1-second timer: the tick notifies, and the test app redraws.
    cx.executor().advance_clock(Duration::from_millis(1100));
    cx.run_until_parked();
}

#[cfg(feature = "components")]
#[gpui::test]
fn controls_renders(cx: &mut TestAppContext) {
    // The runtime does this before building a panel; without our theme mapped
    // onto it, gpui-component uses its default theme, which is fine here.
    cx.update(waybar_gpui::components::init);
    open::<crate::controls::ControlsPanel>(cx, &[]);
    cx.run_until_parked();
}
