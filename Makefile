PREFIX ?= $(HOME)/.local

.PHONY: install uninstall

install:
	cargo install --path . --locked --force --root "$(PREFIX)"

uninstall:
	rm -f "$(PREFIX)/bin/trunk-codex"
