# AGENTS.md

`waybar_gpui`: an SDK for writing GPUI panels that open as popovers from Waybar
bar modules on Hyprland. Read before working:

- `docs/GLOSSARY.md`: the domain terms; use them (a panel is never a "module").
- `docs/spec.md`: what is being built, with the tickets behind each decision.
- `docs/architecture.md`: how the runtime works and why; verified behaviour;
  rejected designs. Read before changing `crates/waybar_gpui`.
- `docs/adr/`: decisions that are hard to reverse.
- `docs/CONTRIBUTING.md`: setup, checks and pull-request rules; read before opening a PR.
- `docs/guide/`: the author-facing guide (README quickstart → guide → rustdoc).
  A behaviour change updates the one doc that owns the fact.

Crates: `waybar_gpui` (SDK + runtime), `example-panels` (a program built on
it), `hypr` (internal IPC).

- Build/test: `cargo build`, `cargo test`, `make lint` (clippy without and with
  `--all-features`, plus `cargo doc` with `-D warnings`).
- Try it: `make dev` builds with the `components` feature and runs a second
  Waybar from `crates/example-panels/waybar/` wired to `example-panels`.
- GPUI is `gpui-pre =0.3.9` and the API is pre-1.0: check the source in
  `~/.cargo/registry/src/*/gpui-pre-0.3.9/` before using an API; don't bump the
  pin casually.
- Hyprland is 0.56 with a **Lua** config (`hl.*`), not hyprlang. Verify
  dispatcher/rule syntax against the wiki before writing it.
- Waybar is 0.15.0; the Waybar wiki documents master, which differs.
- `make dev` and `example-panels toggle|preview|daemon <panel>` open popovers
  on the user's screen; say so before running them.
- Push over HTTPS: `git -c credential.helper='!gh auth git-credential' push …`.
  SSH fails from the agent shell (the key has a passphrase and no ssh-agent
  runs).

## Agent skills

### Issue tracker

Issues are tracked in GitHub Issues (Cerclique/waybar_gpui_plugin_template) via the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

Default vocabulary: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: one `docs/GLOSSARY.md` and `docs/adr/`. See `docs/agents/domain.md`.
