#!/usr/bin/env bash
# Checks the starter template: every versioned reference to the SDK (the
# template's dependency, tagged guide links, the README quickstart) matches the
# workspace version, every GPUI version the docs mention matches the
# `gpui-pre` pin, the template declares the workspace's rust-version, and
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

# Every mention of the GPUI pin in the docs (the README badge and its docs.rs
# link, the Versions section, the guide, AGENTS.md) matches Cargo.toml's.
gpui=$(sed -n 's/^gpui *= {.*version = "=\([^"]*\)".*/\1/p' Cargo.toml)
stale=$(grep -rnoE 'gpui-?-pre[-/ ]*(=|%3D)?[0-9]+\.[0-9]+\.[0-9]+' \
  README.md AGENTS.md docs --exclude-dir=research | grep -vE "[-/ =D]$gpui\$" || true)
if [[ -n "$stale" ]]; then
  echo "mentions of another GPUI version than $gpui (the gpui-pre pin):" >&2
  echo "$stale" >&2
  exit 1
fi
echo "every GPUI version mention is $gpui"

# The template declares the workspace's MSRV.
msrv=$(sed -n '/^\[workspace.package\]/,/^\[/s/^rust-version = "\(.*\)"/\1/p' Cargo.toml)
if ! grep -qx "rust-version = \"$msrv\"" template/Cargo.toml.liquid; then
  echo "template/Cargo.toml.liquid must declare rust-version = \"$msrv\" (the workspace's)" >&2
  exit 1
fi
echo "the template's rust-version is $msrv"

out=$(mktemp -d)
trap 'rm -rf "$out"' EXIT
# Shared between the variants, so dependencies build once.
export CARGO_TARGET_DIR=${TEMPLATE_TARGET_DIR:-$repo/target/template}
# The generated crates sit outside the repository, where rust-toolchain.toml
# doesn't apply: build them with its toolchain all the same. A RUSTUP_TOOLCHAIN
# already set wins, as it would over the file inside the repository.
toolchain=$(sed -n 's/^channel = "\(.*\)"/\1/p' rust-toolchain.toml)
export RUSTUP_TOOLCHAIN=${RUSTUP_TOOLCHAIN:-$toolchain}
echo "building the variants with $(rustc --version)"

for components in false true; do
  name=hello-panels-$components
  cargo generate --path "$repo/template" --destination "$out" --name "$name" \
    --define id="$name" --define components="$components" --silent --vcs none
  sed -i "/^waybar_gpui = /s|version = \"[^\"]*\"|path = \"$repo/crates/waybar_gpui\"|" "$out/$name/Cargo.toml"
  grep -q "path = \"$repo/crates/waybar_gpui\"" "$out/$name/Cargo.toml"
  (cd "$out/$name" && cargo clippy --all-targets -- -D warnings)
  echo "variant components=$components: ok"
done
