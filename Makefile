.PHONY: install build build-rust test test-rust lint lint-rust clean clean-rust \
        dashboard dashboard-build elisp-test elisp-lint \
        install-claude uninstall-claude install-codex uninstall-codex install-agents help

REPO_ROOT      := $(shell pwd)
DASH_PORT      ?= 9876
DASH_BIN       := $(REPO_ROOT)/packages/dashboard-server/target/release/ortk-dashboard

help:
	@echo "Targets (development inside this monorepo — end users install via brew):"
	@echo "  install            npm install (TS workspaces)"
	@echo "  build              tsc -b + cargo build --release (everything)"
	@echo "  dashboard-build    alias of build (kept for muscle memory)"
	@echo "  dashboard          run the Rust dashboard binary in foreground"
	@echo "  test               npm test + cargo test + (eldev test if Eldev present)"
	@echo "  lint               npm run lint + cargo clippy + eldev lint"
	@echo "  clean              tsc -b --clean + cargo clean"
	@echo "  install-agents     run install-claude + install-codex (dev mode)"
	@echo "  install-claude     print Claude Code marketplace-add instructions + clean legacy symlink"
	@echo "  install-codex      write [mcp_servers.org-roam] into ~/.codex/config.toml"
	@echo "  uninstall-claude   remove the legacy ~/.claude/plugins/org-roam-toolkit symlink (if any)"
	@echo "  uninstall-codex    no-op (codex plugin cache is managed by codex CLI)"
	@echo "  elisp-test/-lint   eldev test/lint in packages/emacs"
	@echo ""
	@echo "End-user install (after publishing):"
	@echo "  brew tap iBenjamin/tap && brew install org-roam-toolkit"
	@echo "  brew services start org-roam-toolkit       # autostart dashboard"

install:
	npm install

build: build-rust
	npm run build

build-rust:
	cd packages/dashboard-server && cargo build --release
	cargo build --release --manifest-path packages/agent-install/Cargo.toml
	cargo build --release --manifest-path mcp-servers/org-roam/Cargo.toml

dashboard-build: build

dashboard: build-rust
	@echo "Starting dashboard on http://127.0.0.1:$(DASH_PORT) (Ctrl-C to stop)"
	@$(DASH_BIN) --port=$(DASH_PORT)

test: build
	npm test
	@$(MAKE) test-rust
	@$(MAKE) elisp-test

test-rust:
	cd packages/dashboard-server && cargo test
	cargo test --manifest-path packages/agent-install/Cargo.toml
	cargo test --manifest-path mcp-servers/org-roam/Cargo.toml

lint:
	npm run lint
	@$(MAKE) lint-rust
	@$(MAKE) elisp-lint

lint-rust:
	cd packages/dashboard-server && cargo clippy --all-targets -- -D warnings
	cargo clippy --all-targets --manifest-path packages/agent-install/Cargo.toml -- -D warnings
	cargo clippy --all-targets --manifest-path mcp-servers/org-roam/Cargo.toml -- -D warnings

clean: clean-rust
	npm run clean

clean-rust:
	cd packages/dashboard-server && cargo clean
	cargo clean --manifest-path packages/agent-install/Cargo.toml
	cargo clean --manifest-path mcp-servers/org-roam/Cargo.toml

elisp-test:
	@if [ -f packages/emacs/Eldev ] && command -v eldev >/dev/null 2>&1; then \
		cd packages/emacs && eldev -C --unstable test; \
	else \
		echo "skip: eldev is not installed or packages/emacs/Eldev is not present"; \
	fi

elisp-lint:
	@if [ -f packages/emacs/Eldev ] && command -v eldev >/dev/null 2>&1; then \
		cd packages/emacs && eldev -C --unstable lint; \
	else \
		echo "skip: eldev is not installed or packages/emacs/Eldev is not present"; \
	fi

# ---- agent plugin install helpers -------------------------------------------
#
# `install-claude` prints the slash-command instructions and clears any legacy
# symlink left behind by older releases. `install-codex` edits
# ~/.codex/config.toml only; the actual plugin cache is managed by `codex
# plugin marketplace add iBenjamin/org-roam-toolkit`.

CLAUDE_PLUGINS_DIR := $(HOME)/.claude/plugins
PLUGIN_NAME        := org-roam-toolkit

install-claude:
	cargo run --manifest-path packages/agent-install/Cargo.toml -- claude

install-codex:
	cargo run --manifest-path packages/agent-install/Cargo.toml -- codex

install-agents:
	cargo run --manifest-path packages/agent-install/Cargo.toml -- all

uninstall-claude:
	@target="$(CLAUDE_PLUGINS_DIR)/$(PLUGIN_NAME)"; \
	if [ -L "$$target" ]; then \
		rm "$$target"; \
		echo "removed legacy symlink $$target"; \
	else \
		echo "no legacy symlink at $$target — nothing to do"; \
	fi

uninstall-codex:
	@echo "Codex plugin cache is managed by codex CLI:"
	@echo "  codex plugin marketplace remove org-roam-toolkit"
	@echo "Then manually remove [mcp_servers.org-roam] and"
	@echo "[plugins.\"org-roam-toolkit@org-roam-toolkit\"] from ~/.codex/config.toml."
