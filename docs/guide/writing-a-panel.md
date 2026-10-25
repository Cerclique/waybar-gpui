# Writing a panel

A **panel** is a GPUI view. Your **program** registers one or more panels and
hands control to the SDK, which turns a click on a bar module into a
**popover** showing the panel.

## The smallest program

```rust
use waybar_gpui::{card, gpui::{prelude::*, Context, Window}, Panel};

struct Hello;

impl Panel for Hello {
    const NAME: &'static str = "hello";
    fn new(_args: &[String], _window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Hello
    }
}

impl Render for Hello {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        card(cx).child("Hello")
    }
}

fn main() -> std::process::ExitCode {
    waybar_gpui::App::new("my-panels").panel::<Hello>().run()
}
```

The starter template generates exactly this (see the
[README](../../README.md#quickstart)).

- **`Panel::NAME`** is what a bar module's `on-click` names:
  `my-panels toggle hello`.
- **`Panel::new`** builds the panel each time its popover opens. The panel is
  dropped when the popover closes; nothing is kept between opens (see
  [Live data and state](live-data-and-state.md) for what to do instead).
  `args` are the [panel arguments](panel-arguments.md).
- **`Render`** is plain GPUI. `card(cx)` gives the standard rounded, bordered,
  themed rectangle; it's optional ([Theming](theming.md)).
- **`App::new(id)`** names your program. The **program ID** names its socket,
  its log file and its Hyprland layer namespace, so pick it once and keep it.
  It must match `^[a-z0-9][a-z0-9-]*$`; anything else panics at start.
- **`.panel::<P>()`** registers a panel; two panels with the same `NAME` panic.
- **`.run()`** owns the command line: `toggle`, `close`, `quit`, `preview`,
  `daemon`, `help`. Run your program with no arguments to see the usage and an
  `on-click` line per panel. If you want commands of your own, handle them in
  `main` before calling `run`.

## GPUI

The SDK re-exports GPUI as `waybar_gpui::gpui`; don't add `gpui` (or
`gpui-pre`) to your own `Cargo.toml`, or you'll get two copies that don't fit
together. GPUI is pre-1.0 and the SDK pins an exact snapshot: the
[SDK's API](https://docs.rs/waybar_gpui) and
[GPUI's](https://docs.rs/gpui-pre/0.3.9) are on docs.rs, and
`cargo doc --open` in your project shows both offline.

GPUI's derive macros (`#[derive(IntoElement)]` and friends) expand to paths
starting with `gpui::`. In any module that uses them, bring the name into
scope:

```rust
use waybar_gpui::gpui;
```

[`workspace.rs`](../../examples/example-panels/src/workspace.rs) in the example
panels shows a `#[derive(IntoElement)]` row.

## Size

Don't declare a size. The **card** fits its content and is placed next to the
clicked bar module, centred on the cursor. Cap it yourself if it can grow, and
scroll the overflow:

```rust
card(cx)
    .w(gpui::px(340.))
    .max_h(gpui::px(420.))
    .child(div().id("rows").flex().flex_col().overflow_y_scroll().children(rows))
```

The runtime also keeps the card inside the monitor's usable area; past that,
scrolling is up to you. When content grows while the popover is open, the card
is re-placed on the next frame.

## Several panels

One program can hold many panels:

```rust
waybar_gpui::App::new("my-panels")
    .panel::<Clock>()
    .panel::<Volume>()
    .run()
```

Only one popover is open at a time, across all programs: opening one closes
the other.

## Next

- Try it without Waybar: [Dev loop](dev-loop.md).
- Patterns, each shown in one runnable example:
  [`examples/example-panels`](../../examples/example-panels/src/main.rs).
