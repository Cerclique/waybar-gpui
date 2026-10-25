PREFIX ?= $(HOME)/.local
BINS := waybar-hypr-ws example-panels

.PHONY: build test lint check-template dev install reload uninstall

build:
	cargo build --release

test:
	cargo test

# The Rust checks CI runs (.github/workflows/ci.yml); `test` covers doctests.
lint:
	cargo clippy --all-targets -- -D warnings
	cargo clippy --all-targets --all-features -- -D warnings
	RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features

# The starter template, as CI checks it. Needs cargo-generate.
check-template:
	./scripts/check-template.sh

dev:
	./scripts/dev.sh

install: build
	install -Dm755 $(addprefix target/release/,$(BINS)) -t $(PREFIX)/bin
	$(MAKE) reload

# Restart the panel daemon (next click respawns it) and make Waybar re-spawn
# custom modules, picking up new binaries.
reload:
	-example-panels quit
	-pkill -SIGUSR2 -x waybar

uninstall:
	-example-panels quit
	rm -f $(addprefix $(PREFIX)/bin/,$(BINS))
