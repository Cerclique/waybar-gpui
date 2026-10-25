# Troubleshooting

Start with the log: `~/.local/state/waybar-gpui/<id>.log` for a daemon started
by a click, the terminal for `preview`.

| Symptom | Likely cause | Fix |
|---|---|---|
| Clicking the bar module does nothing; no log file | Waybar can't find the program. | Use the full path in `on-click` ([Installing](installing.md#waybar)); run the same command in a terminal to see the error. |
| Clicking does nothing; the log shows `cannot open` | Hyprland unreachable (`HYPRLAND_INSTANCE_SIGNATURE` not set for Waybar), or no monitor found. | Start Waybar from Hyprland (`exec-once`); check `hyprctl monitors`. |
| `unknown panel "x"` | `on-click` names a panel your program doesn't register. | Run the program with no arguments for the list. |
| The popover shows an old version | An old daemon. | Click again: a click replaces a daemon whose binary changed. Or `<program> quit`. |
| The popover vanished and the log ends with `panicked at` | A panic in the panel took the daemon down. | Fix the panic; the next click starts a fresh daemon. |
| A click on another bar module only closed the popover | Expected: Waybar can't be clicked while a popover is open. | Click again. |
| Esc doesn't reach my panel's key handler | Nothing of yours has focus. | Focus an element in `Panel::new` ([Closing and Esc](closing-and-esc.md#esc)). |
| Errors about `gpui::…` paths from a derive macro | `gpui` isn't in scope in that module. | `use waybar_gpui::gpui;` ([Writing a panel](writing-a-panel.md#gpui)). |
| Two versions of `gpui` in the dependency tree, types don't match | You depend on `gpui`/`gpui-pre` or `gpui-component` directly. | Remove them; use `waybar_gpui::gpui` and the `components` feature. |
| A dialog or notification from gpui-component doesn't show | Unsupported without `Root`. | Build it as a view inside your panel ([Components](components.md)). |
| `invalid program ID` panic at start | The ID passed to `App::new` has uppercase letters, `_`, spaces… | Use `a-z`, `0-9` and `-`, not starting with `-`. |
| The program keeps running after `cargo uninstall` | The daemon outlives its deleted binary. | `<program> quit` (or log out). |
| The first popover after login is slow | The first click starts the daemon. | Expected; later clicks are fast. |
