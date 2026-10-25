# Using a waybar-gpui program

This page is for people who install a program built with waybar-gpui (say,
`my-panels`) to get popovers on their Waybar. You don't need to write Rust.
Requirements: Hyprland, Waybar, a Rust toolchain to build it, and a working
Vulkan driver.

## Install

Follow the program's own instructions; usually:

```sh
git clone <the program's repository> && cd <it>
cargo install --path . --locked
```

## Add it to Waybar

Run the program with no arguments. It lists its panels, each with the line to
paste into a Waybar bar module:

```text
panels and their Waybar on-click lines:
  clock        "on-click": "/home/me/.cargo/bin/my-panels toggle clock"
```

In `~/.config/waybar/config.jsonc`, add a bar module and put it in a
`modules-*` list:

```jsonc
"custom/clock": {
  "exec": "echo clock",
  "interval": "once",
  "exec-on-event": false,
  "on-click": "/home/me/.cargo/bin/my-panels toggle clock",
},
```

Keep the full path: Waybar usually can't find programs in `~/.cargo/bin` by
name. Some panels take extra words after their name (`toggle clock Paris 1`);
the program's README says which. Reload Waybar with `pkill -SIGUSR2 waybar`.

Optional, in `~/.config/hypr/hyprland.lua`, for blur behind every waybar-gpui
popover:

```lua
hl.layer_rule({ match = { namespace = "^waybar-gpui:" }, blur = true, ignore_alpha = 0.3, animation = "fade" })
```

## Using it

- Click the bar module: the popover opens next to it.
- Close it with Esc, a click anywhere outside it, or a click on the bar.
- While a popover is open, **the first click elsewhere only closes it**:
  clicking another bar module takes a second click. That's how Hyprland
  routes clicks to popovers, not a bug.
- Only one popover is open at a time, even across different programs.
- The first click after login takes a moment longer (the program starts in
  the background); later ones are fast.

## When something is wrong

- **Nothing opens:** check the path in `on-click` (run it in a terminal), and
  read the log: `~/.local/state/waybar-gpui/<program>.log`.
- **It still shows an old version after an update:** click again; a click
  replaces an outdated background process. `my-panels quit` stops it outright.
- **Uninstall:** `my-panels quit`, then `cargo uninstall my-panels`, then
  remove the bar module.
