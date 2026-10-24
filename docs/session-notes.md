# Session notes — 2026-10-24

Handoff from the first Claude Code session that researched and scaffolded this
project (renamed from `waybar-plugin-template` to `waybar_gpui_plugin_template`
at the end of that session; the old name may appear in earlier context).
Read this, then `README.md`, then the two research docs.

## What was asked, in order

1. "I'd like to be able to create custom waybar plugin for hyprland. Start a
   research session to see what is available and what should be a good
   starting point." → `docs/research/waybar-custom-modules.md`
2. "Explore if there is any possibility using GPUI framework in rust. To create
   small UI windows for instance" → `docs/research/gpui-popup-windows.md`
3. "yes, scaffold it with GPUI for the popup" → the current code.
4. Mid-scaffold: "instead of a real popup that will create a full window from
   start, can clicking on the waybar icon open a popover that would be created
   using GPUI?" → answered below; design adjusted accordingly.

## Key decisions and why

- **Bar module = `custom/*` continuous script in Rust**, not CFFI. Crash
  isolation, any language, no Waybar restart to pick up rebuilds. CFFI in
  Waybar 0.15.0 is GTK3-only, crashes take the bar down, and has a `deinit`
  check bug (always export `wbcffi_deinit` if CFFI is ever added).
- **Hyprland IPC hand-rolled** (`crates/hypr`, ~150 lines) instead of the
  `hyprland` crate, whose 0.4 line has been beta since 2024. Request socket is
  opened/closed per request (Hyprland freezes on held connections).
- **Popover question:** a true `xdg_popup` attached to Waybar's icon is
  impossible from another process (Wayland popups need a parent surface owned
  by the same client); GPUI can't live inside Waybar either. Chosen approach: a
  borderless **layer-shell** surface on the Overlay layer, anchored to the
  bar's edge + left, margins computed from `j/cursorpos` and the monitor's
  `reserved` area, `exclusive_zone = -1`. Only a GPUI-drawn bar (like
  gpui-shell) would allow a real anchored popover.
- **Daemon + Unix socket** (`$XDG_RUNTIME_DIR/waybar-popup.sock`, line protocol
  `toggle <panel>` / `close` / `quit`). First `toggle` spawns the daemon in its
  own process group; GPUI uses `QuitMode::Explicit` so it stays warm.
- **GPUI source:** community `gpui-pre` / `gpui-pre-platform` pinned `=0.3.9`
  (snapshot of zed@93d7456). Official crates.io `gpui` 0.2.2 is stale and lacks
  layer shell. Zed main would need Rust 1.98.1; gpui-pre builds on local 1.97.1.
- **Output selection:** GPUI's Wayland display UUID is
  `Uuid::new_v5(NAMESPACE_DNS, wl_output name)`, matched against the Hyprland
  monitor name.
- **Dismissal (changed twice after real-click tests):**
  1. Close on focus loss: failed, the user's Hyprland has `follow_mouse = 1`;
     moving the pointer onto the popover handed focus to the window underneath
     and the popover closed within ~300 ms.
  2. Separate transparent full-output backdrop surface under an
     exclusive-keyboard popover: Esc worked, but clicks outside (and on the
     icon) did nothing; neither the backdrop nor Waybar got them. Hypothesis
     (not verified in Hyprland source): Hyprland restricts pointer input to the
     exclusive-keyboard layer while it is open.
  3. Current: **one** full-output transparent layer surface (Overlay,
     `exclusive_zone -1`, covers the bar, `KeyboardInteractivity::Exclusive`);
     the panel is a card drawn inside it at `Placement::origin`, `.occlude()`d
     so card clicks stay in the card; any other click or Esc closes. The 300 ms
     reopen guard is gone.

## Verified on this machine

Arch, Hyprland 0.56.2 (Lua config), Waybar 0.15.0, Rust 1.97.1, NVIDIA + Vulkan,
two 1920×1080 monitors (DP-2 at x=0, DP-1 at x=1920), Waybar height 36.

- `cargo build`, `cargo test` (6 tests), `cargo clippy --all-targets -D warnings`: clean.
- `waybar-hypr-ws` and `python/hypr_ws.py` print correct JSON lines.
- `waybar-popup toggle workspace`: popover maps on overlay layer at the
  computed geometry (checked via `hyprctl -j layers`), renders correctly
  (screenshot via `grim`), second toggle closes it, `quit` stops the daemon and
  removes the socket.
- Release: popup binary 26 MB, module 450 KB; cold open ~100 ms, warm < 20 ms.

## Not yet tested / next steps

- [x] `make dev` click test (first round): opens under the icon in ~130 ms,
      second click closes. Found the follow_mouse dismissal bug (fixed above)
      and blank icons (fixed: `\uXXXX` escapes in examples/waybar/config.jsonc).
- [x] Second real-click round (backdrop design): Esc 4/4, popover stays open
      while the mouse moves, both monitors OK, icons OK; click-outside and icon
      re-click did nothing → led to the single-surface design.
- [x] Third real-click round (single-surface design): click outside 3/3,
      icon re-click 4/4, click inside the card stays open, Esc 2/2, stays open
      while the mouse moves. Opens in ~410 ms when the click starts the daemon,
      130–150 ms after. No 0×0 first open this round.
- [x] Git repo + private GitHub repo (Cerclique/waybar_gpui_plugin_template).
      Pushing over SSH fails from the agent shell (`Permission denied
      (publickey)`); the agent pushed over HTTPS with `gh auth git-credential`.
      Root cause: the key has a passphrase and no ssh-agent runs in the
      session; the user's terminal prompts, the agent shell can't. Left as is
      for now (fix options: systemd `ssh-agent.socket` + `SSH_AUTH_SOCK`, or
      HTTPS via `gh auth setup-git`).
- [x] #1 fixed: Hyprland routes every click to the exclusive-keyboard layer
      while it is open, including clicks on other monitors, which arrive with
      coordinates outside the surface (InputManager.cpp L453–465 at v0.56.2,
      "forced above all"; also explains why the round-2 backdrop never got
      clicks). The card now closes via `on_mouse_down_out` (geometric test),
      not a hover-based handler. Confirmed with real clicks both ways.
- [x] #2 closed as not reproducible: ~27 cold starts (CLI + spawn path) and
      four real-click rounds after the redesign all drew in 250–410 ms. Only
      seen once, with the old focus-loss design. Reopen if Waybar's log shows
      an `opened` line with no drawn popover.

Open work is tracked as GitHub issues (see `docs/agents/issue-tracker.md`):

- #3 Try `make install` in the real Waybar bar (PATH question)
- #4 Workspace panel: update live while open
- #5 Workspace panel: click a row to focus that window (verify Lua dispatcher)
- #6 More popover panels (calendar, volume) via gpui-component

## Side effects from the research session

The Waybar research agent ran `hyprctl dispatch focusmonitor DP-1`,
`hyprctl keyword general:gaps_in 5`, then `hyprctl reload` to restore config.
Scratch files lived in `/tmp/claude-1000/...` (session scratchpad, disposable).
