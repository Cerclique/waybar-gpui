---
name: waybar-gpui
description: Writing, changing or debugging a waybar-gpui panel, or a program built on the `waybar_gpui` crate (GPUI popovers opened from Waybar bar modules on Hyprland). Use whenever the project depends on `waybar_gpui`.
---

# waybar-gpui panels

This project is a **program** built on the `waybar_gpui` SDK: it registers
**panels** (GPUI views), and a click on a Waybar **bar module** opens one as a
**popover**. GPUI is pre-1.0 and the SDK pins one exact snapshot of it, so
your memory of either API is unreliable. This skill holds no API facts; it
tells you where the facts for *this* project's version live. Where anything
here contradicts that version's guide, the guide wins.

## 1. Find the version

Before writing code, run in the project:

```sh
cargo pkgid waybar_gpui
cargo metadata --format-version 1 |
  grep -oE '[^"]*/(waybar_gpui(-[0-9.]+)?|gpui-pre-[0-9.]+)/Cargo.toml'
```

The first line gives the SDK's **ref**, the git ref its guide is read at:

- `registry+…#waybar_gpui@X.Y.Z` (from crates.io): the ref is `vX.Y.Z`.
- `git+…?branch=B#…`, `?tag=T#…` or `?rev=R#…` (a git dependency): the ref
  is `B`, `T` or `R`. Ignore the `@X.Y.Z` at the end: an unreleased branch
  still carries the last release's version.

The second prints the `Cargo.toml` of the SDK and of GPUI (`gpui-pre`) that
this project actually compiles; their directories are the **sources** below.
Also check whether the `waybar_gpui` dependency in the project's `Cargo.toml`
turns on the `components` feature (gpui-component widgets). Done when you have
the ref, both source directories, and `components` on or off.

## 2. Where the facts live

- **The guide**, at the ref:
  `https://raw.githubusercontent.com/Cerclique/waybar-gpui/<ref>/docs/guide/<page>.md`.
  Start at `docs/guide/index.md`; terms are in `docs/GLOSSARY.md`. A relative
  link in a page resolves against that page's URL. A code block holding
  `{{#include snippets/src/<file>.rs:<name>}}` stands for the lines between
  `// ANCHOR: <name>` and `// ANCHOR_END: <name>` in that file, which sits
  next to the page (`docs/guide/snippets/src/<file>.rs`, at the same ref).
  The guide's example panels are at `https://raw.githubusercontent.com/Cerclique/waybar-gpui/<ref>/examples/example-panels/src/<file>.rs`;
  `main.rs` there lists which panel shows which pattern. The same guide is
  a site at `https://cerclique.github.io/waybar-gpui/` (the latest release;
  `/dev/` for `main`): link the user there, but read the raw pages at the ref.
- **The SDK's source and rustdoc**: `src/` in its source directory. Read
  `lib.rs` first: its docs list the public API.
- **GPUI's source**: its source directory. Read the definition of every GPUI
  type, method or macro before you use it. The SDK re-exports GPUI as
  `waybar_gpui::gpui`; use that path, and leave `gpui-pre` out of the
  project's `Cargo.toml`. The pin moves only with an SDK release.

Reach for the guide page that owns the task:

| Task | Page |
| --- | --- |
| The `Panel` trait, `Program`, sizing, GPUI basics | `docs/guide/writing-a-panel.md` |
| One panel behind several bar modules | `docs/guide/panel-arguments.md` |
| Timers, event streams, state kept across opens | `docs/guide/live-data-and-state.md` |
| Colours, `Theme`, `card()` | `docs/guide/theming.md` |
| gpui-component widgets | `docs/guide/components.md` |
| Closing, Esc | `docs/guide/closing-and-esc.md` |
| Running, logs, panics | `docs/guide/dev-loop.md` |
| Waybar and Hyprland config, installing | `docs/guide/installing.md` |
| Something misbehaves | `docs/guide/troubleshooting.md` |

Read `docs/guide/limits.md` before promising a feature: the runtime fixes
placement, allows one popover at a time, and lets panels open no other panels.

## 3. Words

Speak the glossary: a **panel** is the view the author writes, a **bar
module** is the Waybar item, a **popover** is the panel on screen, the
**card** is its visible rectangle, the **program** is the binary and its
**program ID** is the string given to `Program::new`. A Waybar entry is a "bar
module", in full, every time.

## 4. Adding a panel

1. Write a struct implementing `Panel` (a unique `NAME`, `new` taking the
   panel arguments) and GPUI's `Render`, in its own module.
2. Register it in `main` with `.panel::<NewPanel>()`.
3. `cargo build`, then `cargo clippy --all-targets`, clean.
4. Run it with `cargo run -- preview <NAME>`, after telling the user it opens
   a popover at their cursor (step 6).
5. Give the user the Waybar bar module for it: run the program with no
   arguments, which prints the ready-to-paste `on-click` line per panel, and
   wrap it as in `docs/guide/installing.md`.

Done when the panel builds clean, the user has seen it in `preview`, and they
have its bar module.

## 5. The user's desktop

Waybar and Hyprland versions vary per machine, and their docs describe other
versions: the Waybar wiki documents master, and Hyprland's config moved from
hyprlang to Lua. Before editing either config, run `waybar --version` and
`hyprctl version`, find which Hyprland config file is in use
(`~/.config/hypr/hyprland.lua` or `hyprland.conf`), and write syntax for
those versions. Waybar reloads with `pkill -SIGUSR2 waybar`.

## 6. Commands that draw on screen

`preview`, `toggle` and `daemon` open popovers on the user's screen, and
while one is open every click on that monitor goes to it. Announce each one
before running it, and leave closing it to the user (Esc). `close` and `quit`
only close things. `preview` logs to the terminal; a daemon started by a click
logs to `~/.local/state/waybar-gpui/<program ID>.log`
(`$XDG_STATE_HOME/waybar-gpui/` when set).
