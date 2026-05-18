PREFIX ?= /usr/local
BINDIR ?= $(PREFIX)/bin
SYSDDIR ?= /etc/systemd/system
CONFDIR ?= /etc/opagentd

.PHONY: build install uninstall test clean lint format check

build:
	cargo build --release

install:
	install -Dm755 target/release/opagentd $(DESTDIR)$(BINDIR)/opagentd
	install -Dm755 target/release/opagentctl $(DESTDIR)$(BINDIR)/opagentctl
	install -Dm644 systemd/opagentd.service $(DESTDIR)$(SYSDDIR)/opagentd.service
	@if [ -f $(DESTDIR)$(CONFDIR)/config.toml ]; then \
		echo "Config exists, skipping (use 'make install-config' to overwrite)"; \
	else \
		install -Dm644 config/opagentd.toml $(DESTDIR)$(CONFDIR)/config.toml; \
	fi

install-config:
	install -Dm644 config/opagentd.toml $(DESTDIR)$(CONFDIR)/config.toml

uninstall:
	rm -f $(DESTDIR)$(BINDIR)/opagentd
	rm -f $(DESTDIR)$(BINDIR)/opagentctl
	rm -f $(DESTDIR)$(SYSDDIR)/opagentd.service

test:
	cargo test

clean:
	cargo clean

lint:
	cargo clippy --all-targets -- -D warnings

format:
	cargo fmt

check:
	cargo check
