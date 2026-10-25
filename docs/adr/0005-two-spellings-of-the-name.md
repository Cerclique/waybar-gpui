# Two spellings of the name

The project is `waybar-gpui` and the crate is `waybar_gpui`. The hyphen spelling names the project and everything it puts outside Rust: the repository, the guide's titles, the socket and log directories (`$XDG_RUNTIME_DIR/waybar-gpui/`, `$XDG_STATE_HOME/waybar-gpui/`), the Hyprland layer namespace `waybar-gpui:<id>` and the app ID `waybar-gpui-<id>`. The underscore spelling names only the crate, written as code: the dependency line, `use waybar_gpui::…`, and its crates.io and docs.rs pages. A single spelling would mean changing one side, and neither can change cheaply. crates.io treats `-` and `_` as the same name, so the published crate can't be renamed, only abandoned for a new one. The runtime names are matched by people's Hyprland layer rules and quoted in bug reports, so changing them is a breaking release that silently stops those rules from matching.

## Consequences

- A heading, title or sentence about the project says waybar-gpui; one about the crate says `waybar_gpui`, in backticks. When a sentence could mean either, say "the `waybar_gpui` crate".
