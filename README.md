# waybar_gpui_plugin_template

Template for custom Waybar modules on Hyprland, with a GPUI popover opened by
clicking the module.

```
 Waybar ── exec ──────▶ waybar-hypr-ws  ◀── Hyprland .socket2.sock (events)
   │                    prints one JSON line per change
   └── on-click ──────▶ example-panels toggle workspace
                            │ Unix socket ($XDG_RUNTIME_DIR/waybar-gpui/example-panels.sock)
                            ▼
                        example-panels daemon (GPUI, stays running)
                        opens/closes a layer-shell popover under the cursor
```

| Path | What |
|---|---|
| `crates/hypr` | Minimal Hyprland IPC: `request::<T>("activeworkspace")`, `Events` iterator. No dependencies besides serde. |
| `crates/module` | `waybar-hypr-ws`, a continuous `custom/*` module (active workspace). Event-driven, no polling. |
| `crates/waybar_gpui` | The SDK: `Panel`, `App`, `card`, `Theme`, `close`; the runtime (daemon, placement, dismissal) is private inside it. |
| `crates/example-panels` | `example-panels`, a program built on the SDK the way an author's would be. |
| `examples/waybar/` | Waybar config + CSS (also used by `scripts/dev.sh`): a top bar and a left bar. |
| `examples/hypr/waybar-gpui.lua` | Optional Hyprland layer rule (blur/animation) for every program's popovers. |
| `docs/spec.md` | What is being built: the runtime + SDK spec. |
| `docs/research/` | Background research: Waybar extension mechanisms, GPUI on Wayland. |

## Requirements

Hyprland, Waybar ≥ 0.10, Rust stable (tested with 1.97.1), a working Vulkan
driver (GPUI renders with wgpu/Vulkan).

## Try it

```sh
make dev          # builds, runs a second Waybar with examples/waybar/, Ctrl-C to stop
```

Click the workspace label on the test bar to open the popover. Esc or a click
anywhere outside it (including the label again) closes it.

## Install into your own bar

```sh
make install      # release build into ~/.local/bin, then reloads Waybar
```

Copy the `"custom/hypr-ws"` block from `examples/waybar/config.jsonc` into
`~/.config/waybar/config.jsonc`, add it to a `modules-*` list, and copy the CSS
rules into your `style.css`. Waybar runs commands with its own `PATH`; if
`~/.local/bin` isn't on it, use absolute paths in `exec` and `on-click`
(`~` works: Waybar runs them through `/bin/sh -c`).

## How the popover works

Wayland only lets a real popover (`xdg_popup`) attach to a surface owned by the
same process, so a separate program can't attach one to Waybar. Instead the
daemon opens a **layer-shell surface** on the overlay layer and draws the panel
under the clicked icon, positioned from `hyprctl cursorpos` and the monitor's
reserved area (`crates/waybar_gpui/src/placement.rs`).
It has no border or title bar and shows on the monitor you clicked.

The surface actually covers the **whole output**, bar included, and is
transparent except for the panel card drawn at the computed position. It has
exclusive keyboard focus, so Esc always works, and a click anywhere outside the
card (including on the module again) lands on the same surface and closes it.
This avoids two dead ends found while testing: closing on focus loss (Hyprland's
`follow_mouse` moves focus as soon as the pointer moves) and a separate
backdrop surface (it never received clicks while an exclusive-keyboard layer
was open). Open/close events are logged to stderr, which ends up in Waybar's
log.

The first `toggle` starts the daemon in the background (~100 ms to show the
first popover). Later ones only create a small surface (<20 ms), because the
GPU device and fonts stay loaded. `example-panels quit` stops it.

## Adding things

- **New module output:** edit `render()` in `crates/module/src/main.rs` and
  `RELEVANT_EVENTS`. The JSON fields are in `output.rs` (`man 5 waybar-custom`).
- **New popover panel:** implement `waybar_gpui::Panel` (a `NAME` and a
  `new`) plus GPUI `Render` like `crates/example-panels/src/workspace.rs`,
  wrap the content in `card(cx)`, and register it with `.panel::<YourPanel>()`
  in `main.rs`. Point an `on-click` at `example-panels toggle <name>`.
- **More Hyprland data:** add a struct to `crates/hypr` for any `hyprctl -j`
  command.

## Caveats

- GPUI comes from the community `gpui-pre` snapshot crates, pinned to
  `=0.3.9`. The official `gpui` crate on crates.io is outdated and lacks
  layer-shell support. The API changes between snapshots, so upgrade
  deliberately and keep `Cargo.lock`.
- While the popover is open, the first click outside it only closes it; it
  doesn't reach the window below.
- Waybar runs one module process per monitor. That's expected.
