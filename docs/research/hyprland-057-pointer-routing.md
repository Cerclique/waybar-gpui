# Hyprland 0.57: pointer focus no longer follows exclusive-keyboard layers

Research date: 2026-11-07. Ticket: #134. Compares Hyprland **v0.56.2** with **v0.57.1**. This supersedes the pointer half of [hyprland-two-exclusive-layers.md](hyprland-two-exclusive-layers.md); the keyboard half still holds.

Sources: `src/managers/input/InputManager.cpp` and `src/desktop/state/FocusState.cpp` at both tags, read only, plus the commit and PR named below. The symptoms were seen by hand on Hyprland 0.57.1 with `make dev`, on both `main` (gpui-pre 0.3.9) and gpui-pre 0.3.11.

## TL;DR

- **The change:** [hyprwm/Hyprland#15899](https://github.com/hyprwm/Hyprland/pull/15899), "Decouple pointer focus from layer-surface `keyboard_interactivity`". It was merged 2026-08-21 as commit [`d29916a91`](https://github.com/hyprwm/Hyprland/commit/d29916a91) and first released in 0.57. It removes the "forced above all" block from `mouseMoveUnified` ([v0.56.2 L453–465](https://github.com/hyprwm/Hyprland/blob/v0.56.2/src/managers/input/InputManager.cpp#L453-L465)). It also renames `m_exclusiveLSes` to `m_exclusiveKeyboardLSes` and stops `onMap` from forcing pointer focus.
- **Pointer, from 0.57:** an exclusive-keyboard layer surface gets the pointer only where its input region is under the cursor, like any other surface. A click on another monitor goes to whatever is there: a window, the wallpaper, or Waybar. The popover gets no event at all, not even one with coordinates outside its bounds.
- **Keyboard, unchanged:** while an exclusive-keyboard layer is mapped, Hyprland refuses keyboard focus to windows ([FocusState.cpp v0.57.1 L111–112](https://github.com/hyprwm/Hyprland/blob/v0.57.1/src/desktop/state/FocusState.cpp#L111-L112), "Refusing a keyboard focus to a window because of an exclusive ls"). After a click on a window on the other monitor, that window gets pointer events but typing still goes to the popover.
- **Effect on this repo:**
  - The `click on another monitor` close path (`on_mouse_down_out` in `runtime.rs`) never fires on 0.57.
  - Waybar on another monitor can be clicked while a popover is open, so one click there toggles a new panel open.
  - On the popover's own monitor nothing changes, because the full-monitor surface covers everything there, Waybar included.
- **A separate backdrop surface would now work.** [ADR 0001](../adr/0001-full-monitor-surface-with-drawn-card.md) rejected one because "Hyprland 0.56 sends every click to the exclusive-keyboard layer". From 0.57, a non-exclusive layer surface on another monitor gets the clicks under it.

## Why upstream changed it

From the PR: the wlr-layer-shell spec says "Layer surfaces receive pointer, touch, and tablet events normally", and `keyboard_interactivity` concerns the keyboard only. Before the PR, any mapped exclusive-keyboard surface "anywhere, on any output" meant "your pointer will never be able to click or focus anything else". The PR is a spec conformance fix, so it isn't likely to be reverted.

## Not checked

- Whether the popover's surface gets `wl_pointer.leave` when the pointer crosses to the other monitor (expected: yes, as for any surface), and whether GPUI exposes it.
- A non-exclusive catcher surface on the other monitors, on a live session.
