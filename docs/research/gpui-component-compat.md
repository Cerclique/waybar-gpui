# gpui-component compatibility with gpui-pre 0.3.9

Research date: 2026-10-24. Ticket: [#13](https://github.com/Cerclique/waybar-gpui/issues/13), part of map [#7](https://github.com/Cerclique/waybar-gpui/issues/7). Local toolchain: rustc 1.97.1. The repo pins `gpui = { package = "gpui-pre", version = "=0.3.9" }` and `gpui_platform = { package = "gpui-pre-platform", version = "=0.3.9", features = ["wayland"] }`. That snapshot is `zed@93d7456`; see [gpui-popup-windows.md](gpui-popup-windows.md).

## TL;DR

- **Use `gpui-component = "=0.8.0"`.** It is the latest release (2026-10-19), and 0.7.3 also works. Both pin `gpui-pre =0.3.9` exactly, the same snapshot we use, so Cargo resolves a single `gpui-pre` and no `[patch]` is needed.
- **Nothing else changes in Cargo.toml.** Keep our `gpui` / `gpui_platform` aliases as they are, with no renames, patches or feature flags. gpui-component has no default features and doesn't depend on `gpui-pre-platform`, so our wayland-only feature set stays wayland-only (no X11 crates show up).
- **Rust 1.97.1 is enough.** No `rust-version` is declared and gpui-kit has no `rust-toolchain.toml` at v0.8.0. A throwaway crate using `Root`, `Button`, `cx.theme()` and a `WindowKind::LayerShell` window passed `cargo check` and `cargo build --release` on 1.97.1.
- **Code changes are small.** Call `gpui_component::init(cx)` once at startup, and make each window's root view a `gpui_component::Root` wrapping our view. `Root` hosts dialogs, notifications and the focus/keyboard infrastructure.
- **It costs size and dependencies.** The resolved graph goes from 557 to 667 unique crates (adds markdown, html5ever, lsp-types, resvg, rust-i18n, …). A stripped thin-LTO release of the trivial check binary is **36 MB**, against **26 MB** for our current `waybar-popup`.
- **Pins move together.** Upgrading means bumping `gpui-pre`, `gpui-pre-platform` and `gpui-component` in one step to a matching triple. A new gpui-component release usually requires the next `gpui-pre` snapshot. gpui-kit releases about weekly, together with the snapshot.
- **What other gpui-pre users do:**
  - Most published crates build on gpui-component / gpui-kit (zedis-ui, gpui-ai, native-theme-gpui, gpui-storybook) or fork it (gpui-macos-component).
  - A few ship their own component libraries (woocraft, herogpui-components).
  - The closest precedent, gpui-shell, uses zed git `main` (not gpui-pre) and keeps its own in-repo `ui` crate.

## 1. Which versions pin which snapshot

From the crates.io API (`/api/v1/crates/gpui-component/<ver>/dependencies`):

| gpui-component | released | gpui dependency |
|---|---|---|
| **0.8.0** | 2026-10-19 | `gpui-pre =0.3.9`, `gpui-pre-macros =0.3.9`, `gpui-pre-sum-tree =0.3.9` |
| 0.7.3 | 2026-10-12 | `gpui-pre =0.3.9` (same set) |
| 0.7.2 / 0.7.1 | 2026-10-12 / 10-05 | `=0.3.8` |
| 0.7.0 | 2026-09-28 | `=0.3.7` |
| 0.6.6 | 2026-09-21 | `=0.3.6` |
| 0.6.0 | 2026-09-03 | `^0.3.1` (caret, see below) |
| ≤ 0.5.1 | ≤ 2026-02-05 | official `gpui ^0.2.2` (stale crates.io gpui, no layer shell) |

- `gpui-pre` 0.3.9 (2026-10-12) is still the newest snapshot. gpui-kit `main` also pins `=0.3.9` ([Cargo.toml L69](https://github.com/longbridge/gpui-kit/blob/main/Cargo.toml)).
- The exact-pin policy is deliberate. The comment in [gpui-kit v0.8.0 Cargo.toml L62-70](https://github.com/longbridge/gpui-kit/blob/v0.8.0/Cargo.toml) says: "a caret requirement let the weekly release move applications onto a newer snapshot that gpui-component did not compile with (#3156)". CI enforces exact pins.
- The repo moved from `longbridge/gpui-component` to [`longbridge/gpui-kit`](https://github.com/longbridge/gpui-kit), and the old URL redirects. The project is now layered:
  - `gpui-base`: headless behaviour plus `Root`.
  - `gpui-component`: the styled widgets, 60+ components such as button, input, select, list, popover, menu, dialog, switch, slider, tooltip and table.
  - `gpui-kit-assets`: Lucide icons.
  - `gpui-kit`: an umbrella crate.

  gpui-component 0.8.0 pulls in `gpui-base ^0.8.0`, `gpui-component-macros ^0.8.0` and `gpui-kit-assets ^0.8.0`. All of them pin `gpui-pre =0.3.9`.

## 2. What using it requires

### Cargo

Add one line next to the existing pins:

```toml
gpui-component = "=0.8.0"   # must match the gpui-pre snapshot above; bump together
```

- **Crate names:** gpui-component refers to gpui under the dependency key `gpui` (`gpui = { package = "gpui-pre", … }`), and the gpui-pre lib name is already `gpui`. Its types are therefore the same `gpui::*` types our code uses. `cargo tree -i gpui-pre` shows exactly one `gpui-pre v0.3.9`, shared by our crate, `gpui-base`, `gpui-component`, `gpui-kit-assets` and the platform crates.
- **Features:** gpui-component has no `default` feature. The optional ones are `gpui-fast`, `inspector`, `decimal`, `tree-sitter*`, `syntect`, `speech` and `test-support`; we need none of them. Its `gpui-pre-platform` dependency is dev-only, so our `features = ["wayland"]` holds and no `x11`/`xcb` crates appear in the tree.
- **Don't use the `gpui-kit` umbrella crate.** Its README recommends it, but it adds `gpui-pre-platform` with `["font-kit", "x11", "wayland", "runtime_shaders"]` and `gpui-pre-web` (from crates.io dependency metadata for gpui-kit 0.8.0). Depending on `gpui-component` directly keeps our platform features.
- **`gpui-fast`:** an optional alternative core published by the same maintainers ([longbridge/gpui-fast](https://github.com/longbridge/gpui-fast)), selected by a feature. It's irrelevant unless we choose to switch.

### Rust version

- No `rust-version` is declared in gpui-component's manifest (crates.io reports `rust_version: null` for every release), and [`rust-toolchain.toml`](https://github.com/longbridge/gpui-kit/blob/v0.8.0/rust-toolchain.toml) returns 404 at v0.8.0. The crate uses edition 2024.
- The compile check below succeeded on **rustc 1.97.1**.

### Code

- Call `gpui_component::init(cx)` before using any component ([`src/lib.rs`](https://docs.rs/crate/gpui-component/0.8.0/source/src/lib.rs), `pub fn init`). It registers the theme, global state, root, inputs, popovers, menus, notifications and tooltips.
- Make the root view of each window a `Root`: `cx.new(|cx| Root::new(view, window, cx))`. `Root` comes from `gpui_base::Root` and is re-exported as `gpui_component::Root` ([gpui-base `src/root.rs` L170](https://docs.rs/crate/gpui-base/0.8.0/source/src/root.rs)).
  - The README's `gpui_kit::open_window` helper "always mounts a Base `Root`". We open layer-shell windows with `cx.open_window` ourselves, so we wrap the view by hand, which works.
  - In our daemon, wrap the `Overlay` view in `crates/popup/src/daemon.rs`.
- Read colours with `use gpui_component::ActiveTheme as _; cx.theme().background`, and so on.
- **Glob-import gotcha:** `use gpui::{layer_shell::*, *}` is an error. Both export an `Anchor` (`layer_shell::Anchor` versus `gpui::Anchor`), and `ambiguous_glob_imports` is deny-by-default on 1.97. Import `layer_shell::{Anchor, Layer, LayerShellOptions}` explicitly. Our `daemon.rs` already uses `layer_shell::*` together with a non-glob `gpui::{…}` list, so it's fine.

### Compile check (throwaway, outside the repo)

The scratch crate depended on `gpui-pre =0.3.9`, `gpui-pre-platform =0.3.9` (`wayland`) and `gpui-component =0.8.0`. Its `main` called `gpui_component::init` and opened a `WindowKind::LayerShell` window whose root was `Root::new(cx.new(|_| Panel), …)`. `Panel` rendered `Button::new("ok").primary().label("OK").on_click(…)` on `cx.theme().background`.

- `cargo check` passed, and `cargo build --release` (with `lto = "thin"` and `strip = true`, like ours) took 43 s on a warm registry.
- Unique crates in the graph: 667 with gpui-component, against 557 for `crates/popup` today.
- Binary size: 36,373,192 bytes, against 26,457,616 for the current `target/release/waybar-popup`.
- Not checked: runtime behaviour. Nothing ran on screen, so we haven't confirmed how the theme looks on a transparent layer surface, or whether in-window popovers and menus behave inside our full-output overlay.

## 3. What other gpui-pre projects do for widgets

The crates.io reverse dependencies of `gpui-pre` (66 crates, 2026-10-24) fall into four groups:

- **Build on gpui-component / gpui-kit:**
  - `zedis-ui` 0.13.0 (`gpui-kit ^0.7.1`)
  - `gpui-ai` 0.8.0 (`gpui-component ^0.6.0`)
  - `native-theme-gpui` 0.6.1 (`gpui-component ^0.7.1`)
  - `gpui-storybook-core` 0.7.2 (`gpui-kit ^0.8.0`, pins `gpui-pre =0.3.9`)

  This is the majority pattern for apps that want ready-made widgets.
- **Fork it:** `gpui-macos-component` ("The gpui-macos pinned fork of gpui-component", pinned to `=0.3.8`).
- **Own component library:**
  - `woocraft` 0.6.3 ("GPUI components lib for Woocraft design system", pins `=0.3.9`)
  - `herogpui-components` 0.14.0 (HeroUI-style, pins `=0.3.5`)
  - `wk-ui` ("headless UI primitives for Kumizo shells", `=0.3.8`)
- **Closest precedent to us, a Wayland shell:** [andre-brandao/gpui-shell](https://github.com/andre-brandao/gpui-shell) doesn't use gpui-pre at all. It depends on zed git `main` ([Cargo.toml L8-9](https://github.com/andre-brandao/gpui-shell/blob/main/Cargo.toml)) and keeps an in-repo `crates/ui` (`components/button/{button,icon_button,split_button,toggle_button}.rs`, …) shaped like Zed's own `ui` crate. A zed git dependency can't use gpui-component, which is pinned to gpui-pre crates; you'd have to `[patch]` every `gpui-pre*` crate to the git source, which is fragile.

Note: several of these crates use caret requirements such as `gpui-pre ^0.3.6`. Under 0.x semver, `^0.3.6` means `>=0.3.6, <0.4.0`, so it also resolves to 0.3.9. The upstream #3156 incident is exactly why gpui-kit stopped doing this.

## Recommendation for this template

If the popover needs real widgets (buttons, toggles, sliders, lists, inputs), add `gpui-component = "=0.8.0"` directly, not the `gpui-kit` umbrella, and document that the three pins move together. This works today with zero build friction. The cost is about 10 MB of binary and roughly 110 more crates. For a template that only shows a card with text and icons, hand-rolled `div()` styling (what we have now) stays lighter. A middle ground is to make gpui-component an opt-in Cargo feature of `crates/popup`.

## Sources

- crates.io API: `/api/v1/crates/{gpui-component,gpui-base,gpui-kit,gpui-fast,gpui-pre}` and `/<version>/dependencies` and `/reverse_dependencies`, fetched 2026-10-24.
- gpui-component 0.8.0 crate tarball (`Cargo.toml.orig`, `README.md`, `src/lib.rs`), and the gpui-base 0.8.0 tarball (`src/root.rs`).
- [longbridge/gpui-kit v0.8.0 Cargo.toml](https://github.com/longbridge/gpui-kit/blob/v0.8.0/Cargo.toml) and [main Cargo.toml](https://github.com/longbridge/gpui-kit/blob/main/Cargo.toml).
- [andre-brandao/gpui-shell Cargo.toml](https://github.com/andre-brandao/gpui-shell/blob/main/Cargo.toml) and its `crates/ui` tree (GitHub API).
- Local `cargo check`, `cargo build --release` and `cargo tree` in a scratch crate (rustc 1.97.1).
