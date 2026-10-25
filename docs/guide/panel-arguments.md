# Panel arguments

The words after the panel's name in `on-click` are its **panel arguments**,
handed to `Panel::new` as `&[String]`:

```jsonc
"custom/clock-paris": { "on-click": "~/.cargo/bin/my-panels toggle clock Paris 1", … },
"custom/clock-tokyo": { "on-click": "~/.cargo/bin/my-panels toggle clock Tokyo 9", … },
```

```rust
impl Panel for ClockPanel {
    const NAME: &'static str = "clock";

    fn new(args: &[String], window: &mut Window, cx: &mut Context<Self>) -> Self {
        let label = args.first().cloned().unwrap_or_else(|| "UTC".into());
        let offset_hours = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
        …
    }
}
```

Full example: [`clock.rs`](../../crates/example-panels/src/clock.rs).

- **You parse them.** The SDK passes plain strings; decide your own defaults
  and what to do with bad input (fall back, or show an error in the card).
- **Toggle compares them.** A click closes the popover if the *same panel with
  the same arguments* is open; otherwise it opens the panel with the new
  arguments. So "Paris" and "Tokyo" are two different popovers of one panel.
- **Words, not a shell string.** Waybar runs `on-click` through `/bin/sh -c`,
  so quoting works as in a shell (`toggle note "two words"`). Arguments can't
  contain tabs or newlines.
- **`preview` takes them too:** `cargo run -- preview clock Tokyo 9`.
