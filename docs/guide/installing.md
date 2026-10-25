# Installing

## Build and install

In your program's repository:

```sh
cargo install --path . --locked
```

This puts the binary in `~/.cargo/bin`. Commit your `Cargo.lock` so `--locked`
installs exactly what you tested. To update later, pull and run the same
command: the next click replaces the running daemon with the new binary, with
no Waybar reload.

## Waybar

Run your program with no arguments: the usage text ends with a ready-to-paste
`on-click` line per panel, using the real path of the binary:

```text
panels and their Waybar on-click lines:
  hello        "on-click": "/home/me/.cargo/bin/my-panels toggle hello"
```

Waybar started by Hyprland usually doesn't have `~/.cargo/bin` on its `PATH`,
so use the full path. `~` works too: Waybar runs `on-click` through
`/bin/sh -c`. A bar module for it, in `~/.config/waybar/config.jsonc`:

```jsonc
"modules-right": ["custom/hello", …],

"custom/hello": {
  "exec": "echo hello",        // what the bar shows; any custom/* program works
  "interval": "once",
  "exec-on-event": false,      // no need to re-run exec after a click
  "on-click": "~/.cargo/bin/my-panels toggle hello",
},
```

The bar module's own output (`exec`, `format`, styling) is ordinary Waybar
`custom/*` configuration (`man 5 waybar-custom`); the SDK only handles the
popover. If `~/.cargo/bin` *is* on Waybar's `PATH`, the bare name works:
`"on-click": "my-panels toggle hello"`.

After editing the config, reload Waybar: `pkill -SIGUSR2 waybar`.

## Hyprland

Nothing is required: the popover is a layer surface that places itself. One
optional rule, for every waybar-gpui program at once, adds blur behind the
card and a fade (Hyprland 0.55+ Lua config, `~/.config/hypr/hyprland.lua`):

```lua
hl.layer_rule({
  match = { namespace = "^waybar-gpui:" },
  blur = true,
  ignore_alpha = 0.3,   -- blur only the card, not the transparent rest
  animation = "fade",
})
```

For one program only, match `^waybar-gpui:<id>$`. The same rule is in
[`examples/hypr/waybar-gpui.lua`](../../examples/hypr/waybar-gpui.lua).

## Uninstalling

Stop the daemon first, or it keeps running from the deleted binary until you
log out:

```sh
my-panels quit
cargo uninstall my-panels
```

Then remove the bar modules from your Waybar config.

## For your users

People installing your program need the same steps without the Rust: link
[Using a waybar-gpui program](using-a-program.md) from your README (the
starter template's README already does).
