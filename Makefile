PREFIX ?= $(HOME)/.local
BINS := example-panels

.PHONY: build test lint check-template book dev install reload uninstall

build:
	cargo build --release

test:
	cargo test --locked
	cargo test --locked --all-features

# The Rust checks CI runs (.github/workflows/ci.yml); `test` covers doctests.
lint:
	cargo fmt --all --check
	cargo clippy --locked --all-targets -- -D warnings
	cargo clippy --locked --all-targets --all-features -- -D warnings
	RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps --all-features

# The starter template, as CI checks it. Needs cargo-generate.
check-template:
	./scripts/check-template.sh

# The guide as a site (docs/book.toml), into docs/book/, then its links and
# anchors, offline; CI runs the same. Needs mdbook, python3 and lychee.
# 404.html is left out: its links are rooted at the site's URL (`site-url`).
# `mdbook serve docs` previews it with live reload.
book:
	mdbook build docs
	lychee --offline --no-progress --include-fragments \
		--exclude-path docs/book/404.html docs/book

# A second, throwaway Waybar (top + left bars) wired to the freshly built
# programs, first on PATH; your own bar is untouched. Built with the
# `components` feature, so the controls panel is there too. A daemon left by
# another build (or the installed copy) replaces itself with this one on the
# first click; its log is ~/.local/state/waybar-gpui/example-panels.log.
# Ctrl-C stops it; extra Waybar flags go in ARGS.
dev:
	cargo build --features example-panels/components
	PATH="$(CURDIR)/target/debug:$$PATH" waybar \
		-c examples/example-panels/waybar/config.jsonc \
		-s examples/example-panels/waybar/style.css -l debug $(ARGS)

install: build
	install -Dm755 $(addprefix target/release/,$(BINS)) -t $(PREFIX)/bin
	$(MAKE) reload

# Restart the panel daemon (next click respawns it) and make Waybar reload,
# picking up the new program.
reload:
	-example-panels quit
	-pkill -SIGUSR2 -x waybar

uninstall:
	-example-panels quit
	rm -f $(addprefix $(PREFIX)/bin/,$(BINS))
