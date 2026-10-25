# Hyprland: two clients with exclusive-keyboard overlay layers

Research date: 2026-10-24. Target: Hyprland **v0.56.2** (tag commit `efb5099`). Ticket: #12, child of map #7.

Sources: the Hyprland source at tag `v0.56.2` (read only, not built or run), and the wlr-layer-shell protocol XML that ships with that tag (`protocols/wlr-layer-shell-unstable-v1.xml`). Links use `https://github.com/hyprwm/Hyprland/blob/v0.56.2/<path>#L<n>`. No experiments were run, so nothing here was checked on a live session. Some conclusions follow from reading the code paths together. Those are marked **(inferred)**.

## Question

A client maps an Overlay layer surface with `keyboard_interactivity = exclusive` while another client's exclusive Overlay layer is already open. Who gets keyboard focus and pointer clicks? Can a click on Waybar reach Waybar while either one is open? Does the second surface get input at all?

## TL;DR

- Hyprland keeps a single global list, `CInputManager::m_exclusiveLSes`, of every mapped exclusive-keyboard layer surface on every monitor, in the order they were mapped. Neither the layer (Top or Overlay), the monitor nor the client changes that.
- **Keyboard:** the surface that maps most recently takes keyboard focus as it maps. After that, keyboard focus **follows the pointer** between exclusive layers: whenever the pointer is over one exclusive layer's input region, that layer gets keyboard focus, even though another exclusive layer holds focus (unless `input:follow_mouse = 3`). The protocol leaves this case to the compositor ("implementation-defined… no guarantee… (if ever)").
- **Pointer:** a hit test runs over the exclusive list, newest first. The most recently mapped exclusive surface whose input region contains the cursor gets the pointer, and so the click. If no exclusive surface's input region contains the cursor (another monitor, or a gap in the input regions), the pointer goes to the **oldest** exclusive surface (`begin()`), with coordinates outside its bounds.
- **Waybar can't be clicked while any exclusive layer is mapped.** It isn't in the list, and the "forced above all" block always produces a surface. Waybar is reached only through the normal Overlay/Top lookup, which runs only when that block found nothing.
- **Does the second one get input?** Yes. It gets keyboard focus as it maps, and it gets the pointer wherever its input region covers the cursor, because the newest surface wins the hit test. The older surface still gets the pointer, and keyboard focus with it, wherever the newer one's input region does not cover the cursor on the older one's monitor. It also gets every click on a monitor where no exclusive surface's input region covers the cursor.
- **For this template:** the popover is a full-output exclusive surface. Suppose an exclusive launcher (fuzzel, rofi, wofi, a password prompt) opens on top of it, from a keybind, since Waybar itself can't be clicked. Moving the pointer outside the launcher hands the **keyboard back to the popover**. A click outside the launcher goes to the popover, whose `on_mouse_down_out` closes the popover, while the launcher stays open. A full-output exclusive surface mapped later on the same monitor (wlogout, for example) covers the popover completely: Esc and clicks go to it, and the popover is stuck until that surface unmaps.

## 1. Protocol: what the spec promises

`wlr-layer-shell-unstable-v1.xml`, enum `keyboard_interactivity`, entry `exclusive` ([L226–245](https://github.com/hyprwm/Hyprland/blob/v0.56.2/protocols/wlr-layer-shell-unstable-v1.xml#L226-L245)):

> For the top and overlay layers, the seat will always give exclusive keyboard focus to the top-most layer which has keyboard interactivity set to exclusive. If this layer contains multiple surfaces with keyboard interactivity set to exclusive, the compositor determines the one receiving keyboard events in an implementation-defined manner. In this case, no guarantee is made when this surface will receive keyboard focus (if ever).

So two exclusive Overlay surfaces are explicitly implementation-defined. The spec says nothing about pointer input for exclusive surfaces. Routing all clicks to them is Hyprland's own choice.

## 2. Bookkeeping: `m_exclusiveLSes`

- Declared as `std::vector<PHLLSREF> m_exclusiveLSes;` on the global input manager ([InputManager.hpp L176](https://github.com/hyprwm/Hyprland/blob/v0.56.2/src/managers/input/InputManager.hpp#L176)). It holds one list for all monitors, with no per-layer or per-monitor split.
- On map, an exclusive surface is **appended** ([LayerSurface.cpp L185–188](https://github.com/hyprwm/Hyprland/blob/v0.56.2/src/desktop/view/LayerSurface.cpp#L185-L188)). So `begin()` is the oldest and `back()` the newest.
- On unmap it is erased ([LayerSurface.cpp L230](https://github.com/hyprwm/Hyprland/blob/v0.56.2/src/desktop/view/LayerSurface.cpp#L230)). A live change of `keyboard_interactivity` on commit adds the surface to the list or removes it ([L381–388](https://github.com/hyprwm/Hyprland/blob/v0.56.2/src/desktop/view/LayerSurface.cpp#L381-L388)).

## 3. Keyboard focus

### 3.1 On map: the newcomer takes it

`CLayerSurface::onMap` ([L190–207](https://github.com/hyprwm/Hyprland/blob/v0.56.2/src/desktop/view/LayerSurface.cpp#L190-L207)): `GRABSFOCUS` is unconditionally true for an exclusive surface. Hyprland drops a seat grab that would reject the surface, releases all mouse buttons, and calls `rawSurfaceFocus(new surface)` and `setPointerFocus(new surface, …)`. Nothing in this path checks whether another exclusive surface already exists, and `rawSurfaceFocus` ([FocusState.cpp L223–251](https://github.com/hyprwm/Hyprland/blob/v0.56.2/src/desktop/state/FocusState.cpp#L223-L251)) refuses only for a session lock or a rejecting seat grab. **The second surface gets the keyboard as it maps, and the first one gets a `wl_keyboard.leave`.**

### 3.2 Afterwards: focus follows the pointer between exclusive layers

In `mouseMoveUnified`, `allowKeyboardRefocus` is set to false when the currently focused surface is an exclusive layer ([InputManager.cpp L636–642](https://github.com/hyprwm/Hyprland/blob/v0.56.2/src/managers/input/InputManager.cpp#L636-L642)). The layer-surface branch then focuses the found layer when

```cpp
pFoundLayerSurface && interactivity != NONE && FOLLOWMOUSE != 3 &&
(allowKeyboardRefocus || pFoundLayerSurface->...interactivity == EXCLUSIVE)
```

([L733–736](https://github.com/hyprwm/Hyprland/blob/v0.56.2/src/managers/input/InputManager.cpp#L733-L736)). The exclusive check short-circuits the guard, so **plain pointer motion onto another exclusive layer moves keyboard focus to it** with any `input:follow_mouse` value except 3. `pFoundLayerSurface` is set only when the hit test actually hits a surface (§4). The `begin()` fallback sets `foundSurface` but not `pFoundLayerSurface` ([L461–464](https://github.com/hyprwm/Hyprland/blob/v0.56.2/src/managers/input/InputManager.cpp#L461-L464)), so pointer motion on an empty monitor does not move the keyboard.

Windows can't take focus back while any exclusive layer exists: `rawWindowFocus` refuses ("Refusing a keyboard focus to a window because of an exclusive ls", [FocusState.cpp L109–112](https://github.com/hyprwm/Hyprland/blob/v0.56.2/src/desktop/state/FocusState.cpp#L109-L112)), and `refocusLastWindow` returns early ([InputManager.cpp L1757–1761](https://github.com/hyprwm/Hyprland/blob/v0.56.2/src/managers/input/InputManager.cpp#L1757-L1761)).

### 3.3 When one of them unmaps

`onUnmap` ([LayerSurface.cpp L262–276](https://github.com/hyprwm/Hyprland/blob/v0.56.2/src/desktop/view/LayerSurface.cpp#L262-L276)): if the unmapping surface had keyboard or pointer focus, it calls `refocusLastWindow`. That returns false while another exclusive surface remains, so the code falls through to `refocus()`, which is `mouseMoveUnified(refocus = true)`. With `refocus = true`, `allowKeyboardRefocus` stays true, so if the cursor is over the remaining exclusive surface, that surface gets the keyboard back. **(inferred)** If the cursor is not over it (other monitor), only the `begin()` fallback applies. That sets pointer focus but not `pFoundLayerSurface`, so the keyboard may stay unfocused until the pointer enters the remaining surface. I did not trace this edge case to the end.

## 4. Pointer and clicks

`mouseMoveUnified`, block commented `// forced above all` ([InputManager.cpp L453–465](https://github.com/hyprwm/Hyprland/blob/v0.56.2/src/managers/input/InputManager.cpp#L453-L465)):

1. `layerPopupSurfaceAt(mouse, &m_exclusiveLSes, …)`: xdg popups of exclusive layers.
2. `layerSurfaceAt(mouse, &m_exclusiveLSes, …)`: the layer surfaces themselves.
3. Else: `foundSurface = m_exclusiveLSes.begin()->surface`, positioned at that surface's own origin.

Both hit tests walk the list **in reverse**, newest first ([ViewHitTester.cpp L328–364](https://github.com/hyprwm/Hyprland/blob/v0.56.2/src/desktop/state/ViewHitTester.cpp#L328-L364)). They test global cursor coordinates against `m_geometry`, which is global because it is laid out from `full_area` = monitor position + size ([Renderer.cpp L2588](https://github.com/hyprwm/Hyprland/blob/v0.56.2/src/render/Renderer.cpp#L2588), [L2662](https://github.com/hyprwm/Hyprland/blob/v0.56.2/src/render/Renderer.cpp#L2662)). They use the surface's **input region** (`at(…, allowsInput = true)` → `effectiveInputRegion()`, [Compositor.cpp L488–505](https://github.com/hyprwm/Hyprland/blob/v0.56.2/src/protocols/core/Compositor.cpp#L488-L505)), and they skip a surface whose input region is empty. No monitor filter applies.

The normal lookup comes after this block: overlay popups, then Overlay layers, then Top layers (where Waybar lives), then windows ([L467–478](https://github.com/hyprwm/Hyprland/blob/v0.56.2/src/managers/input/InputManager.cpp#L467-L478), [L530](https://github.com/hyprwm/Hyprland/blob/v0.56.2/src/managers/input/InputManager.cpp#L530)). Each step runs only `if (!foundSurface)`. Step 3 never leaves `foundSurface` null while the list is non-empty, so **nothing outside the exclusive list can receive pointer focus**. A button press goes to the pointer-focused surface (`sendPointerButton`, [L930](https://github.com/hyprwm/Hyprland/blob/v0.56.2/src/managers/input/InputManager.cpp#L930)).

Only these come before the block: IME popups, focus-priority windows (Hyprland's own permission popups), the session lock branch ([L366](https://github.com/hyprwm/Hyprland/blob/v0.56.2/src/managers/input/InputManager.cpp#L366)), forced focus, and "keep focus on the surface under a held button" ([L409–443](https://github.com/hyprwm/Hyprland/blob/v0.56.2/src/managers/input/InputManager.cpp#L409-L443)).

## 5. Answers for concrete layouts

A = first exclusive Overlay surface (mapped earlier), B = second (mapped later).

| Situation | Pointer / click goes to | Keyboard |
|---|---|---|
| B just mapped | B, if the cursor is in B's input region, else A (by hit test or the `begin()` fallback) | B (onMap) |
| Cursor over B's input region (B overlaps A or not) | B | B |
| Cursor over A, outside B's input region | A | moves to **A** on motion (§3.2) |
| Cursor on a monitor with neither (or a gap in both input regions) | **A** (`begin()`), coordinates outside A | unchanged |
| Cursor over Waybar | whichever of the rows above applies, never Waybar | unchanged |
| A and B both full-output, same monitor, infinite input | always B | always B; A gets nothing until B unmaps |
| A and B full-output on different monitors | the monitor's own surface; a third monitor goes to A | follows the pointer between monitors |

Applied to this template's popover (full-output, infinite input region, so it hit-tests everywhere on its monitor):

- **Popover open, then an exclusive launcher opens (a smaller surface).** The launcher has the keyboard at first. Pointer motion off the launcher onto the popover's monitor steals the keyboard back to the popover, which the launcher's user would not expect. A click there closes the popover through `on_mouse_down_out`, and the launcher stays open.
- **Popover open, then another full-output exclusive surface opens on the same monitor.** The other surface owns all input on that monitor. The popover can't see Esc or clicks there. Clicks on another monitor still go to the popover (`begin()` fallback) and close it.
- **Launcher open, then the user tries to open the popover from Waybar.** Impossible: the click never reaches Waybar. The popover opens only from a keybind or CLI, and then it is B and takes everything on its monitor.

## 6. What this does not cover

- No live test. Clients' own reactions (a launcher that quits on `wl_keyboard.leave`, for example) are not part of Hyprland and were not checked.
- `input:follow_mouse = 3` disables the hover-steal in §3.2. The template's user has `follow_mouse = 1`.
- Layer rules (`hl.*` Lua) that could change focus behaviour, such as `noFocus` or `aboveLock`, were not examined.
