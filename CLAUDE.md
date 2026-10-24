# CLAUDE.md

Template for custom Waybar modules on Hyprland with a GPUI popover. Start with
`docs/session-notes.md` (history, decisions, open TODOs), then `README.md`.

- Build/test: `cargo build`, `cargo test`, `make lint` (clippy `-D warnings`).
- Try it: `make dev` runs a second Waybar from `examples/waybar/`.
- GPUI is `gpui-pre =0.3.9` and the API is pre-1.0: check the source in
  `~/.cargo/registry/src/*/gpui-pre-0.3.9/` before using an API; don't bump the
  pin casually.
- Hyprland is 0.56 with a **Lua** config (`hl.*`), not hyprlang. Verify
  dispatcher/rule syntax against the wiki before writing it.
- Waybar is 0.15.0; the Waybar wiki documents master, which differs.
- The live tests open popovers on the user's screen; say so before running them.
