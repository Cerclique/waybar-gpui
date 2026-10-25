# Published on crates.io

`waybar_gpui` is published on crates.io, and authors depend on it with `waybar_gpui = "X.Y.Z"`. Each release is still tagged `vX.Y.Z`, and the starter template is generated from that tag. The plan was a git dependency at a tag, on the grounds that GPUI isn't stable enough to publish against ([#18](https://github.com/Cerclique/waybar-gpui/issues/18), [#20](https://github.com/Cerclique/waybar-gpui/issues/20)). That reason doesn't hold: the GPUI snapshot we pin (`gpui-pre`) and gpui-component are crates.io packages themselves, and docs.rs builds them. Publishing gives authors a normal dependency line, semver resolution, API docs on docs.rs and a crates.io baseline for semver checks ([#85](https://github.com/Cerclique/waybar-gpui/issues/85)).

## Consequences

- Only crates.io dependencies are allowed, so the Hyprland IPC lives inside the SDK as a private module, not as a workspace crate. `example-panels` has its own small client, as an author would.
- The exact pins stay (`gpui-pre`, `gpui-pre-platform`, `gpui-component`), and any change to them is a breaking release. A program depending on the SDK can't use another GPUI snapshot alongside it.
- Publishing can't be undone: the name is taken for good, and a release can be yanked but not deleted. The release process publishes right after the tag, from the tagged commit.
