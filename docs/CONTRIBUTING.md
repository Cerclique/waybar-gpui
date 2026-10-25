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

- Rust 1.97.1, the version CI tests with, with clippy and rustfmt. The
  minimum supported Rust version (MSRV) is the `rust-version` in
  [`Cargo.toml`](../Cargo.toml), 1.95 (GPUI needs it); don't use language or
  library features newer than it. CI checks it builds.
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
- [cargo-deny](https://github.com/EmbarkStudios/cargo-deny)
  (`cargo install --locked cargo-deny`), for `cargo deny check`.

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
cargo deny check      # advisories, licenses, bans, sources (deny.toml)
```

Cargo commands run with `--locked`: commit `Cargo.lock` with any dependency
change. Lints are set in `[workspace.lints]` of the top-level `Cargo.toml`
(`unsafe_code` is forbidden) and every crate opts in with
`[lints] workspace = true`; the SDK also denies missing docs.

CI also checks Markdown links offline with
[lychee](https://github.com/lycheeverse/lychee):
`lychee --offline --exclude-path target './**/*.md'`.

## CI

The workflows live in [`.github/workflows/`](../.github/workflows/):

| Workflow | Runs on | What it does |
| --- | --- | --- |
| `ci.yml` | PRs, pushes to `main` | `rust`: fmt, build, tests, clippy and docs on Rust 1.97.1, as `make lint` and `make test`; `msrv`: `cargo check` on the `rust-version` from `Cargo.toml`; `deny`: `cargo deny check --all-features`; `template`: `make check-template`; `links`: the lychee link check |
| `pr-title.yml` | PRs (opened, edited, pushed to) | `conventional`: the PR title is a conventional commit |
| `release.yml` | `v*` tag push | Creates the GitHub Release from the tag's `CHANGELOG.md` section |

A new push to a PR cancels its running `ci.yml` run. Jobs time out after 45
minutes (build jobs, which build all of GPUI on a cold cache) or less.
Actions are pinned to a commit, with the release in a comment.

[Dependabot](../.github/dependabot.yml) opens one grouped PR a week for Cargo
dependencies (`build(deps): …`, listed in the changelog) and one for actions
(`ci(deps): …`). It leaves the exactly pinned GPUI crates (`gpui-pre`,
`gpui-pre-platform`, `gpui-component`, `gpui-kit-assets`) and the starter
template alone: those move by hand, as a breaking release.

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
- Pull requests are **squash-merged**: the PR title becomes the commit title on
  `main`, and the changelog is generated from those titles. The PR title must
  follow [Conventional Commits](https://www.conventionalcommits.org/), which
  CI checks (`PR title` workflow):

  ```text
  <type>[(scope)][!]: <description>
  ```

  Types: `feat`, `fix`, `perf`, `refactor`, `docs`, `build` (shown in the
  changelog as Added, Fixed, Performance, Changed, Documentation, Build), and
  `ci`, `test`, `chore`, `style` (left out of it). Mark a breaking change with
  `!` (e.g. `feat(runtime)!: …`); it is always listed, whatever its type.
- Don't edit `CHANGELOG.md` by hand: it is generated by
  [git-cliff](https://git-cliff.org) ([`cliff.toml`](../cliff.toml)) when a
  release is cut. The 0.1.0 entry is the one exception, written by hand
  because the history before it isn't conventional.

## Releases

Releases are cut by the maintainer, through a pull request. You need
[git-cliff](https://git-cliff.org) (e.g. `cargo install --locked git-cliff`).

1. On a `release/vX.Y.Z` branch from `main`, bump `version` in
   `[workspace.package]` of the top-level `Cargo.toml` (`cargo build` updates
   `Cargo.lock`) and every versioned SDK reference with it: the template's
   `tag`, the README's `--tag`, tagged guide links. `make check-template` lists
   any left behind.
2. Add the release's section to the changelog:

   ```sh
   git cliff --unreleased --tag vX.Y.Z --prepend CHANGELOG.md
   ```

   It renders the commits since the last tag and puts them on top, leaving
   the rest of the file alone. Never regenerate the whole file
   (`git cliff -o CHANGELOG.md`): that drops the hand-written 0.1.0 entry.
3. Commit as `chore: release vX.Y.Z` and open a pull request with that title.
4. Once it is merged, tag the merge commit on `main` and push the tag:

   ```sh
   git switch main && git pull
   git tag -a vX.Y.Z -m "Release vX.Y.Z"
   git push origin vX.Y.Z
   ```

   Pushing the tag runs the `Release` workflow: it creates the GitHub Release
   with the tag's section of `CHANGELOG.md` as notes, and fails if that
   section is missing or undated.

v0.1.0 differs: the version is already 0.1.0 and its entry is hand-written,
so its release pull request only replaces `Unreleased` in the entry's heading
with the date. Its gate and order are in [spec §13](spec.md#13-release-v010).

## License

By contributing, you agree that your contributions are licensed under the
[MIT License](../LICENSE).

[new-issue]: https://github.com/Cerclique/waybar_gpui_plugin_template/issues/new/choose
