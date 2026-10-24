#!/usr/bin/env bash
# Build, then run a second, throwaway Waybar using examples/waybar/ with the
# freshly built binaries first on PATH. Ctrl-C to stop; your own bar is untouched.
set -euo pipefail
cd "$(dirname "$0")/.."

# With gpui-component, so the controls panel is there too.
cargo build --features example-panels/components
export PATH="$PWD/target/debug:$PATH"
# Make sure the daemon is the one just built.
example-panels quit 2>/dev/null || true

trap 'example-panels quit 2>/dev/null || true' EXIT
waybar -c examples/waybar/config.jsonc -s examples/waybar/style.css -l debug "$@"
