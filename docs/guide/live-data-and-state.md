# Live data and state

The SDK adds nothing here: it's plain GPUI. Two facts shape everything:

- A panel is **built when its popover opens and dropped when it closes**.
- Your program's **daemon stays running** between opens, until `quit` or logout.

So: work tied to one open belongs to the panel; anything that must outlive a
close belongs to the daemon, in a GPUI `Global`.

## Updating while open

Store the `Task` in a field. Dropping the panel drops the task, which stops it;
there's nothing to clean up.

**From a timer**, as in [`clock.rs`](../../examples/example-panels/src/clock.rs):

```rust
pub struct ClockPanel {
    _tick: Task<()>,
    …
}

let _tick = cx.spawn(async move |this, cx| loop {
    cx.background_executor().timer(Duration::from_secs(1)).await;
    if this.update(cx, |_, cx| cx.notify()).is_err() {
        return; // the panel is gone
    }
});
```

**From a blocking source** (a socket, a pipe, a D-Bus call), as in
[`workspace.rs`](../../examples/example-panels/src/workspace.rs), which follows
Hyprland's event socket: block on a background thread, update on the
foreground one. (`hypr` is the example's own small Hyprland client,
[`hypr.rs`](../../examples/example-panels/src/hypr.rs); the SDK doesn't provide
one, so bring your own or a crate.)

```rust
let _events = cx.spawn(async move |this, cx| {
    let Ok(mut events) = hypr::Events::connect() else { return };
    loop {
        let (back, event) = cx
            .background_executor()
            .spawn(async move { let event = events.next(); (events, event) })
            .await;
        events = back;
        let Some(Ok(event)) = event else { return };
        if this.update(cx, |panel, cx| { panel.refresh(); cx.notify(); }).is_err() {
            return;
        }
    }
});
```

After the popover closes, a background read that is already waiting keeps its
thread until the next event arrives; then the update fails and the loop ends.
That's harmless for event streams that tick often. For sources that can stay
silent for a long time, prefer a timeout or a non-blocking API.

## State that survives a close

Put it in a GPUI `Global`. It lives in the daemon, so it's there on the next
open, until the daemon quits (or is replaced by a rebuilt binary, see
[Dev loop](dev-loop.md)).

```rust
#[derive(Default)]
struct Opens(u32);
impl Global for Opens {}

// in Panel::new
let opens = {
    let opens = cx.default_global::<Opens>();
    opens.0 += 1;
    opens.0
};
```

[`clock.rs`](../../examples/example-panels/src/clock.rs) counts its opens this
way. For anything that must survive the daemon too (settings, history), write
it to a file yourself.
