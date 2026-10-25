# Components

[gpui-component](https://github.com/longbridge/gpui-component) is a widget
library for GPUI: buttons, switches, inputs, dropdowns, lists. The SDK offers
it behind an opt-in feature:

```toml
waybar_gpui = { version = "0.1.0", features = ["components"] }
```

(The starter template asks "Use gpui-component widgets?" and writes this for
you.) It's then available as `waybar_gpui::components`; don't depend on
`gpui-component` yourself, for the same reason as GPUI.

```rust
use waybar_gpui::components::{button::{Button, ButtonVariants as _}, switch::Switch};

card(cx)
    .child(Switch::new("wifi").checked(self.wifi).on_click(…))
    .child(Button::new("done").primary().label("Done").on_click(|_, _, cx| close(cx)))
```

Full example with switches and a dropdown:
[`controls.rs`](../../crates/example-panels/src/controls.rs).

## What the runtime does for you

- Initialises the library and registers its icon assets.
- Copies your [`Theme`](theming.md) into gpui-component's theme at startup, so
  widgets match your cards. Adjust gpui-component's theme in code afterwards
  if you want finer control.

## What works, what doesn't

- **Works:** widgets, menus and dropdowns, in-window popovers. A menu may
  overflow the card; clicking it doesn't dismiss the popover.
- **Doesn't:** gpui-component **dialogs and notifications**. They need its
  `Root` view, which on Linux draws a window border and shadow around layer
  surfaces and veils the whole monitor, so the runtime doesn't use it. Build a
  "dialog" as a second view inside your panel instead.

## Costs

About 110 more crates to build and ~10 MB more binary. Programs without the
feature pay nothing.

## Versions

gpui-component is pinned exactly, together with GPUI: the three pins always
move together, and each move is a breaking SDK release (see
[Limits](limits.md#versions)).
