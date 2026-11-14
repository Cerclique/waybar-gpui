# Contributing

Thanks for taking the time to contribute to `waybar_gpui`. Bug reports,
feature requests and pull requests are all welcome. Everyone taking part is
expected to follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## Reporting issues

Open an issue using one of the [templates][new-issue]. For a security
vulnerability, do **not** open a public issue; follow
[SECURITY.md](SECURITY.md) instead.

## Before you change anything

- [`GLOSSARY.md`](GLOSSARY.md): the terms used in code, docs and issues
  (a panel is never a "module", an executable is a "program", the project is
  waybar-gpui and only the crate is `waybar_gpui`); please stick to them.
- [`architecture.md`](architecture.md): how the runtime works and why, what
  was verified, and the designs that were rejected. Read it before changing
  `crates/waybar_gpui`.
- [`adr/`](adr/): the decisions that are hard to reverse.
- [The guide](guide/index.md): what authors are told. The README's
  [repository table](../README.md#this-repository) says what each crate is.

## Setup

- [rustup](https://rustup.rs). The Rust version the project is tested with,
  and its clippy and rustfmt, are declared once, in
  [`rust-toolchain.toml`](../rust-toolchain.toml): rustup uses it in the
  repository whatever your default, and installs it on first use. CI builds
  with the same file. Dependabot proposes each new stable release as a
  `chore(deps)` PR; to bump it yourself, edit `channel` there: that one edit
  moves CI and local builds together. The minimum supported Rust version
  (MSRV) is the `rust-version` in [`Cargo.toml`](../Cargo.toml), 1.95 (GPUI
  needs it); don't use language or library features newer than it. CI checks
  it builds.
- The system libraries GPUI's Wayland platform links against. On Debian or
  Ubuntu, as CI installs them
  ([`gpui-system-packages`](../.github/actions/gpui-system-packages/action.yml)):

  ```sh
  sudo apt-get install pkg-config libxkbcommon-dev libwayland-dev \
    libvulkan-dev libfontconfig-dev libfreetype-dev
  ```

  Other distributions: the same libraries under their own package names.
- [cargo-generate](https://github.com/cargo-generate/cargo-generate)
  (`cargo install cargo-generate`), for `make check-template`.
- [prek](https://github.com/j178/prek), for the [Git hooks](#git-hooks). It
  installs the [actionlint](https://github.com/rhysd/actionlint) (built with
  Go), [zizmor](https://docs.zizmor.sh) and
  [typos](https://github.com/crate-ci/typos) (both from PyPI) hooks itself,
  downloading Go or Python if missing.
  [ShellCheck](https://www.shellcheck.net) (e.g. your distribution's
  `shellcheck` package) lets actionlint check `run:` scripts, as CI does.
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
  syntax, merge-conflict markers, typos (spell check; a real term it flags goes
  in [`_typos.toml`](../_typos.toml)), `cargo fmt --all`, `cargo clippy` without
  and with `--all-features`, and, when `.github/` workflows or actions change,
  actionlint and zizmor (offline audits only, unless `GH_TOKEN` is set). Fixers rewrite the files and fail the commit;
  review the changes, `git add` them and commit again.
- **pre-push**: `make test`.

`make check-template` and the link check aren't hooks (slow, and they need
cargo-generate and lychee); CI runs them. Run every hook on the whole tree with
`prek run --all-files`. Skip them for one commit with `git commit --no-verify`;
CI runs the same checks anyway.

## Editing the guide

The author guide, [`docs/guide/`](guide/index.md), is also published as an
[mdbook](https://rust-lang.github.io/mdBook/) site,
<https://cerclique.github.io/waybar-gpui/>: the latest release at the root,
`main` at `/dev/`. Its configuration is [`docs/book.toml`](book.toml); its
table of contents, [`docs/guide/SUMMARY.md`](guide/SUMMARY.md), lists every
page, so a new page goes there too. The glossary enters the book through
`docs/guide/glossary.md`, which includes [`GLOSSARY.md`](GLOSSARY.md): edit
the latter.

- Link from a page to the rest of the repository with ordinary relative links
  (`../../examples/…`): they work on GitHub, and on the site the book's
  preprocessor, [`scripts/book-preprocessor.py`](../scripts/book-preprocessor.py),
  turns them into GitHub links at the version being built.
- Rust code goes in the snippets crate, `docs/guide/snippets/`, one module
  per page (`src/<page>.rs`), between `// ANCHOR: <name>` and
  `// ANCHOR_END: <name>` comments; the page shows it with
  `{{#include snippets/src/<page>.rs:<name>}}` inside a `rust` code block.
  The crate is a workspace member, so `cargo build`, `make lint` and CI
  compile it and the snippets keep up with the SDK and GPUI; its
  `components` feature (on with `--all-features`) holds the Components page.
  `cargo fmt` skips it: lay the snippets out by hand for the page. The site
  strips the indentation of a snippet taken from inside a function.
- Preview with live reload: `mdbook serve docs`
  ([mdbook](https://rust-lang.github.io/mdBook/guide/installation.html), and
  `python3` for that script).
- `make book` builds the site into `docs/book/` and checks its links and
  anchors with lychee, as CI's `book` job does.

## Checks

Before opening a pull request, run what CI runs:

```sh
make lint             # cargo fmt --all --check, clippy without and with
                      # --all-features, cargo doc -D warnings
make test             # unit tests and doctests, without and with --all-features
make check-template   # every versioned SDK reference matches the workspace version;
                      # both template variants generate and pass clippy
make book             # the guide's site builds and its links resolve
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
| `ci.yml` | PRs, pushes to `main` | `rust`: fmt, build, tests, clippy and docs on the toolchain in `rust-toolchain.toml`, as `make lint` and `make test`; `msrv`: `cargo check` on the `rust-version` from `Cargo.toml`; `deny`: `cargo deny check --all-features`, then `cargo machete` for unused dependencies; `template`: `make check-template`; `links`: the lychee link check; `book`: the guide's site, as `make book`; `typos`: the spell check (`_typos.toml`); `workflows`: actionlint and zizmor over `.github/` (the workflows, the local action and Dependabot's config), failing on any finding; `semver` (PRs only): `cargo semver-checks` against the latest release on crates.io |
| `pr-title.yml` | PRs (opened, edited, pushed to) | `conventional`: the PR title is a conventional commit |
| `release.yml` | `v*` tag push | Publishes `waybar_gpui` on crates.io, then creates the GitHub Release from the tag's `CHANGELOG.md` section, then deploys the release's guide (`book.yml`) |
| `book.yml` | Pushes to `main` touching `docs/`, release.yml, or by hand | `deploy`: builds the guide's site and commits it to the `gh-pages` branch, `main` to `/dev/`, a release (`latest`) to the root |
| `scheduled.yml` | Mondays, or by hand | `advisories`: `cargo deny check advisories` on the locked dependencies; `beta`: tests and clippy on Rust beta |

Every job in `ci.yml` and `pr-title.yml` is required to merge, except
`semver`: a red `semver` means the PR breaks the SDK's API, so the next
release bumps the minor (0.x semver). A new push to a PR cancels its running
`ci.yml` run. Jobs time out after 45 minutes (build jobs, which build all of
GPUI on a cold cache) or less. Actions are pinned to a commit, with the
release in a comment. A zizmor finding is fixed where it can be; a deliberate
exception is ignored inline (`# zizmor: ignore[<audit>]`) or in
[`.github/zizmor.yml`](../.github/zizmor.yml), with a comment saying why it is
safe.

GitHub Pages serves the `gh-pages` branch (Settings → Pages: deploy from a
branch, `gh-pages`, `/ (root)`). Each deploy commits only its own folder, so
`/dev/` and the release at the root don't overwrite each other.

[Dependabot](../.github/dependabot.yml) opens one grouped PR a week for Cargo
dependencies (`build(deps): …`, listed in the changelog) and one for actions
(`ci(deps): …`), each proposing only releases at least a week old. It leaves the exactly pinned GPUI crates (`gpui-pre`,
`gpui-pre-platform`, `gpui-component`, `gpui-kit-assets`) and the starter
template alone: those move by hand, as a breaking release.

## Trying it on screen

`make dev` needs Hyprland 0.57 (Lua config) and Waybar 0.15. It builds with
the `components` feature and starts a second Waybar, with a top and a left bar
wired to `example-panels`, on your screen next to your
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
- A behaviour change updates the one doc that owns the fact (guide, rustdoc
  or architecture), in the same pull request.
- Renaming a guide page or a command-line verb also updates the authors'
  agent skill, [`skills/waybar-gpui/SKILL.md`](../skills/waybar-gpui/SKILL.md),
  which names them. CI checks the pages it names exist.
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

Tags are plain `vX.Y.Z` and always equal the single `version` in
`[workspace.package]`, which every crate inherits. 0.x semver: a breaking
change to the SDK bumps the minor, a fix the patch. Any change to the
`gpui-pre`, `gpui-pre-platform` or `gpui-component` pins is a breaking release
whose changelog names the new versions.

1. On a `release/vX.Y.Z` branch from `main`, bump `version` in
   `[workspace.package]` of the top-level `Cargo.toml` (`cargo build` updates
   `Cargo.lock`) and every versioned SDK reference with it: the template's
   and the guide's `waybar_gpui` version, the README's `--tag`, the tagged
   links (`blob/vX.Y.Z`, `tree/vX.Y.Z`) in the README and the template
   README, and the agent skill's URL in the `curl`
   of `docs/guide/using-an-agent.md`. `make check-template` lists any left
   behind.
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

   Pushing the tag runs the `Release` workflow. It first publishes the SDK
   on crates.io from the tagged commit (`cargo publish -p waybar_gpui`), with
   [trusted publishing](https://crates.io/docs/trusted-publishing): crates.io
   trusts this repository's `release.yml`, so no token is stored. It fails
   before publishing if the tag's section of `CHANGELOG.md` is missing or
   undated.

   Publishing can't be undone: a published version can be yanked, never
   replaced ([ADR 0004](adr/0004-published-on-crates-io.md)). If the
   `publish` job fails, no Release is created; check crates.io:

   - The version isn't there: for a transient failure (a crates.io outage),
     re-run the job. Otherwise fix the cause in a pull request, then delete
     the tag (`git push origin :vX.Y.Z`, `git tag -d vX.Y.Z`) and tag the
     new merge commit on `main` as above.
   - The version is there (the upload succeeded, then the job failed): don't
     move the tag, and don't re-run the job, whose `cargo publish` would now
     fail. Create the Release by hand, with the `gh release create` line
     from `release.yml` and the tag's changelog section as notes.
5. Once the version is on crates.io, the same workflow creates the GitHub
   Release with the tag's section of `CHANGELOG.md` as notes. If only this
   job fails, re-run it on its own. Check the new version appears on
   crates.io and the Release on GitHub.
6. Last, it deploys the tag's guide at the site's root (`book` job, calling
   `book.yml`). If it fails, re-run it, or run the `Book` workflow by hand
   with the tag as `ref` and `latest` checked.

v0.1.0 differs: the version is already 0.1.0 and its entry is hand-written,
so its release pull request only replaces `Unreleased` in the entry's heading
with the date.

## License

By contributing, you agree that your contributions are licensed under the
[MIT License](../LICENSE).

[new-issue]: https://github.com/Cerclique/waybar-gpui/issues/new/choose
