#!/usr/bin/env bash
# Checks the starter template: every versioned reference to the SDK (the
# template's dependency, tagged guide links, the README quickstart) matches the
# workspace version, the template declares the workspace's rust-version, and
# both variants (with and without `components`) generate and pass clippy
# against this checkout. The generated crates.io dependency is rewritten to a
# path one: the version only exists on crates.io once released.
# Needs cargo-generate. Used by CI; run it locally with `make check-template`.
set -euo pipefail
cd "$(dirname "$0")/.."
repo=$PWD

version=$(sed -n '/^\[workspace.package\]/,/^\[/s/^version = "\(.*\)"/\1/p' Cargo.toml)
# Every versioned reference to the SDK: tagged guide links and `--tag` in the
# quickstart (`vX.Y.Z`), the `waybar_gpui` dependency in the template, the
# guide and example-panels (`"X.Y.Z"`).
stale=$( (grep -rnoE '(blob/|--tag )v[0-9]+\.[0-9]+\.[0-9]+' \
  template README.md docs/guide crates/waybar_gpui/src | grep -v "v$version\$"
  grep -rnoE 'waybar_gpui = (\{ version = )?"[0-9]+\.[0-9]+\.[0-9]+"' \
  template docs/guide examples/example-panels/Cargo.toml | grep -v "\"$version\"\$") || true)
if [[ -n "$stale" ]]; then
  echo "references to another version than v$version (the workspace version):" >&2
  echo "$stale" >&2
  exit 1
fi
echo "every SDK version reference is v$version"

# The template declares the workspace's MSRV.
msrv=$(sed -n '/^\[workspace.package\]/,/^\[/s/^rust-version = "\(.*\)"/\1/p' Cargo.toml)
if ! grep -qx "rust-version = \"$msrv\"" template/Cargo.toml; then
  echo "template/Cargo.toml must declare rust-version = \"$msrv\" (the workspace's)" >&2
  exit 1
fi
echo "the template's rust-version is $msrv"

out=$(mktemp -d)
trap 'rm -rf "$out"' EXIT
# Shared between the variants, so dependencies build once.
export CARGO_TARGET_DIR=${TEMPLATE_TARGET_DIR:-$repo/target/template}

for components in false true; do
  name=hello-panels-$components
  cargo generate --path "$repo/template" --destination "$out" --name "$name" \
    --define id="$name" --define components="$components" --silent --vcs none
  sed -i "/^waybar_gpui = /s|version = \"[^\"]*\"|path = \"$repo/crates/waybar_gpui\"|" "$out/$name/Cargo.toml"
  grep -q "path = \"$repo/crates/waybar_gpui\"" "$out/$name/Cargo.toml"
  (cd "$out/$name" && cargo clippy --all-targets -- -D warnings)
  echo "variant components=$components: ok"
done
