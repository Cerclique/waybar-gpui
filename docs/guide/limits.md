# Limits

## By design

- **Hyprland only.** Placement and click handling depend on Hyprland's IPC and
  how it routes input to layer surfaces. Other compositors (Sway, niri…) aren't
  supported.
- **Waybar can't be clicked while a popover is open.** The first click outside
  the card only closes the popover; clicking another bar module takes a second
  click. It comes from how Hyprland routes clicks; nothing in the SDK can
  change it.
- **One popover at a time,** across all programs.
- **Placement is fixed:** next to the bar, centred on the cursor. Panels don't
  choose an alignment or an offset.
- **Panels don't open other panels.** Only bar-module clicks (or `toggle` from
  a script or keybind) open panels; several views inside one panel are plain
  GPUI state.
- **No gpui-component dialogs or notifications** (they need its `Root`, which
  breaks layer surfaces). Menus and in-window popovers work. See
  [Components](components.md).
- **Other exclusive surfaces** (rofi, fuzzel, wlogout) opened over a popover
  interact awkwardly: moving the pointer can hand the keyboard back to the
  popover, and a full-screen one blocks it until it closes.
- **The theme is set in code** (`App::theme`): people who install a program
  can't restyle it with a theme file.
- **The SDK covers the popover, not the bar.** What the bar module shows is
  ordinary Waybar `custom/*` configuration.

## Versions

- The SDK is a **git dependency** with version tags (`tag = "vX.Y.Z"`), not a
  crates.io crate: GPUI isn't stable enough to publish against.
- **0.x semver:** a breaking change bumps the minor version (`v0.1.x` →
  `v0.2.0`); fixes bump the patch.
- GPUI (`gpui-pre`) and gpui-component are **pinned exactly**, and move
  together. Every change to those pins is a breaking release, with the new
  versions in the changelog. Upgrade deliberately, and keep your `Cargo.lock`.
- Tested with Hyprland 0.56 (Lua config), Waybar 0.15 and Rust 1.97.1; the
  oldest Rust that builds it is 1.95 (`rust-version`, checked in CI). GPUI
  renders with Vulkan.
