# GPUI for Waybar popup windows on Hyprland

Research date: 2026-10-24. Target system: Arch Linux, Hyprland 0.56.2 (Lua config), Waybar 0.15.0, rustc/cargo 1.97.1, NVIDIA proprietary driver (`nvidia-utils 615.78.08`), `vulkan-icd-loader 1.4.363`, `libxkbcommon 1.13.2`, `wayland 1.26.0`, `fontconfig 2.18.3` (checked locally with `pacman -Q`).

Zed sources were read at commit [`347f171a3f`](https://github.com/zed-industries/zed/tree/347f171a3fd00a3b672382d78348b37bd969d17c) (main, 2026-10-24). Line-anchored links below point to that commit and are shortened as `zed@347f171`.

## TL;DR

- **It works, and it's a better fit than expected.** GPUI on Zed `main` has native **wlr-layer-shell** support: `WindowKind::LayerShell(LayerShellOptions { layer, anchor, margin, exclusive_zone, keyboard_interactivity, namespace })` ([platform.rs L2820-2823](https://github.com/zed-industries/zed/blob/347f171a3fd00a3b672382d78348b37bd969d17c/crates/gpui/src/platform.rs#L2820-L2823), [layer_shell.rs](https://github.com/zed-industries/zed/blob/347f171a3fd00a3b672382d78348b37bd969d17c/crates/gpui/src/platform/layer_shell.rs)). It also has native **xdg_popup** windows (`WindowKind::AnchoredPopup`), which can be parented to a layer surface. Layer shell landed in [PR #35610](https://github.com/zed-industries/zed/pull/35610) on 2025-10-29, and anchored popups in [PR #60232](https://github.com/zed-industries/zed/pull/60232) on 2026-07-08. A popup anchored to the bar edge needs no Hyprland window rules.
- **Distribution is the main risk.** The official `gpui` crate on crates.io is **0.2.2 (2025-10-22)**. It has been frozen for a year, predates layer-shell support, and still uses the old `blade` renderer. `gpui_platform` isn't published by Zed at all. You have two realistic options:
  1. A **git dependency on zed `main`**, pinned to a `rev`. This is what [gpui-shell](https://github.com/andre-brandao/gpui-shell/blob/main/Cargo.toml) does.
  2. The community **`gpui-pre` snapshot crates**. These are published by the gpui-component maintainer (huacnlee), are weekly snapshots of zed (0.3.9 = `zed@93d7456`, 2026-10-12), and are what gpui-component 0.8.0 pins to.
  Either way the API is pre-1.0 and breaks often, as the [README](https://github.com/zed-industries/zed/blob/347f171a3fd00a3b672382d78348b37bd969d17c/crates/gpui/README.md) says.
- **Rendering needs Vulkan.** It goes through wgpu (Vulkan + GLES backends on Linux, [gpui_wgpu/Cargo.toml L44-45](https://github.com/zed-industries/zed/blob/347f171a3fd00a3b672382d78348b37bd969d17c/crates/gpui_wgpu/Cargo.toml#L44-L45)). Your NVIDIA plus Vulkan loader setup is fine; Zed 1.25.3 is already installed and running on it.
- **Toolchain:** Zed `main` pins Rust **1.98.1** ([rust-toolchain.toml](https://github.com/zed-industries/zed/blob/347f171a3fd00a3b672382d78348b37bd969d17c/rust-toolchain.toml)), but you have 1.97.1. Expect to need `rustup update` or a newer Arch `rust` package for a `main` git dependency.
- **There's precedent:** [andre-brandao/gpui-shell](https://github.com/andre-brandao/gpui-shell) is a Wayland bar, launcher, control center and OSD for Hyprland/Niri built entirely on GPUI layer surfaces. It is active (last push 2026-10-24) but early and self-described as heavily AI-generated. Its `toggle_panel` is exactly the "click module → popup → click again closes" pattern ([panel.rs](https://github.com/andre-brandao/gpui-shell/blob/main/crates/app/src/panel.rs)).
- **Unknowns:** I found no published data on binary size or cold-start time. Each popup costs a wgpu/Vulkan device init, so measure before committing to a "spawn per click" design.
- **Verdict:** GPUI is **feasible but high-maintenance**. For a *template* repo aimed at other people, **gtk4-rs + gtk4-layer-shell (or relm4)** is the low-risk default: it's in Arch repos (you already have `gtk4-layer-shell 1.3.0`), uses a crates.io-stable API, and has CSS styling that matches Waybar. Offer GPUI as an optional/experimental flavour pinned to an exact `gpui-pre` version or zed `rev`, with a **long-running daemon + Unix socket** (toggle via `on-click`) rather than spawning a process per click.

## 1. Using GPUI standalone, outside Zed

### Distribution and versioning

| Source | Version / date | Layer shell? | Renderer | Notes |
|---|---|---|---|---|
| crates.io [`gpui`](https://crates.io/crates/gpui) | 0.2.2, 2025-10-22 (last release) | **No**: `WindowKind` only has `Normal`, `PopUp`, `Floating` (checked in the 0.2.2 tarball, `src/platform.rs` L1262-1272) | blade-graphics | Published by Zed; stale |
| crates.io `gpui_platform` | not published | n/a | n/a | Required on `main` since gpui_platform was extracted ([PR #49277](https://github.com/zed-industries/zed/pull/49277), 2026-02-19) |
| git `zed-industries/zed` `main` | `gpui` 0.2.2 in Cargo.toml, but a different API | Yes, plus `AnchoredPopup` | wgpu (git rev `3b3506ea`, [workspace Cargo.toml L1003](https://github.com/zed-industries/zed/blob/347f171a3fd00a3b672382d78348b37bd969d17c/Cargo.toml#L1003)) | Pulls the zed repo; wgpu is itself a git dep, which is probably why crates.io publishing stalled |
| crates.io [`gpui-pre`](https://crates.io/crates/gpui-pre) / `gpui-pre-platform` / `-linux` / `-wgpu` | 0.3.9, 2026-10-12 (`zed@93d7456`); weekly since 2026-09-03 | Yes (verified in the 0.3.9 tarball: `WindowKind::LayerShell`, `AnchoredPopup`; `examples/layer_shell.rs` identical to `main`) | wgpu 29.0 from crates.io | Community snapshot published by `huacnlee`. The lib names stay `gpui`/`gpui_platform`. Some files carry "Modified for gpui-pre" patches (e.g. `src/action.rs` L1) |

gpui-component pins `gpui = { package = "gpui-pre", version = "=0.3.9" }` and documents why it pins exactly: "any snapshot may change GPUI's API… a caret requirement let the weekly release move applications onto a newer snapshot that gpui-component did not compile with (#3156)" ([gpui-kit Cargo.toml L62-70](https://github.com/longbridge/gpui-kit/blob/main/Cargo.toml)).

- **License:** Apache-2.0 ([crates/gpui/Cargo.toml](https://github.com/zed-industries/zed/blob/347f171a3fd00a3b672382d78348b37bd969d17c/crates/gpui/Cargo.toml)). gpui-component is also Apache-2.0.
- **Stability:** "still pre-1.0. There will often be breaking changes between versions. You'll also need to use the latest version of stable Rust" ([README](https://github.com/zed-industries/zed/blob/347f171a3fd00a3b672382d78348b37bd969d17c/crates/gpui/README.md)). The README's own instruction (`gpui_platform = { version = "*" }`) doesn't resolve on crates.io, which shows how the docs lag the code.
- **Docs:** the [README](https://github.com/zed-industries/zed/blob/347f171a3fd00a3b672382d78348b37bd969d17c/crates/gpui/README.md), [gpui.rs](https://gpui.rs), [docs.rs/gpui](https://docs.rs/gpui) (for stale 0.2.2), and the ~40 examples in [crates/gpui/examples](https://github.com/zed-industries/zed/tree/347f171a3fd00a3b672382d78348b37bd969d17c/crates/gpui/examples), including `layer_shell.rs`, `popover.rs` and `window_positioning.rs`. In practice the source is the documentation.

### Linux build and runtime requirements

- **Renderer:** wgpu with `vulkan` and `gles` features on Linux ([gpui_wgpu/Cargo.toml L44-45](https://github.com/zed-industries/zed/blob/347f171a3fd00a3b672382d78348b37bd969d17c/crates/gpui_wgpu/Cargo.toml#L44-L45)). Zed's Linux docs say "we use Vulkan to communicate with your GPU" and list `NoSupportedDeviceFound` / `vkcube` troubleshooting ([docs/src/linux.md L143-167](https://github.com/zed-industries/zed/blob/347f171a3fd00a3b672382d78348b37bd969d17c/docs/src/linux.md#L143-L167)). Locally you have `vulkan-icd-loader`, `nvidia-utils` and `vulkan-tools`, so this is OK.
- **Wayland stack:** `wayland-client 0.31`, `wayland-protocols`, `wayland-protocols-wlr`, `xkbcommon` (with the `wayland` feature), `calloop-wayland-source`, and `ashpd` for portals ([gpui_linux/Cargo.toml L18-33, L85-110](https://github.com/zed-industries/zed/blob/347f171a3fd00a3b672382d78348b37bd969d17c/crates/gpui_linux/Cargo.toml)).
- **Text:** cosmic-text 0.19 plus font-kit/fontconfig ([gpui_wgpu/Cargo.toml](https://github.com/zed-industries/zed/blob/347f171a3fd00a3b672382d78348b37bd969d17c/crates/gpui_wgpu/Cargo.toml)).
- **Arch build deps:** Zed's `script/linux` lists (for building all of Zed, a superset) `gcc clang lld cmake alsa-lib fontconfig glib2 libva wayland libgit2 libxcb libxkbcommon-x11 llvm openssl zstd pkgconf sqlite pipewire …` ([script/linux L206-232](https://github.com/zed-industries/zed/blob/347f171a3fd00a3b672382d78348b37bd969d17c/script/linux#L206-L232)). A GPUI-only app needs a subset (wayland, libxkbcommon, fontconfig/freetype, a Vulkan loader, pkgconf, clang/cmake for some -sys crates).
- **Rust:** no `rust-version` is declared, but zed `main` pins **1.98.1** ([rust-toolchain.toml](https://github.com/zed-industries/zed/blob/347f171a3fd00a3b672382d78348b37bd969d17c/rust-toolchain.toml)), and gpui-shell builds on nightly ([rust-toolchain.toml](https://github.com/andre-brandao/gpui-shell/blob/main/rust-toolchain.toml)). Local cargo is 1.97.1.
- **Binary size and startup:** I found no published measurements for GPUI. The dependency graph is large (wgpu + naga + cosmic-text + accesskit + ashpd/zbus + tokio-free async executor), so expect a multi-MB release binary and a cold start dominated by Vulkan instance/device creation and font loading. **Measure it** (`hyperfine`, `cargo bloat`) before choosing per-click spawning.

## 2. Wayland specifics

### Layer shell: yes, natively

- **API:** `LayerShellOptions { namespace, layer: Layer::{Background,Bottom,Top,Overlay}, anchor: Anchor (TOP|BOTTOM|LEFT|RIGHT bitflags), exclusive_zone, exclusive_edge, margin (CSS order t,r,b,l), keyboard_interactivity: KeyboardInteractivity::{None,Exclusive,OnDemand} }` ([layer_shell.rs L1-83](https://github.com/zed-industries/zed/blob/347f171a3fd00a3b672382d78348b37bd969d17c/crates/gpui/src/platform/layer_shell.rs)).
  - `Layer` defaults to `Overlay` and `KeyboardInteractivity` defaults to `OnDemand`.
  - Opening one fails with `LayerShellNotSupportedError` when the compositor lacks `zwlr_layer_shell_v1`.
- **Implementation:**
  - `WaylandSurfaceState::new` calls `get_layer_surface`, then `set_size`, `set_anchor`, `set_keyboard_interactivity`, `set_margin` and `set_exclusive_zone` ([gpui_linux wayland/window.rs L160-204](https://github.com/zed-industries/zed/blob/347f171a3fd00a3b672382d78348b37bd969d17c/crates/gpui_linux/src/linux/wayland/window.rs#L160-L204)).
  - The global is bound at versions 1..=5 ([client.rs L305](https://github.com/zed-industries/zed/blob/347f171a3fd00a3b672382d78348b37bd969d17c/crates/gpui_linux/src/linux/wayland/client.rs#L305)).
  - `WindowOptions.display_id` selects the output.
- **Example:** [examples/layer_shell.rs](https://github.com/zed-industries/zed/blob/347f171a3fd00a3b672382d78348b37bd969d17c/crates/gpui/examples/layer_shell.rs) shows a transparent clock anchored `LEFT|RIGHT|BOTTOM` with a 40 px margin.
- **Protocol limits** ([wlr-layer-shell v5 spec](https://wayland.app/protocols/wlr-layer-shell-unstable-v1)):
  - **Positioning:** a layer surface is placed only by anchor + margin + size, never by absolute x/y. "Under the module" therefore means anchoring `TOP|RIGHT` (or `TOP|LEFT`) and computing a margin.
  - **`on_demand`** keyboard interactivity (since v4) gives normal focus semantics. `exclusive` grabs the keyboard while the surface is on top, which suits launchers and lock prompts.
  - **`get_popup`** lets an xdg_popup be parented to a layer surface.

### Anchored xdg_popup

`WindowKind::AnchoredPopup(PopupOptions { parent, anchor_rect, anchor, gravity, constraint_adjustment, offset, grab })` creates a real `xdg_popup` ([platform.rs L2809-2815](https://github.com/zed-industries/zed/blob/347f171a3fd00a3b672382d78348b37bd969d17c/crates/gpui/src/platform.rs#L2809-L2815), [popup.rs](https://github.com/zed-industries/zed/blob/347f171a3fd00a3b672382d78348b37bd969d17c/crates/gpui/src/platform/popup.rs)).

- If the parent is a layer surface, GPUI uses `zwlr_layer_surface_v1.get_popup` ([wayland/window.rs L207-250](https://github.com/zed-industries/zed/blob/347f171a3fd00a3b672382d78348b37bd969d17c/crates/gpui_linux/src/linux/wayland/window.rs#L207-L250)).
- With `grab: true` the compositor dismisses the popup on clicks into other apps. The grab "must be requested while the triggering input is still active", so open the popup from a mouse-down handler ([popup.rs L41-52](https://github.com/zed-industries/zed/blob/347f171a3fd00a3b672382d78348b37bd969d17c/crates/gpui/src/platform/popup.rs#L41-L52)).
- This only works for popups whose parent is *your own* GPUI window. It suits an all-GPUI bar, but not a Waybar `on-click`, because Waybar's surface belongs to another process.

### Closing on focus loss

- Wayland keyboard `Enter`/`Leave` events call `set_focused` ([client.rs L2088-2103](https://github.com/zed-industries/zed/blob/347f171a3fd00a3b672382d78348b37bd969d17c/crates/gpui_linux/src/linux/wayland/client.rs#L2088-L2103)), which fires the window's `active_status_change` callback ([window.rs L1601-1611](https://github.com/zed-industries/zed/blob/347f171a3fd00a3b672382d78348b37bd969d17c/crates/gpui_linux/src/linux/wayland/window.rs#L1601-L1611)).
- Views see this through `cx.observe_window_activation(window, …)` ([context.rs L420](https://github.com/zed-industries/zed/blob/347f171a3fd00a3b672382d78348b37bd969d17c/crates/gpui/src/app/context.rs#L420)) and `window.is_window_active()`. `window.remove_window()` then closes the window ([window.rs L2396](https://github.com/zed-industries/zed/blob/347f171a3fd00a3b672382d78348b37bd969d17c/crates/gpui/src/window.rs#L2396)).
- **Caveat (untested here):** with `OnDemand`, whether Hyprland gives the new layer surface keyboard focus on map, and whether a click on the empty desktop counts as "leaving", is compositor behaviour. Verify it on your machine. Fallbacks: `Exclusive` (always gets keys, so Esc-to-close works), an explicit Esc binding, or a timeout.

### If you used a normal xdg-toplevel instead

This is the fallback for toolkits without layer shell, and it is also how `WindowKind::PopUp`/`Normal` behave on Wayland ([wayland/window.rs L252+](https://github.com/zed-industries/zed/blob/347f171a3fd00a3b672382d78348b37bd969d17c/crates/gpui_linux/src/linux/wayland/window.rs#L252)). Hyprland ≥ 0.55 uses a **Lua config** ("Since Hyprland 0.55, hyprlang is deprecated in favor of lua", [wiki: Window Rules](https://wiki.hypr.land/Configuring/Basics/Window-Rules/)), and your setup uses `~/.config/hypr/hyprland.lua`. Current syntax, taken from the wiki source ([window-rules.md](https://github.com/hyprwm/hyprland-wiki/blob/main/content/configuring/core/rules/window-rules.md)):

```lua
-- ~/.config/hypr/hyprland.lua  (Hyprland >= 0.55 Lua syntax)
hl.window_rule({
  name             = "waybar-popup",
  match            = { class = "^(waybar-popup-.*)$" },   -- GPUI sets this via WindowOptions.app_id
  float            = true,
  pin              = true,          -- ignored unless floating
  size             = { 320, 360 },
  move             = { "cursor_x - (window_w * 0.5)", 40 },  -- monitor-local; 40 = bar height
  border_size      = 0,
  rounding         = 12,
  no_shadow        = false,
  no_anim          = true,
  stay_focused     = false,        -- true forces focus while visible (pinentry-style)
})
```

- **Effect types:** `float`, `pin`, `size`, `move`, `center`, `no_initial_focus` and `monitor` are *static* (evaluated once on open). `border_size`, `rounding`, `no_shadow`, `no_anim`, `stay_focused`, `dim_around`, `opacity` and `decorate` are *dynamic*.
- **Expressions:** `cursor_x`, `cursor_y`, `window_w`, `monitor_w`, etc. are monitor-local.
- **Dispatching from a shell:** `hyprctl dispatch '<lua dispatcher>'` is shorthand for `hyprctl eval 'hl.dispatch(...)'` ([using-hyprctl.md](https://github.com/hyprwm/hyprland-wiki/blob/main/content/configuring/core/advanced-configuration/using-hyprctl.md)):
  - close: `hyprctl dispatch 'hl.dsp.window.close({ window = "class:waybar-popup-calendar" })'`
  - spawn with rules: `hl.dsp.exec_cmd("cmd", { float = true, move = {0, 0} })`. This tracks the spawned PID, so it fails if the process forks before mapping ([dispatchers.md](https://github.com/hyprwm/hyprland-wiki/blob/main/content/configuring/core/dispatchers.md)).
  - `hyprctl cursorpos` gives global cursor coordinates.
- **Layer surfaces (the recommended path)** are styled with `hl.layer_rule({ match = { namespace = "…" }, blur = true, ignore_alpha = 0.5, animation = "…", dim_around = true, no_anim = true })` ([layer-rules.md](https://github.com/hyprwm/hyprland-wiki/blob/main/content/configuring/core/rules/layer-rules.md)). The namespace is `LayerShellOptions.namespace`; check it with `hyprctl layers`.

A layer surface on `Layer::Overlay`/`Top`, anchored to the bar edge, already floats, sits on all workspaces, has no border, and stays put. Window rules are only needed for a toplevel fallback.

## 3. Integrating with Waybar

Waybar's `custom/*` module runs a shell command for `on-click` and doesn't pass click coordinates ([Waybar wiki: Module: Custom](https://github.com/Alexays/Waybar/wiki/Module:-Custom)). See the sibling doc `waybar-custom-modules.md` for module mechanics.

**Option A: spawn per click (simplest)**

```jsonc
"custom/calendar": { "format": "", "on-click": "waybar-popup calendar --toggle" }
```

- **Single instance and toggle:** the binary first tries to connect to `$XDG_RUNTIME_DIR/waybar-popup.sock`. If a running instance answers, it sends "toggle" and exits. Otherwise it binds the socket, opens the layer surface, and quits when the window closes (`QuitMode::LastWindowClosed` is the Linux default, [app.rs L492](https://github.com/zed-industries/zed/blob/347f171a3fd00a3b672382d78348b37bd969d17c/crates/gpui/src/app.rs#L492)).
- **Pidfile alternative:** `pkill -f 'waybar-popup calendar' || waybar-popup calendar` works but is racy.
- **Cost:** every open pays process start, Wayland connect, Vulkan device creation and font loading, possibly a few hundred ms (unmeasured).

**Option B: long-running daemon (recommended for GPUI)**

- Start it once (`hl.exec_once`/systemd user unit) with `Application::with_quit_mode(QuitMode::Explicit)` ([app.rs L257](https://github.com/zed-industries/zed/blob/347f171a3fd00a3b672382d78348b37bd969d17c/crates/gpui/src/app.rs#L257)) so it survives with zero windows.
- `on-click` becomes a tiny client (`waybar-popup-ctl toggle calendar`, or `socat`/`nc -U`) that writes to a Unix socket.
- The daemon opens or removes a layer-shell window, so the GPU device and fonts stay warm.
- This is exactly gpui-shell's design:
  - IPC service on a `UnixListener` ([ipc/service.rs](https://github.com/andre-brandao/gpui-shell/blob/main/crates/app/src/ipc/service.rs))
  - `toggle_panel`, which closes the existing panel and returns if the same id is reopened, otherwise opens a `Layer::Overlay`, `KeyboardInteractivity::OnDemand`, transparent layer window ([panel.rs L207-279](https://github.com/andre-brandao/gpui-shell/blob/main/crates/app/src/panel.rs))
- GPUI on Linux has no "hide window" for layer surfaces, so recreate the window per toggle (cheap once the app is warm).
- **Positioning under the clicked module:** anchor `TOP|RIGHT` (or `TOP|LEFT`) and set `margin.top` to the bar height. Set the horizontal margin from `hyprctl cursorpos` at click time, made monitor-local and minus half the popup width. gpui-shell's `margin_from_click` does the same maths ([panel.rs L110](https://github.com/andre-brandao/gpui-shell/blob/main/crates/app/src/panel.rs)).

## 4. Ecosystem

- **[gpui-component / gpui-kit](https://github.com/longbridge/gpui-kit)** (Longbridge): 18.6k★, Apache-2.0, [0.8.0 released 2026-10-19](https://crates.io/crates/gpui-component), weekly cadence.
  - Has a shadcn-style set of 60+ components, including **`time/calendar.rs`, `date_picker.rs`, `slider.rs`, `switch.rs`, `popover.rs`, `list`, `notification`** and theming ([src listing](https://github.com/longbridge/gpui-kit/tree/main/crates/component/src)). Calendar and volume-slider popups are covered out of the box.
  - Pinned to `gpui-pre =0.3.9`, so your app must use the same snapshot. A separate optimised fork, [`gpui-fast`](https://crates.io/crates/gpui-fast) 0.1.10, also exists.
- **Third-party apps** listed in [awesome-gpui](https://github.com/zed-industries/awesome-gpui):
  - Shell-adjacent: Loungy (launcher, archived), Steward (Windows launcher on gpui-component), [clp](https://github.com/lostf1sh/clp) (Wayland clipboard manager)
  - Others: hummingbird (music player), Cap's GPUI desktop app, and more
- **Desktop shells on GPUI:**
  - [andre-brandao/gpui-shell](https://github.com/andre-brandao/gpui-shell) (156★, last push 2026-10-24): bar, launcher (`KeyboardInteractivity::Exclusive`), control center (PulseAudio volume, brightness, network, Bluetooth, MPRIS), OSD, notifications and dock, all as GPUI layer surfaces. It depends on `gpui`/`gpui_platform` from zed `main` via git. The README warns: "early development… significant amount of AI-generated code".
  - [`wk-gpui-ext`](https://docs.rs/wk-gpui-ext/latest/wk_gpui_ext/) 0.1.0 adds layer-shell convenience traits, but has very low adoption (14 downloads).

## 5. Alternatives

| Toolkit | Layer-shell support | Maturity / maintenance | Rust-native | Styling | Startup / footprint |
|---|---|---|---|---|---|
| **GPUI** (zed main / gpui-pre) | Native `WindowKind::LayerShell` + xdg_popup on layer parent | Pre-1.0, frequent breaks; crates.io official release stale since 2025-10 | Yes | Tailwind-like Rust builder API; gpui-component themes | Vulkan device init per process; no published numbers |
| **gtk4-rs + [gtk4-layer-shell](https://crates.io/crates/gtk4-layer-shell)** | Yes, via the C lib [wmww/gtk4-layer-shell](https://github.com/wmww/gtk4-layer-shell) (Arch pkg `gtk4-layer-shell` 1.3.0 installed) | Stable; crate 0.8.1 (2026-08-10), C lib pushed 2026-10-16 | Rust bindings to C | **GTK CSS**, same model as Waybar's `style.css` | GTK init; shared system libs, small binary |
| **[relm4](https://crates.io/crates/relm4)** | Same as gtk4 (+ gtk4-layer-shell) | 0.11.0 (2026-04-08), active | Yes (Elm-style over gtk4) | GTK CSS | as gtk4 |
| **[iced](https://crates.io/crates/iced) + [iced_layershell](https://crates.io/crates/iced_layershell)** | Yes, via waycrate [exwlshelleventloop](https://github.com/waycrate/exwlshelleventloop); popups described as "only a toy" in its README | iced 0.14.0 (2025-12); iced_layershell 0.19.1 (2026-07) | Yes | Rust theme/style closures | wgpu (or tiny-skia) init |
| **[Slint](https://crates.io/crates/slint)** | Not built in; community [layer-shika](https://crates.io/crates/layer-shika) 0.3.1 (AGPL-3.0, 2026-01) | Slint 1.18.1 stable; layer-shika young | Yes (+ `.slint` DSL) | Slint DSL | Light; software or GL renderer |
| **[egui/eframe](https://crates.io/crates/eframe)** | No first-party layer-shell; no maintained integration crate found on crates.io | 0.37.0 (2026-10-15), active | Yes | Immediate-mode, limited theming | Light; GL/wgpu |
| **[eww](https://github.com/elkowar/eww)** | Yes, gtk-layer-shell (GTK3, [Cargo.toml](https://github.com/elkowar/eww/blob/master/crates/eww/Cargo.toml)) | Mature; last push 2026-07-17 | Rust daemon, config in yuck | GTK CSS/SCSS | Daemon; `eww open/close` is instant |
| **[AGS](https://github.com/Aylur/ags) / [Astal](https://github.com/Aylur/astal)** | Yes (gtk layer shell) | Active (Astal pushed 2026-10-19) | No (Vala/C libs, TS/JS/Lua front ends) | GTK CSS | Daemon (gjs) |
| **[Quickshell](https://quickshell.org/docs/types/Quickshell/PanelWindow/)** | Yes: `PanelWindow` maps to `WlrLayershell`; has `PopupWindow` | Active (pushed 2026-10-22), v0.x | No (QML/C++) | QML | Daemon (Qt) |

## 6. Verdict and recommendation

**Feasible:** yes, technically. GPUI is one of the few Rust-native toolkits with first-class layer-shell support, it has a gpui-component calendar and slider ready to use, and gpui-shell proves the bar → popup pattern on Hyprland.

**Risks for a template repo:**

1. **Dependency source churn.** The official crates.io release is stale. The choice is between a git dep on a ~monorepo (slow first fetch, needs the newest stable Rust, currently 1.98.1 > local 1.97.1) and a third-party snapshot (`gpui-pre`) that could stop being published.
2. **API breakage.** The API breaks weekly, so template users will copy something that stops compiling unless it is pinned exactly (`=0.3.9` or a `rev = "…"` with a committed `Cargo.lock`).
3. **Vulkan-only runtime.** Machines without working Vulkan get `NoSupportedDeviceFound`.
4. **Unmeasured cold start and binary size.** This makes "spawn per click" a gamble.
5. **Sparse docs.** Expect to read GPUI source.

**Recommended approach for `waybar_gpui_plugin_template`:**

- **Default popup flavour:** gtk4-rs + gtk4-layer-shell (or relm4). It is stable on crates.io, the system library is already installed, its CSS matches Waybar theming, and startup is quick.
- **Optional `gpui` flavour:**
  - Depend on `gpui = { package = "gpui-pre", version = "=0.3.9" }` + `gpui_platform = { package = "gpui-pre-platform", version = "=0.3.9", features = ["wayland"] }` (or `gpui-component = "=0.8.0"`, which brings the same pins). Alternatively use a zed git `rev`.
  - Run as a single **daemon** with `QuitMode::Explicit`, a Unix-socket toggle protocol, and a `LayerShell` window on `Layer::Overlay`, anchored `TOP|RIGHT`, with `margin.top` = bar height and `KeyboardInteractivity::OnDemand`.
  - Close on `observe_window_activation` → `!is_window_active()`, plus an Esc binding.
  - Add a `hl.layer_rule` for blur and animation.

### Minimal layer-shell popup (gpui-pre 0.3.9 / zed main API)

This is adapted from the upstream [examples/layer_shell.rs](https://github.com/zed-industries/zed/blob/347f171a3fd00a3b672382d78348b37bd969d17c/crates/gpui/examples/layer_shell.rs), which is byte-identical in the `gpui-pre` 0.3.9 tarball. Every type and method used was checked against the 0.3.9 source. **It was not compiled locally** (nothing was installed).

```toml
# Cargo.toml
[dependencies]
gpui          = { package = "gpui-pre",          version = "=0.3.9" }
gpui_platform = { package = "gpui-pre-platform", version = "=0.3.9", features = ["wayland"] }
```

```rust
use gpui::{
    App, Bounds, Context, Size, Window, WindowBackgroundAppearance, WindowBounds, WindowKind,
    WindowOptions, div, layer_shell::*, point, prelude::*, px, rgb, white,
};

struct Popup;

impl Popup {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Close when keyboard focus leaves the surface (wl_keyboard.leave).
        cx.observe_window_activation(window, |_, window, _| {
            if !window.is_window_active() {
                window.remove_window();
            }
        })
        .detach();
        Popup
    }
}

impl Render for Popup {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .p_4()
            .rounded_xl()
            .bg(rgb(0x1e1e2e))
            .text_color(white())
            .child("Hello from GPUI")
    }
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        cx.open_window(
            WindowOptions {
                titlebar: None,
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: point(px(0.), px(0.)), // ignored for layer surfaces
                    size: Size::new(px(320.), px(240.)),
                })),
                app_id: Some("waybar-popup".into()),
                window_background: WindowBackgroundAppearance::Transparent,
                kind: WindowKind::LayerShell(LayerShellOptions {
                    namespace: "waybar-popup".into(),
                    layer: Layer::Overlay,
                    anchor: Anchor::TOP | Anchor::RIGHT,
                    // top, right, bottom, left — top = Waybar height
                    margin: Some((px(36.), px(8.), px(0.), px(0.))),
                    keyboard_interactivity: KeyboardInteractivity::OnDemand,
                    ..Default::default()
                }),
                focus: true,
                ..Default::default()
            },
            |window, cx| cx.new(|cx| Popup::new(window, cx)),
        )
        .unwrap();
    });
}
```

```lua
-- ~/.config/hypr/hyprland.lua
hl.layer_rule({ match = { namespace = "waybar-popup" }, blur = true, ignore_alpha = 0.3, animation = "slide top" })
```

```jsonc
// Waybar config
"custom/hello": { "format": "", "on-click": "pkill -x waybar-popup || waybar-popup" }
```

## Sources

**Zed / GPUI** (commit `347f171a3fd00a3b672382d78348b37bd969d17c`, 2026-10-24)
- https://github.com/zed-industries/zed/blob/347f171a3fd00a3b672382d78348b37bd969d17c/crates/gpui/README.md
- https://github.com/zed-industries/zed/blob/347f171a3fd00a3b672382d78348b37bd969d17c/crates/gpui/Cargo.toml
- https://github.com/zed-industries/zed/blob/347f171a3fd00a3b672382d78348b37bd969d17c/crates/gpui/src/platform.rs#L2801-L2828
- https://github.com/zed-industries/zed/blob/347f171a3fd00a3b672382d78348b37bd969d17c/crates/gpui/src/platform/layer_shell.rs
- https://github.com/zed-industries/zed/blob/347f171a3fd00a3b672382d78348b37bd969d17c/crates/gpui/src/platform/popup.rs
- https://github.com/zed-industries/zed/blob/347f171a3fd00a3b672382d78348b37bd969d17c/crates/gpui/examples/layer_shell.rs
- https://github.com/zed-industries/zed/blob/347f171a3fd00a3b672382d78348b37bd969d17c/crates/gpui/src/app.rs#L492
- https://github.com/zed-industries/zed/blob/347f171a3fd00a3b672382d78348b37bd969d17c/crates/gpui/src/app/context.rs#L420
- https://github.com/zed-industries/zed/blob/347f171a3fd00a3b672382d78348b37bd969d17c/crates/gpui_linux/src/linux/wayland/window.rs#L160-L250
- https://github.com/zed-industries/zed/blob/347f171a3fd00a3b672382d78348b37bd969d17c/crates/gpui_linux/src/linux/wayland/client.rs#L305
- https://github.com/zed-industries/zed/blob/347f171a3fd00a3b672382d78348b37bd969d17c/crates/gpui_linux/Cargo.toml
- https://github.com/zed-industries/zed/blob/347f171a3fd00a3b672382d78348b37bd969d17c/crates/gpui_wgpu/Cargo.toml
- https://github.com/zed-industries/zed/blob/347f171a3fd00a3b672382d78348b37bd969d17c/crates/gpui_platform/Cargo.toml
- https://github.com/zed-industries/zed/blob/347f171a3fd00a3b672382d78348b37bd969d17c/rust-toolchain.toml
- https://github.com/zed-industries/zed/blob/347f171a3fd00a3b672382d78348b37bd969d17c/script/linux#L206-L232
- https://github.com/zed-industries/zed/blob/347f171a3fd00a3b672382d78348b37bd969d17c/docs/src/linux.md#L143-L167
- PR #35610 (layer shell, merged 2025-10-29): https://github.com/zed-industries/zed/pull/35610
- PR #49277 (gpui_platform extraction, 2026-02-19): https://github.com/zed-industries/zed/pull/49277
- PR #60232 (anchored xdg_popup, 2026-07-08): https://github.com/zed-industries/zed/pull/60232

**crates.io / docs.rs**
- https://crates.io/crates/gpui (0.2.2, 2025-10-22) · https://docs.rs/gpui
- https://crates.io/crates/gpui-pre and https://crates.io/crates/gpui-pre-platform (0.3.9, 2026-10-12, snapshot of zed@93d7456)
- https://crates.io/crates/gpui-component (0.8.0, 2026-10-19) · https://docs.rs/gpui-component
- https://crates.io/crates/gpui-fast
- https://docs.rs/wk-gpui-ext/latest/wk_gpui_ext/
- https://crates.io/crates/gtk4-layer-shell · https://crates.io/crates/relm4 · https://crates.io/crates/iced · https://crates.io/crates/iced_layershell · https://crates.io/crates/slint · https://crates.io/crates/layer-shika · https://crates.io/crates/eframe

**Ecosystem repos**
- https://github.com/longbridge/gpui-kit (Cargo.toml pin rationale; `crates/component/src`)
- https://github.com/andre-brandao/gpui-shell (README, Cargo.toml, `crates/app/src/panel.rs`, `crates/app/src/ipc/service.rs`)
- https://github.com/zed-industries/awesome-gpui
- https://github.com/wmww/gtk4-layer-shell · https://github.com/waycrate/exwlshelleventloop · https://github.com/elkowar/eww · https://github.com/Aylur/ags · https://github.com/Aylur/astal · https://quickshell.org/docs/types/Quickshell/PanelWindow/

**Protocols / Hyprland / Waybar**
- wlr-layer-shell-unstable-v1 (v5): https://wayland.app/protocols/wlr-layer-shell-unstable-v1
- Hyprland wiki, Window Rules (Lua, ≥ 0.55): https://wiki.hypr.land/Configuring/Basics/Window-Rules/ (source: https://github.com/hyprwm/hyprland-wiki/blob/main/content/configuring/core/rules/window-rules.md)
- Hyprland wiki, Layer Rules: https://github.com/hyprwm/hyprland-wiki/blob/main/content/configuring/core/rules/layer-rules.md
- Hyprland wiki, Dispatchers: https://wiki.hypr.land/Configuring/Basics/Dispatchers/ (source: https://github.com/hyprwm/hyprland-wiki/blob/main/content/configuring/core/dispatchers.md)
- Hyprland wiki, Using hyprctl: https://github.com/hyprwm/hyprland-wiki/blob/main/content/configuring/core/advanced-configuration/using-hyprctl.md
- Waybar custom module: https://github.com/Alexays/Waybar/wiki/Module:-Custom
