# {{project-name}}

Waybar popovers on Hyprland, built with
[waybar_gpui](https://github.com/Cerclique/waybar-gpui).

```sh
cargo run -- preview hello            # try the panel, no Waybar needed
cargo install --path . --locked       # install into ~/.cargo/bin
{{project-name}}                      # prints the on-click line for each panel
```

Commit the `Cargo.lock` created by your first build: `cargo install --locked`
uses it.
