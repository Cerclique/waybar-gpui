# One full-monitor surface with a drawn card, not `xdg_popup`

A popover is one transparent layer surface on the Overlay layer covering the whole monitor, bar included, with exclusive keyboard focus; the panel's card is drawn inside it with GPUI's `anchored()`, and a click on the rest (the backdrop) dismisses. A real `xdg_popup` anchored to the bar module is impossible: Wayland popups need a parent surface owned by the same client, and our program is not Waybar. Of the layer-shell designs, closing a panel-sized surface on focus loss failed under `follow_mouse = 1`, and a separate backdrop surface never received clicks, because Hyprland 0.56 sends every click to the exclusive-keyboard layer while one is mapped.

## Consequences

- While a popover is open Waybar can't be clicked: the first click outside the card only dismisses, including a click on another bar module.
- Clicks on other monitors also land on the surface, with coordinates outside it, so the card tests them geometrically (`on_mouse_down_out`).
- The design depends on Hyprland's exclusive-layer routing; see [`docs/architecture.md`](../architecture.md#hyprland-exclusive-layer-input-routing).
