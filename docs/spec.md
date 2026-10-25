# Spec: waybar-gpui runtime + panel SDK (v0.1.0)

What to build when this repo becomes `waybar-gpui`: a Rust **SDK** that lets an
**author** write **panels** in their own repository, and a **runtime** that opens
a panel as a **popover** when a Waybar **bar module** is clicked. Terms follow
[`GLOSSARY.md`](../GLOSSARY.md).

Every decision here came from a closed ticket of the map
[Wayfinder: split into runtime + panel SDK][map]. Each section links its sources;
the tickets hold the alternatives that were rejected and why. The prototype on
branch [`prototype/panel-api`][proto] (never merged) validated the API on screen.

Targets: Hyprland 0.56 (Lua config), Waybar 0.15.0, `gpui-pre =0.3.9`,
Rust 1.97.1.

## 1. Scope

In scope:

- One library crate, `waybar_gpui`, that an author's program depends on as a git
  dependency at a `vX.Y.Z` tag.
- The runtime inside it: click client, daemon, placement, dismissal,
  coordination between programs.
- Example panels, a starter template, CI, and documentation.

Out of scope for this effort: compositors other than Hyprland; crates.io; help for
the bar side (the JSON a `custom/*` `exec` prints); panels opening other panels;
placement options; a user-editable theme file; interplay with third-party
exclusive surfaces (rofi, fuzzel, wlogout); gpui-component dialogs and
notifications.

## 2. Repository layout

Source: [Crate layout][t18].

| Path | What | Status |
|---|---|---|
| `crates/waybar_gpui` | The SDK; the runtime is a private module inside it | new |
| `crates/hypr` | Hand-rolled Hyprland IPC; internal workspace crate, not re-exported, not covered by tags | stays |
| `crates/example-panels` | `publish = false` binary playing the author, path dependency on `waybar_gpui`, may use `hypr`; receives the workspace panel | new |
| `crates/module` (`waybar-hypr-ws`) | Bar-module example | stays |
| `template/` | cargo-generate starter (§9) | new |
| `examples/waybar/`, `examples/hypr/` | Waybar and Hyprland configs for `make dev` and the docs | stay, updated |
| `crates/popup`, `python/hypr_ws.py` | Today's daemon and Python module | deleted |

- `make dev` builds, puts `target/debug` first on PATH and runs a second Waybar
  wired to `waybar-hypr-ws` and `example-panels`. It no longer needs `quit` (§6.5).
- **Versioning:** plain `vX.Y.Z` repo tags, always equal to `waybar_gpui`'s version;
  every crate inherits the single `[workspace.package] version`. 0.x semver: a
  breaking change bumps the minor. Any change to the three pins (`gpui-pre`,
  `gpui-pre-platform`, `gpui-component`) is a breaking minor release with a
  changelog naming the new versions ([GPUI exposure][t15]).
- **Repo:** renamed `waybar_gpui_plugin_template` → `waybar-gpui` before
  `v0.1.0`; private until `v0.1.0` is released, public from then on ([map]).

## 3. SDK public API

Sources: [Panel API][t17], [Panel lifecycle][t8], [Panel actions][t9],
[Panel theming][t11], [GPUI exposure][t15].

An author's whole program:

```rust
use waybar_gpui::{card, gpui::{self, prelude::*, Context, Window}, Panel};

struct Hello;

impl Panel for Hello {
    const NAME: &'static str = "hello";
    fn new(_args: &[String], _window: &mut Window, _cx: &mut Context<Self>) -> Self { Hello }
}

impl Render for Hello {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        card(cx).child("Hello")
    }
}

fn main() -> std::process::ExitCode {
    waybar_gpui::App::new("my-panels").panel::<Hello>().run()
}
```

Public items, and nothing else:

| Item | Contract |
|---|---|
| `trait Panel: Render + Sized` | `const NAME: &'static str` (what `toggle` names) and `fn new(args: &[String], window, cx) -> Self`. `args` are the panel arguments. |
| `App::new(id)` | Starts a program with its program ID. Panics with a clear message unless `id` matches `^[a-z0-9][a-z0-9-]*$` ([Starter template][t21]). |
| `App::theme(Theme)` | Optional; defaults to `Theme::default()`. |
| `App::panel::<P>()` | Registers a panel. Registering two panels with the same `NAME` panics. |
| `App::run() -> ExitCode` | Parses `std::env::args()` and runs the command (§5). |
| `card(cx) -> Div` | Opt-in card chrome: rounded corners, border, translucent background, padding, text colour and size, all from `Theme`. |
| `Theme` | Plain struct of colours, stored by the runtime as a GPUI global; `Theme::get(cx)`. Prototype fields: `base`, `surface`, `overlay`, `text`, `subtext`, `accent`. `Default` is Catppuccin Mocha. |
| `close(cx)` | Closes the program's open popover. Deferred, so it is safe inside the panel's own event handlers. |
| `pub use gpui` | GPUI re-exported; authors never depend on it directly. Its derive macros emit `gpui::…` paths, so modules using them write `use waybar_gpui::gpui;`. |
| `pub use gpui_component as components` | Only with the `components` feature (§8). |

Behaviour the API implies:

- **Fresh per open:** the runtime builds the panel each time its popover opens
  and drops it on close. Tasks stored in the panel's fields stop on close.
  Opening another panel closes the first and builds the second fresh.
- **Live data and state that survives a close:** plain GPUI, no SDK helper
  (`cx.spawn` tasks + `cx.notify()`; a `Global` or an `Entity`). Shown in
  `example-panels` and the guide.
- **Actions:** the SDK offers only runtime actions (`close`). Commands and
  Hyprland dispatches are the author's own code.
- **Size:** panels declare no size. They cap themselves (`w(..)`, `max_h(..)`)
  and scroll their own overflow (`overflow_y_scroll`).

## 4. Popover behaviour

Sources: [Popover sizing and placement][t10], [Panel actions][t9],
[Panel API][t17], [Hyprland: two exclusive layers][t12].

- **Surface:** one layer surface per popover on the Overlay layer, covering the
  whole monitor, `exclusive_zone = -1`, `KeyboardInteractivity::Exclusive`,
  transparent, layer namespace `waybar-gpui:<id>`. The card is drawn inside it;
  the rest is the backdrop. The runtime paints no chrome of its own.
- **Monitor:** the one under the cursor (Hyprland `j/cursorpos` + `monitors`),
  matched to GPUI's display by `Uuid::new_v5(NAMESPACE_DNS, wl_output name)`.
- **Placement:** GPUI `anchored()` measures and places the card in the same
  frame. The bar edge is the reserved edge of the monitor nearest the click.
  Horizontal bar: centred on the cursor's x (`TopCenter`/`BottomCenter`);
  vertical bar: centred on the cursor's y (`LeftCenter`/`RightCenter`); 6 px
  from the bar; snapped into the usable area (`snap_to_window_with_margin`).
  When content grows while open, the card is re-placed on the next layout.
  No placement options.
- **Focus and Esc:** a panel that focuses one of its own elements in `new` gets
  Esc first; if it calls `cx.stop_propagation()` the popover stays open. The
  runtime focuses its root only when nothing is focused. Unhandled Esc dismisses.
- **Dismissal:** a click that reaches the backdrop dismisses; the card and any
  overlay a widget draws (menus overflowing the card) occlude it. A click on
  another monitor dismisses geometrically (`on_mouse_down_out`). Clicks outside
  the card can't be vetoed by the panel.
- **Inherent, documented:** while any popover is open Hyprland sends every click
  to it, so Waybar can't be clicked; clicking another bar module only dismisses,
  and a second click opens the new panel.

## 5. Command line

Sources: [Click → runtime contract][t16], [Author dev loop][t19],
[Install story][t20].

`App::run()` owns argv under the author's binary name. Authors who want commands
of their own handle them before calling `run`.

| Command | Behaviour |
|---|---|
| `toggle <panel> [args…]` | Sends the toggle to the program's daemon, spawning one if none answers (§6). Closes if the same panel with the same panel arguments is open; otherwise closes what is open and opens this one. |
| `close` | Closes the program's popover, if any. Succeeds silently when no daemon runs. |
| `quit` | Stops the program's daemon. Succeeds silently when no daemon runs. |
| `daemon [<panel> [args…]]` | Runs the daemon in the foreground, optionally opening a panel at once. This is what `toggle` spawns. |
| `preview <panel> [args…]` | Foreground, logs to the terminal, opens the panel at once and **exits when the popover closes**. Does not bind the program's socket, so it never displaces the daemon; still sends `close` to the other sockets before opening (§7). |
| no arguments, or `help` | Usage text: the commands, the registered panels, and one ready-to-paste `on-click` line per panel using the real absolute path from `current_exe()`, e.g. `"on-click": "/home/me/.cargo/bin/my-panels toggle hello"`. Printed to stdout, exit 0. |

An unknown command or panel name, a missing panel name, or arguments to
`close`/`quit`/`help` print the problem and the usage text to stderr and exit
with status 2. Panel names are checked before anything is sent to the daemon.

## 6. Daemon

Sources: [Click → runtime contract][t16], [Author dev loop][t19],
[Panel lifecycle][t8].

1. **Socket:** `$XDG_RUNTIME_DIR/waybar-gpui/<id>.sock`, directory created on
   demand. A daemon starting up removes its own stale socket. The same ID names
   the layer namespace and prefixes log lines (`<id>: …`).
2. **Spawn:** `toggle` finding no daemon (socket missing or refusing) spawns
   `current_exe() daemon <panel> [args…]` in its own process group, stdin null,
   so it survives Waybar reloads.
3. **Lifetime:** until `quit` or logout. GPUI runs with `QuitMode::Explicit`.
4. **Protocol:** a line protocol carrying the command and the panel arguments.
   The daemon answers each connection with its executable identity (6.5)
   before reading the command. Encoding: tab-separated words, documented in
   `crates/waybar_gpui/src/ipc.rs`.
5. **Stale binary:** at startup the daemon records its executable's identity
   (device + inode of `/proc/self/exe`) and reports it on the socket. `toggle`
   compares it with the file at its own `current_exe()`; on mismatch it sends
   `quit`, waits for the socket to go away, and spawns itself. This covers
   rebuilds and `cargo install` upgrades. Consequence (intended): a
   `target/debug` build and an installed copy with the same ID replace each
   other's daemon, so each click shows its own binary's panels.
6. **Logs:** a spawned daemon's stderr goes to
   `$XDG_STATE_HOME/waybar-gpui/<id>.log` (default `~/.local/state/…`),
   truncated at each daemon start. stderr *is* the log: runtime lines, the
   author's `eprintln!` or any logger they install, and panic messages all land
   there. `toggle`'s own errors stay on its stderr; `preview` logs to the
   terminal. No logging dependency in the SDK.
7. **Panics:** a panicking panel takes the daemon down; the next click spawns a
   fresh one. No `catch_unwind`.

## 7. Several programs

Source: [Several author programs at once][t14], [Click → runtime contract][t16].

- At most one popover is open across all programs. Before opening, a runtime
  (daemon or `preview`) sends `close` to every other `*.sock` in
  `$XDG_RUNTIME_DIR/waybar-gpui/`. Best effort: no lock, no waiting for replies.
  Sockets that refuse the connection are deleted.

## 8. The `components` feature

Sources: [gpui-component compatibility][t13], [GPUI exposure][t15],
[Panel API][t17].

- Opt-in Cargo feature. Adds `gpui-component =0.8.0` (directly, never the
  `gpui-kit` umbrella, which enables X11) and `gpui-kit-assets =0.8.0`.
  Costs ~110 crates and ~10 MB of binary.
- With it, the runtime calls `gpui_component::init`, registers
  `gpui-kit-assets` (`with_assets`) for widget icons, and copies `Theme` into
  gpui-component's theme (`ThemeConfig` fallbacks) at startup. Authors may
  adjust gpui-component's theme in code afterwards.
- The runtime does **not** wrap popovers in gpui-component's `Root`: on Linux it
  forces a 20 px window border and shadow onto layer surfaces, growing the
  surface past the output. So gpui-component dialogs and notifications are
  unsupported; menus and in-window popovers work.

## 9. Starter template

Source: [Starter template][t21].

- cargo-generate template in `template/`:
  `cargo generate --git https://github.com/Cerclique/waybar-gpui template --tag vX.Y.Z`.
  Copying `template/` and replacing the placeholders by hand also works (documented).
- **Prompts:** project name (built in); `id`, default the project name, validated
  `^[a-z0-9][a-z0-9-]*$`; `components`, bool, default false.
- **Contents:** `Cargo.toml` with `waybar_gpui = { git = "https://github.com/Cerclique/waybar-gpui", tag = "vX.Y.Z" }`
  (plus `features = ["components"]` if chosen); `main.rs` with
  `App::new("{{id}}").panel::<Hello>().run()` and a comment that the ID names the
  socket, log and Hyprland namespace and must stay stable; a `Hello` panel (text
  in `card()`, a close button calling `close(cx)`, a gpui-component `Button` when
  `components` is on); a README (§11); `.gitignore` with `target/`. No
  `Cargo.lock` (the README says to commit the generated one), no LICENSE, no dev
  script or Waybar config.

## 10. Install and configuration (author's users)

Source: [Install story][t20].

- Install: `cargo install --path . --locked` in the author's repo, into
  `~/.cargo/bin`. Reinstalling needs no reload (§6.5).
- Waybar started by Hyprland doesn't have `~/.cargo/bin` on its PATH, so the
  documented `on-click` uses the full path:
  `"on-click": "~/.cargo/bin/<program> toggle <panel> [args…]"`. Waybar 0.15.0
  runs `on-click` through `/bin/sh -c`, so `~` and `$HOME` expand (verified in
  the Waybar source, see the ticket). The bare name works if `~/.cargo/bin` is on
  Waybar's PATH.
- Hyprland needs nothing. `examples/hypr/` ships one optional rule for every
  program:

  ```lua
  hl.layer_rule({
    match = { namespace = "^waybar-gpui:" },
    blur = true,
    ignore_alpha = 0.3,
    animation = "fade",
  })
  ```

  Per program: match `^waybar-gpui:<id>$`.
- Notes for the docs: Waybar config edits need `pkill -SIGUSR2 waybar`; run
  `<program> quit` before `cargo uninstall`.

## 11. Documentation

Source: [Documentation][t22].

Each fact lives in exactly one layer; the others link to it.

| Layer | Content |
|---|---|
| `README.md` | What it is, one screenshot, a 5-minute quickstart (cargo-generate → `preview hello` → `cargo install` → `on-click`), links onward. |
| `docs/guide/` | Plain Markdown, `index.md` + one file per chapter, relative links, shaped to become mdBook later. Chapters: writing a panel, panel arguments, live data and state, theming and `card()`, components, closing and Esc, dev loop, installing, troubleshooting, limits, How it works (author level, one Mermaid diagram), Using a waybar-gpui program (for people who only install a program). |
| Rustdoc | Every public item documented; crate page with a compiling example. |
| `crates/example-panels` | Runnable patterns the guide links to instead of long code blocks. |
| `template/README.md` | Short quickstart: `preview hello`, pointing your own bar's `on-click` at `target/debug/<program>`, `cargo install --path . --locked`, the usage text, committing `Cargo.lock`, a link to "Using a waybar-gpui program". Links to the guide use tagged URLs (`…/blob/vX.Y.Z/docs/guide/…`). |
| `docs/architecture.md` | Contributors: socket protocol, Hyprland exclusive-layer routing, GPUI constraints, rejected designs and why; restates the research conclusions it relies on and links the `research/*` branch files. |
| `docs/adr/` | Initial ADRs: one surface with a drawn card instead of `xdg_popup`; a daemon per program; no gpui-component `Root`. |

- `docs/session-notes.md` is retired: its decisions and verified behaviour move
  to `docs/architecture.md`, TODOs stay in issues. `CLAUDE.md` then points to
  `GLOSSARY.md`, `docs/architecture.md` and the guide.
- The checklist of facts each layer must carry is in the [Documentation][t22]
  resolution.

## 12. CI and checks

Sources: [Starter template][t21], [Documentation][t22].

- `cargo build`, `cargo test`, `cargo clippy --all-targets -- -D warnings` (`make lint`).
- `#![deny(missing_docs)]` on `waybar_gpui`; `RUSTDOCFLAGS="-D warnings" cargo doc`
  and doctests, also in `make lint`.
- Template: generate both `components` variants and build them with a `[patch]`
  pointing the git dependency at the checkout; fail if the template's `tag`
  differs from `v` + `[workspace.package] version`.
- `lychee --offline` on Markdown relative links.
- Rule: a PR that changes behaviour updates the layer owning the fact.

## 13. Release v0.1.0

Gate, all required:

- Everything in §2–§10 built, with the live click tests passing on Hyprland
  (top and left bars, two monitors).
- §11 complete: README quickstart, full rustdoc, every guide chapter,
  `docs/architecture.md` + initial ADRs, template README, every checklist item
  placed. Later, not gating: mdBook/Pages, extra media, more examples.
- §12 green.

Order: rename the repo to `waybar-gpui`, tag `v0.1.0`, make the repo public.

## 14. Left to the builder

- ~~The socket line encoding and the shape of the daemon's identity reply
  (§6.4).~~ Settled: see the module doc of `crates/waybar_gpui/src/ipc.rs`.
- Exact `card()` styling values and any `Theme` fields beyond the prototype's.
- Wording of the usage text and log lines.

[map]: https://github.com/Cerclique/waybar_gpui_plugin_template/issues/7
[proto]: https://github.com/Cerclique/waybar_gpui_plugin_template/tree/prototype/panel-api
[t8]: https://github.com/Cerclique/waybar_gpui_plugin_template/issues/8
[t9]: https://github.com/Cerclique/waybar_gpui_plugin_template/issues/9
[t10]: https://github.com/Cerclique/waybar_gpui_plugin_template/issues/10
[t11]: https://github.com/Cerclique/waybar_gpui_plugin_template/issues/11
[t12]: https://github.com/Cerclique/waybar_gpui_plugin_template/issues/12
[t13]: https://github.com/Cerclique/waybar_gpui_plugin_template/issues/13
[t14]: https://github.com/Cerclique/waybar_gpui_plugin_template/issues/14
[t15]: https://github.com/Cerclique/waybar_gpui_plugin_template/issues/15
[t16]: https://github.com/Cerclique/waybar_gpui_plugin_template/issues/16
[t17]: https://github.com/Cerclique/waybar_gpui_plugin_template/issues/17
[t18]: https://github.com/Cerclique/waybar_gpui_plugin_template/issues/18
[t19]: https://github.com/Cerclique/waybar_gpui_plugin_template/issues/19
[t20]: https://github.com/Cerclique/waybar_gpui_plugin_template/issues/20
[t21]: https://github.com/Cerclique/waybar_gpui_plugin_template/issues/21
[t22]: https://github.com/Cerclique/waybar_gpui_plugin_template/issues/22
