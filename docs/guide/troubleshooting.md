# Troubleshooting

Start with the log: `~/.local/state/waybar-gpui/<id>.log` for a daemon started
by a click, the terminal for `preview`.

| Symptom | Likely cause | Fix |
|---|---|---|
| Clicking the bar module does nothing; no log file | Waybar can't find the program. | Use the full path in `on-click` ([Installing](installing.md#waybar)); run the same command in a terminal to see the error. |
| Clicking does nothing; the log shows `cannot open` | Hyprland unreachable (`HYPRLAND_INSTANCE_SIGNATURE` not set for Waybar), or no monitor found. | Start Waybar from Hyprland (`exec-once`); check `hyprctl monitors`. |
| `unknown panel "x"` | `on-click` names a panel your program doesn't register. | Run the program with no arguments for the list. |
| The popover shows an old version | An old daemon. | Click again: a click replaces a daemon whose binary changed. Or `<program> quit`. |
| `daemon` run by hand exits with `<id>: daemon already running` | A daemon already runs; a hand-run `daemon` doesn't replace it. | `<program> quit` first. |
| The popover vanished and the log ends with `panicked at` | A panic in the panel took the daemon down. | Fix the panic; the next click starts a fresh daemon. |
| A click on another bar module only closed the popover | Expected: Waybar can't be clicked while a popover is open. | Click again. |
| Esc doesn't reach my panel's key handler | Nothing of yours has focus. | Focus an element in `Panel::new` ([Closing and Esc](closing-and-esc.md#esc)). |
| Errors about `gpui::…` paths from a derive macro | `gpui` isn't in scope in that module. | `use waybar_gpui::gpui;` ([Writing a panel](writing-a-panel.md#gpui)). |
| Two versions of `gpui` in the dependency tree, types don't match | You depend on `gpui`/`gpui-pre` or `gpui-component` directly. | Remove them; use `waybar_gpui::gpui` and the `components` feature. |
| A dialog or notification from gpui-component doesn't show | Unsupported without `Root`. | Build it as a view inside your panel ([Components](components.md)). |
| `invalid program ID` panic at start | The ID passed to `Program::new` has uppercase letters, `_`, spaces… | Use `a-z`, `0-9` and `-`, not starting with `-`. |
| The program keeps running after `cargo uninstall` | The daemon outlives its deleted binary. | `<program> quit` (or log out). |
| The first popover after login is slow | The first click starts the daemon. | Expected; later clicks are fast. |

## Investigating

For a problem the table doesn't list, start with what the runtime logs. Where
the log is depends on how the daemon started: see
[Dev loop](dev-loop.md#logs).

- **Read the runtime's lines first.** Each open logs
  `<id>: opened <panel> on <monitor> (<edge> bar)`, each close
  `<id>: closed <panel> (<reason>)`, and a failed open
  `<id>: cannot open <panel>: <error>` (a panel's arguments follow its name,
  e.g. `clock ["Paris"]`). They tell you whether the popover opened, on which
  monitor, and why it closed.
- **Get the output in the terminal.** `<program> preview <panel>` opens the
  panel at your cursor and logs to the terminal, without Waybar.
- **Run the daemon in the foreground.** `<program> quit`, then
  `<program> daemon <panel>`: the daemon logs to the terminal, and clicks on
  the bar module reach it if its `on-click` runs the same executable (another
  one replaces it). Run `quit` first: while a daemon runs, a hand-run `daemon`
  exits with `<id>: daemon already running`.
- **Tell Hyprland from GPUI.** While the popover is open,
  `hyprctl layers` lists its surface as `namespace: waybar-gpui:<id>` and a
  backdrop on each other monitor as `namespace: waybar-gpui-backdrop:<id>`.
  A surface that is listed but shows nothing points at drawing (GPUI); no
  surface points at the open itself (look for `cannot open` in the log).
  `hyprctl monitors` and
  `hyprctl cursorpos` show what placement used: the popover opens on the
  monitor under the cursor, next to its bar.

## Reporting a bug

Open an issue with the
[bug report](https://github.com/Cerclique/waybar-gpui/issues/new?template=bug_report.yml)
template. It asks for what happened, the steps to reproduce, the relevant log
lines, your `waybar_gpui`, Rust, Hyprland and Waybar versions, whether the
`components` feature is on, and your distribution and GPU.

Tip: reproduce with `<program> preview <panel>` if you can. The output lands
in the terminal, ready to paste.
