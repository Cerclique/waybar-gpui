# waybar-gpui guide

For **authors**: people writing their own panels with the `waybar_gpui` SDK,
in their own repository. Terms (bar module, panel, popover, card, backdrop,
program, program ID…) are defined in the [glossary](../../GLOSSARY.md).

New here? Start with the quickstart in the [README](../../README.md), then
[Writing a panel](writing-a-panel.md).

## Building panels

- [Writing a panel](writing-a-panel.md): the `Panel` trait, `App`, the card,
  sizing, GPUI basics you'll need.
- [Panel arguments](panel-arguments.md): one panel, several bar modules.
- [Live data and state](live-data-and-state.md): timers, event streams, state
  that survives a close.
- [Theming](theming.md): `Theme`, `card()`, your own colours.
- [Components](components.md): gpui-component widgets, and their limits.
- [Closing and Esc](closing-and-esc.md): how popovers close, and how a panel
  takes part.

## Shipping

- [Dev loop](dev-loop.md): `preview`, trying it on your bar, logs, panics.
- [Installing](installing.md): `cargo install`, the Waybar and Hyprland config.
- [Using a waybar-gpui program](using-a-program.md): for people who install
  your program; link it from your README.

## Background

- [How it works](how-it-works.md): what happens on a click, in plain terms.
- [Limits](limits.md): what isn't possible, and versioning.
- [Troubleshooting](troubleshooting.md): symptoms and fixes.

Contributors changing the SDK itself: see
[`docs/architecture.md`](../architecture.md).
