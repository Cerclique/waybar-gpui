# Dev loop

## Preview, no Waybar needed

```sh
cargo run -- preview hello            # or: preview clock Tokyo 9
```

`preview` opens the panel at your cursor, logs to the terminal, and **exits
when the popover closes** (Esc, a click outside, or `close(cx)`). It doesn't
touch your program's running daemon, but it does close any open popover first.
It has no socket, so the reverse doesn't hold: a `toggle` from a keybind or
script, or another program opening its popover, can't close a preview, and you
get two popovers. (Clicks still close it, like any popover.)
Exit status: 0 after a normal close, 1 if the panel couldn't open, 2 for a bad
command line.

To rebuild and reopen on save, let a file watcher restart the preview, for
example [watchexec](https://github.com/watchexec/watchexec) or
[cargo-watch](https://github.com/watchexec/cargo-watch):

```sh
watchexec -r -e rs -- cargo run -- preview hello
cargo watch -x 'run -- preview hello'
```

## On your real bar

Point a bar module at your build directly:

```jsonc
"custom/hello": {
  "exec": "echo hello",
  "interval": "once",
  "on-click": "/path/to/your-repo/target/debug/my-panels toggle hello",
},
```

Then just `cargo build` and click: a daemon started from an older binary
**replaces itself** on the next click, so you never need to stop it by hand.
This also works between a `target/debug` build and an installed copy with the
same program ID: whichever one a click runs takes over. `my-panels quit`
remains if you want the daemon gone.

## Logs

A daemon started by a click logs to
`~/.local/state/waybar-gpui/<id>.log` (`$XDG_STATE_HOME/waybar-gpui/<id>.log`),
emptied each time a click starts a new daemon:

```sh
tail -f ~/.local/state/waybar-gpui/my-panels.log
```

Everything written to stderr lands there: the runtime's lines
(`my-panels: opened hello on DP-1 (Top bar)`,
`my-panels: closed hello (escape)`), your `eprintln!`, any logger you set
up (`env_logger`, `tracing-subscriber`), and panic messages. `preview`, and
`daemon` run by hand, log to the terminal instead. The SDK has no logging
dependency; use whatever you like.

## Panics

A panic in your panel takes the daemon down: the popover disappears and the
panic message is in the log. The next click starts a fresh daemon. Nothing
else to do.

## Example panels

This repository's [`crates/example-panels`](../../crates/example-panels/src/main.rs)
is a program built the way yours is, with one panel per pattern. Clone the
repo and run `make dev` to get a second Waybar wired to them, or
`cargo run -p example-panels -- preview clock Paris 1`.
