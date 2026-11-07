# Depending on the gpui-kit umbrella instead of gpui-pre

Research date: 2026-11-07. Follows [gpui-component-compat.md](gpui-component-compat.md) (2026-10-24), which rejected the umbrella. Today the repo pins `gpui-pre`/`gpui-pre-platform =0.3.9` (`wayland` only), plus `gpui-component`/`gpui-kit-assets =0.8.0` behind the `components` feature, and re-exports `gpui` and `gpui_component as components`. Local toolchain: rustc 1.97.1.

## TL;DR

- **The switch is mechanical.** A scratch copy of `main` switched to `gpui = { package = "gpui-kit", version = "=0.9.0", default-features = false }` builds, passes `cargo test --all-features`, and passes `make lint`. The changes were: the dependency lines, `components` mapped to `gpui/component` + `gpui/assets`, three paths (`gpui_platform::` → `gpui::platform::`, `gpui_component::` → `gpui::component::`, `gpui_kit_assets::` → `gpui::assets::`), and one fix needed by any move to 0.9.0 (`ThemeConfig` is now `#[non_exhaustive]`, so it can't be built with a struct literal).
- **The rename is what makes it work.** gpui-kit itself recommends aliasing the kit as `gpui`. The key `gpui` keeps every in-crate `gpui::…` path and `pub use gpui;` unchanged. Authors' `use waybar_gpui::gpui;` plus the derives keep working: `example-panels` compiled unchanged.
- **X11 can't be turned off.** gpui-kit enables `gpui-pre-platform` with `font-kit, x11, wayland, runtime_shaders`, and features are additive. On Linux, `font-kit` and `runtime_shaders` do nothing (macOS/iOS only). `x11` pulls in `x11rb` and links **libxcb, libxcb-xkb, libxkbcommon-x11** into every author's binary.
- **Any move also means a GPUI bump.** gpui-kit 0.9.0 pins `gpui-pre =0.3.11`. No umbrella release pins 0.3.9 *and* matches gpui-component 0.8.0 (gpui-kit 0.8.0 does pin 0.3.9). So the switch is a breaking minor release in any case, which `architecture.md` already requires for a pin change.
- **The umbrella's own cost is small, and it's paid without `components` too.** gpui-base is non-optional, so authors who don't use widgets still pull it in.

## Measurements (`example-panels`, release, thin LTO, stripped)

| variant | crates (`components`) | crates (none) | binary (`components`) | X11 libs linked |
|---|---|---|---|---|
| `main`: gpui-pre 0.3.9 + gpui-component 0.8.0 | 488 | 410 | 37.1 MB | no |
| direct bump: gpui-pre 0.3.11 + gpui-component 0.9.0 | 485 | 407 | 37.7 MB | no |
| umbrella: gpui-kit 0.9.0 | 500 | **448** | 38.8 MB | **yes** |

Crate counts are unique normal-edge packages from `cargo tree -e normal`.

## What the umbrella is (0.9.0, 2026-11-02)

- Released weekly, in lockstep with gpui-base, gpui-component and gpui-kit-assets. All of them pin the gpui-pre snapshot exactly: 0.8.0 → `=0.3.9`, 0.8.1 → `=0.3.10`, 0.9.0 → `=0.3.11`.
- `src/lib.rs` is a facade. It provides `pub use ::gpui::*`, `base` (gpui-base, always on), `component` (feature `component`, default), `assets` (feature `assets`, default), `platform` (gpui-pre-platform), a hidden `pub use crate as gpui`, an `actions!` macro that doesn't need `gpui` in scope, `init`, and `open_window`. `open_window` always mounts `gpui_base::Root`, which [ADR 0003](../adr/0003-no-gpui-component-root.md) rules out for layer surfaces.
- Its "public facade decision" (2026-09-08, a comment in `lib.rs`) says that applications should import only `gpui_kit`, and that a future move to official GPUI crates would be "an internal dependency migration" with no import changes for users. That's the main argument for the umbrella: it absorbs upstream crate churn.
- The `gpui-fast` feature switches the core, the platform and every layer to [gpui-fast](https://github.com/longbridge/gpui-fast), Longbridge's GPUI fork with "rendering and layout optimizations" (0.1.13, lib name `gpui`, has `platform/layer_shell.rs`). Our crate could forward that feature to authors. It's untested here.

## How derive macros behave

- gpui-pre-macros (0.3.9 and 0.3.11) uses `proc_macro_crate::crate_name("gpui-kit")` to rewrite emitted `gpui::`/`gpui_platform::` paths to the kit's name in the crate where the macro expands.
- In `waybar_gpui` the dependency is named `gpui`, so the paths come out as `gpui::…` and resolve.
- In an author's crate that doesn't list gpui-kit, nothing is rewritten. The paths stay `gpui::…`, and `use waybar_gpui::gpui;` resolves them through the kit's glob. That's the same rule as today.
- An author who also lists gpui-kit gets paths rewritten to `gpui_kit::…`. Those resolve as well.

## What the GPUI 0.3.9 → 0.3.11 bump touches

- `platform/layer_shell.rs` is unchanged.
- `gpui-pre-linux`: Wayland `window.rs`/`client.rs` gain `ext-background-effect` blur, client corner radii, and an activation-token rework. The activation change is relevant to the focus behaviour in `architecture.md`, so it needs an on-screen check. A new `power.rs` is also added.
- The 54 `gpui-pre/src` files that differ were not reviewed one by one. The SDK compiled without changes.

## Side check: today's 0.1.0 still resolves

gpui-component 0.8.0 requires `gpui-base ^0.8.0` and `gpui-component-macros ^0.8.0`. Now that 0.8.1 is out (it pins `=0.3.10`), a fresh author lockfile on `waybar_gpui =0.1.0` with `components` resolves to gpui-base 0.8.0 and gpui-pre 0.3.9 by backtracking, and gpui-component-macros goes to 0.8.1, which is fine. No action is needed.

## Not checked

- Runtime behaviour on screen (`make dev`) under 0.3.11: focus, input region, and the theme on the transparent surface.
- MSRV under 0.3.11 (the current 1.95 floor comes from 0.3.9's `std::hint::cold_path`).
- semver-checks output and how docs.rs renders a re-exported facade.

## Sources

- crates.io API: `/api/v1/crates/{gpui-kit,gpui-component,gpui-base,gpui-kit-assets,gpui-fast,gpui-pre}`, `/versions`, and `/<ver>/dependencies`, fetched 2026-11-07.
- Crate tarballs: gpui-kit 0.9.0 (`Cargo.toml.orig`, `src/lib.rs`), gpui-fast 0.1.13, gpui-fast-platform 0.1.13, gpui-pre 0.3.11, gpui-pre-linux 0.3.11, gpui-pre-macros 0.3.11 (`src/gpui_pre_facade_paths.rs`).
- Scratch builds of `main` in three variants, with `cargo build --release`, `cargo tree`, `ldd`, `cargo test --all-features` and `make lint` (rustc 1.97.1).
