# Contributing

Thanks for taking the time to contribute to `waybar_gpui`. Bug reports,
feature requests and pull requests are all welcome. Everyone taking part is
expected to follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## Reporting issues

Open an issue using one of the [templates][new-issue]. For a security
vulnerability, do **not** open a public issue; follow
[SECURITY.md](SECURITY.md) instead.

## Before you change anything

- [`GLOSSARY.md`](../GLOSSARY.md): the terms used in code, docs and issues
  (a panel is never a "module", an executable is a "program"); please stick to
  them.
- [`architecture.md`](architecture.md): how the runtime works and why, what
  was verified, and the designs that were rejected. Read it before changing
  `crates/waybar_gpui`.
- [`spec.md`](spec.md): what is being built, with the tickets behind each
  decision.
- [`adr/`](adr/): the decisions that are hard to reverse.
- [The guide](guide/index.md): what authors are told. The README's
  [repository table](../README.md#this-repository) says what each crate is.

## Setup

- Rust 1.97.1, the version CI tests with, with clippy and rustfmt.
- The system libraries GPUI's Wayland platform links against. On Debian or
  Ubuntu, as CI installs them:

  ```sh
  sudo apt-get install pkg-config libxkbcommon-dev libwayland-dev \
    libvulkan-dev libfontconfig-dev libfreetype-dev
  ```

  Other distributions: the same libraries under their own package names.
- [cargo-generate](https://github.com/cargo-generate/cargo-generate)
  (`cargo install cargo-generate`), for `make check-template`.
- [prek](https://github.com/j178/prek), for the [Git hooks](#git-hooks).

## Git hooks

The repository ships Git hooks in
[`.pre-commit-config.yaml`](../.pre-commit-config.yaml), run by prek. Install
it (e.g. `cargo install --locked prek`; see its README for other methods), then
enable the hooks once per clone:

```sh
prek install
```

- **pre-commit**: trailing whitespace, final newlines, line endings, YAML/TOML
  syntax, merge-conflict markers, `cargo fmt --all`, and `cargo clippy` without
  and with `--all-features`. Fixers rewrite the files and fail the commit;
  review the changes, `git add` them and commit again.
- **pre-push**: `make test`.

`make check-template` and the link check aren't hooks (slow, and they need
cargo-generate and lychee); CI runs them. Run every hook on the whole tree with
`prek run --all-files`. Skip them for one commit with `git commit --no-verify`;
CI runs the same checks anyway.

## Checks

Before opening a pull request, run what CI runs:

```sh
make lint             # cargo fmt --all --check, clippy without and with
                      # --all-features, cargo doc -D warnings
make test             # unit tests and doctests, without and with --all-features
make check-template   # every versioned SDK reference matches the workspace version;
                      # both template variants generate and pass clippy
```

CI also checks Markdown links offline with
[lychee](https://github.com/lycheeverse/lychee):
`lychee --offline --exclude-path target './**/*.md'`.

## Trying it on screen

`make dev` needs Hyprland 0.56 (Lua config) and Waybar 0.15. It builds with
the `components` feature and starts a second Waybar, with a top and a left bar
wired to `waybar-hypr-ws` and `example-panels`, on your screen next to your
own; Ctrl-C stops it. Clicking its bar modules opens the example panels as
popovers. To open one panel without a bar, `cargo run -p example-panels --
preview <panel>`.

The daemon started by a click logs to
`~/.local/state/waybar-gpui/example-panels.log`
(`$XDG_STATE_HOME/waybar-gpui/<id>.log`); `preview` logs to the terminal.
More in the guide's [Dev loop](guide/dev-loop.md#logs).

## Pull requests

- Branch from `main` and open a pull request against `main`. CI must pass
  before merging.
- One pull request per issue; say which one in the description with
  `Closes #<n>`.
- A behaviour change updates the one doc that owns the fact (guide, rustdoc,
  architecture or spec), in the same pull request.

## Releases

Releases are cut by the maintainer: bump `version` in `[workspace.package]` of
the top-level `Cargo.toml` and every versioned SDK reference with it
(`make check-template` lists any left behind), merge, then tag `vX.Y.Z` on
`main` and push the tag. The gate and order for v0.1.0 are in
[spec §13](spec.md#13-release-v010).

## License

By contributing, you agree that your contributions are licensed under the
[MIT License](../LICENSE).

[new-issue]: https://github.com/Cerclique/waybar_gpui_plugin_template/issues/new/choose
