# Closing and Esc

## How a popover closes

| What happens | Who decides |
|---|---|
| A click on the **backdrop** (anywhere outside the card on that monitor, the bar included) | Always closes; a panel can't prevent it. |
| A click on another monitor | Always closes. |
| Esc | The panel first, if it has focus (below). Otherwise closes. |
| The panel calls `close(cx)` | The panel. |
| A click on the bar, the same bar module included | Closes, as a backdrop click (see the note below). |
| Another popover opens (any program) | The old one closes first: one popover at a time. |

Note: while a popover is open, Hyprland sends **every** click to it, so Waybar
can't be clicked. Clicking the bar (the same bar module or another) is a click
on the backdrop: it only closes the popover. A second click opens the new
panel. See [How it works](how-it-works.md).

## Closing from the panel

Call `close(cx)` from any event handler, typically after doing the thing the
user clicked:

```rust
div()
    .id("row")
    .on_click(move |_, _, cx| {
        std::process::Command::new("hyprctl").args(["dispatch", "…"]).spawn().ok();
        close(cx);
    })
```

The close is deferred until the current event finishes, so it's safe inside
the panel's own handlers.

**Actions are yours.** The SDK only provides what needs the runtime (closing
the popover). Running commands, Hyprland dispatches, D-Bus calls are your own
code: `std::process::Command`, a client crate, anything.

## Esc

Esc goes to the focused element first and bubbles up to the runtime, which
closes the popover. To handle keys yourself:

1. Focus one of your elements in `Panel::new` (otherwise nothing of yours has
   focus and Esc goes straight to the runtime).
2. Handle the key; call `cx.stop_propagation()` to keep the popover open.

```rust
fn new(_args: &[String], window: &mut Window, cx: &mut Context<Self>) -> Self {
    let focus = cx.focus_handle();
    window.focus(&focus, cx);
    Self { focus, editing: false }
}

// in render
card(cx)
    .track_focus(&self.focus)
    .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
        if event.keystroke.key == "escape" && this.editing {
            this.editing = false;
            cx.notify();
            cx.stop_propagation(); // first Esc leaves edit mode, the popover stays
        }
    }))
```

[`clock.rs`](../../crates/example-panels/src/clock.rs) does this with an edit
mode (`e` enters it, the first Esc leaves it, the second closes).

Your panel can't stop a backdrop click: with Waybar unclickable while a
popover is open, that guarantees the user can always get out.
