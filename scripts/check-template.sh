#!/usr/bin/env bash
# Checks the starter template: every versioned reference to the SDK (the
# template's dependency, tagged guide links, the README quickstart) matches the
# workspace version, and both variants (with and without `components`) generate and pass
# clippy against this checkout. The generated git dependency is rewritten to a
# path one: Cargo fetches a git source even when it's patched, and the tag only
# exists once released.
# Needs cargo-generate. Used by CI; run it locally with `make check-template`.
set -euo pipefail
cd "$(dirname "$0")/.."
repo=$PWD

version=$(sed -n '/^\[workspace.package\]/,/^\[/s/^version = "\(.*\)"/\1/p' Cargo.toml)
# Every versioned reference to the SDK: the template's dependency, tagged guide
# links, `--tag` in the quickstart, `tag = "…"` in examples.
stale=$(grep -rnoE '(blob/|--tag |tag = ")v[0-9]+\.[0-9]+\.[0-9]+' \
  template README.md docs/guide crates/waybar_gpui/src | grep -v "v$version\$" || true)
if [[ -n "$stale" ]]; then
  echo "references to another version than v$version (the workspace version):" >&2
  echo "$stale" >&2
  exit 1
fi
echo "every SDK version reference is v$version"

out=$(mktemp -d)
trap 'rm -rf "$out"' EXIT
# Shared between the variants, so dependencies build once.
export CARGO_TARGET_DIR=${TEMPLATE_TARGET_DIR:-$repo/target/template}

for components in false true; do
  name=hello-panels-$components
  cargo generate --path "$repo/template" --destination "$out" --name "$name" \
    --define id="$name" --define components="$components" --silent --vcs none
  sed -i "s|git = \"[^\"]*\", tag = \"[^\"]*\"|path = \"$repo/crates/waybar_gpui\"|" "$out/$name/Cargo.toml"
  grep -q "path = \"$repo/crates/waybar_gpui\"" "$out/$name/Cargo.toml"
  (cd "$out/$name" && cargo clippy --all-targets -- -D warnings)
  echo "variant components=$components: ok"
done
