# {{project-name}}

Popovers for Waybar on Hyprland, built with
[waybar-gpui](https://github.com/Cerclique/waybar-gpui/blob/v0.1.0/README.md).

## Develop

```sh
cargo run -- preview hello      # opens the Hello panel at your cursor; Esc closes it
```

`preview` needs no Waybar. To try a panel on your real bar, point a bar
module's `on-click` at your build:
`"/path/to/{{project-name}}/target/debug/{{project-name}} toggle hello"`. Each
`cargo build` shows up on the next click: a click replaces a daemon started
from an older binary.

Write panels in `src/`, register them in `src/main.rs`. Guide:
[writing a panel](https://github.com/Cerclique/waybar-gpui/blob/v0.1.0/docs/guide/writing-a-panel.md),
[dev loop](https://github.com/Cerclique/waybar-gpui/blob/v0.1.0/docs/guide/dev-loop.md),
[all chapters](https://github.com/Cerclique/waybar-gpui/blob/v0.1.0/docs/guide/index.md).

Commit the `Cargo.lock` from your first build: installs use `--locked`.

## Install

```sh
cargo install --path . --locked
{{project-name}}                 # lists the panels with their Waybar on-click lines
```

Paste a line into a `custom/*` bar module in `~/.config/waybar/config.jsonc`
and reload Waybar (`pkill -SIGUSR2 waybar`). Full steps, for you and for
people installing this program:
[Using a waybar-gpui program](https://github.com/Cerclique/waybar-gpui/blob/v0.1.0/docs/guide/using-a-program.md).

## Panels

| Panel | on-click | Arguments |
|---|---|---|
| `hello` | `~/.cargo/bin/{{project-name}} toggle hello` | none |
