PREFIX ?= $(HOME)/.local
BINDIR ?= $(PREFIX)/bin
NAME   ?= trunk-claude

.PHONY: build install uninstall clean

build:
	cargo build --release

install: build
	install -Dm755 target/release/trunk-rs $(BINDIR)/$(NAME)
	@echo "installed $(BINDIR)/$(NAME)"

uninstall:
	rm -f $(BINDIR)/$(NAME)
	@echo "removed $(BINDIR)/$(NAME)"

clean:
	cargo clean
