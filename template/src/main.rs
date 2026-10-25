//! {{project-name}}: Waybar popovers built with waybar_gpui.

mod hello;

fn main() -> std::process::ExitCode {
    // The program ID names this program's socket, log file and Hyprland layer
    // namespace (`waybar-gpui:{{id}}`). Keep it stable once bars point at it.
    waybar_gpui::App::new("{{id}}").panel::<hello::Hello>().run()
}
