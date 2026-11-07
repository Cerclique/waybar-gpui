# A backdrop surface on every other monitor

While a popover is open, the runtime also maps a transparent full-output layer surface on every other monitor. It sits on the Overlay layer, has no keyboard interactivity and its own namespace (`waybar-gpui-backdrop:<id>`), and any click on it dismisses the popover. Together with the popover's surface, these make the backdrop span every monitor: a click anywhere but the card closes the popover and reaches nothing below.

From Hyprland 0.57 ([hyprwm/Hyprland#15899](https://github.com/hyprwm/Hyprland/pull/15899)), exclusive-keyboard layers no longer capture the pointer, so a click on another monitor goes to whatever is there and the popover never hears of it. The keyboard, though, stays with the popover while it's mapped. We rejected two alternatives:

- **Leaving the popover open**, which is 0.57's default. The pointer would work on the other monitor while typing still went to the popover.
- **Closing when the pointer leaves the popover's monitor.** That closes on mere movement.

## Consequences

- Waybar can't be clicked on any monitor while a popover is open: a click on another monitor's bar module only dismisses, as it always has on the popover's own monitor. Hyprland 0.57 would have allowed a one-click switch there, and we gave it up.
- One extra surface per other monitor for each open. The monitors are read when the popover opens, so a monitor plugged in while it is open has no backdrop until the next open.
- On Hyprland 0.56 the backdrop surfaces never get clicks; the card's geometric `on_mouse_down_out` test still closes on those. Both paths stay, and only 0.57 is tested.
- Amends the consequences of [ADR 0001](0001-full-monitor-surface-with-drawn-card.md), whose decision stands. Research: [`docs/research/hyprland-057-pointer-routing.md`](../research/hyprland-057-pointer-routing.md); ticket [#134](https://github.com/Cerclique/waybar-gpui/issues/134).
