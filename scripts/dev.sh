#!/usr/bin/env bash
# Build, then run a second, throwaway Waybar using examples/waybar/ with the
# freshly built programs first on PATH. Ctrl-C to stop; your own bar is untouched.
set -euo pipefail
cd "$(dirname "$0")/.."

# With gpui-component, so the controls panel is there too.
cargo build --features example-panels/components
export PATH="$PWD/target/debug:$PATH"
# A daemon left by another build (or the installed copy) replaces itself with
# this one on the first click; its log is ~/.local/state/waybar-gpui/example-panels.log.
waybar -c examples/waybar/config.jsonc -s examples/waybar/style.css -l debug "$@"
