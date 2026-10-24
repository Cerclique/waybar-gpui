//! An author's whole program: register panels, hand over to the SDK.
//!
//! Waybar `on-click` lines (see `examples/waybar/config.jsonc`):
//!   example-panels toggle workspace

mod workspace;

fn main() -> std::process::ExitCode {
    waybar_gpui::App::new("example-panels").panel::<workspace::WorkspacePanel>().run()
}
