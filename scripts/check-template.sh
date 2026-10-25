#!/usr/bin/env bash
# Checks the starter template: its waybar_gpui tag matches the workspace
# version, and both variants (with and without `components`) generate and pass
# clippy against this checkout. The generated git dependency is rewritten to a
# path one: Cargo fetches a git source even when it's patched, and the tag only
# exists once released.
# Needs cargo-generate. Used by CI; run it locally with `make check-template`.
set -euo pipefail
cd "$(dirname "$0")/.."
repo=$PWD

version=$(sed -n '/^\[workspace.package\]/,/^\[/s/^version = "\(.*\)"/\1/p' Cargo.toml)
tag=$(sed -n 's/.*tag = "\([^"]*\)".*/\1/p' template/Cargo.toml)
if [[ "$tag" != "v$version" ]]; then
  echo "template/Cargo.toml has tag \"$tag\", workspace version is $version: expected \"v$version\"" >&2
  exit 1
fi
echo "template tag $tag matches the workspace version"

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
