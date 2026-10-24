//! An author's whole program: register panels, hand over to the SDK.
//!
//! Waybar `on-click` lines (see `examples/waybar/config.jsonc`):
//!   example-panels toggle workspace
//!   example-panels toggle controls    (with the `components` feature)

#[cfg(feature = "components")]
mod controls;
mod workspace;

fn main() -> std::process::ExitCode {
    let app = waybar_gpui::App::new("example-panels").panel::<workspace::WorkspacePanel>();
    #[cfg(feature = "components")]
    let app = app.panel::<controls::ControlsPanel>();
    app.run()
}
