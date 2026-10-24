PREFIX ?= $(HOME)/.local
BINS := waybar-hypr-ws waybar-popup

.PHONY: build test lint dev install reload uninstall

build:
	cargo build --release

test:
	cargo test

lint:
	cargo clippy --all-targets -- -D warnings

dev:
	./scripts/dev.sh

install: build
	install -Dm755 $(addprefix target/release/,$(BINS)) -t $(PREFIX)/bin
	$(MAKE) reload

# Restart the popup daemon (next click respawns it) and make Waybar re-spawn
# custom modules, picking up new binaries.
reload:
	-waybar-popup quit
	-pkill -SIGUSR2 -x waybar

uninstall:
	-waybar-popup quit
	rm -f $(addprefix $(PREFIX)/bin/,$(BINS))
