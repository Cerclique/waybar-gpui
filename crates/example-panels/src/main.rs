//! An author's whole program: register panels, hand over to the SDK.
//!
//! Each pattern the guide describes is shown in exactly one panel:
//!
//! | Pattern | Panel |
//! |---|---|
//! | panel arguments | `clock.rs` |
//! | live data from a timer | `clock.rs` |
//! | live data from a blocking event source | `workspace.rs` |
//! | state that survives a close (`Global`) | `clock.rs` |
//! | handling Esc in the panel (`stop_propagation`) | `clock.rs` |
//! | closing from the panel (`close`) | `workspace.rs` |
//! | GPUI derive macros (`use waybar_gpui::gpui;`) | `workspace.rs` |
//! | gpui-component widgets (`components` feature) | `controls.rs` |
//!
//! Waybar `on-click` lines (see `waybar/config.jsonc` in this crate), or try
//! any of them with `preview` instead of `toggle`:
//!   example-panels toggle workspace
//!   example-panels toggle clock Paris 1
//!   example-panels toggle controls    (with the `components` feature)

mod clock;
#[cfg(feature = "components")]
mod controls;
mod workspace;

fn main() -> std::process::ExitCode {
    let app = waybar_gpui::App::new("example-panels")
        .panel::<workspace::WorkspacePanel>()
        .panel::<clock::ClockPanel>();
    #[cfg(feature = "components")]
    let app = app.panel::<controls::ControlsPanel>();
    app.run()
}
