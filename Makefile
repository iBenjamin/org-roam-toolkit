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
	@echo "  install-agents     install Claude + Codex plugin symlinks/config (dev mode)"
	@echo "  install-claude     symlink the plugin into ~/.claude/plugins/ (dev mode)"
	@echo "  install-codex      symlink plugin into ~/.codex/plugins/ and add org-roam MCP"
	@echo "  uninstall-claude   remove the plugin symlink"
	@echo "  uninstall-codex    remove the Codex plugin symlink"
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
# Development-only convenience: symlink the plugin directory into agent plugin
# directories and configure Codex MCP. End users should run ortk-agent-install
# from the Homebrew formula.
#
# `uninstall-claude` only removes the symlink if it points back to this repo.

CLAUDE_PLUGINS_DIR := $(HOME)/.claude/plugins
PLUGIN_NAME        := org-roam-toolkit
PLUGIN_DIR         := $(REPO_ROOT)/plugins/$(PLUGIN_NAME)

install-claude:
	cargo run --manifest-path packages/agent-install/Cargo.toml -- claude --plugin-dir "$(PLUGIN_DIR)"

install-codex:
	cargo run --manifest-path packages/agent-install/Cargo.toml -- codex --plugin-dir "$(PLUGIN_DIR)"

install-agents:
	cargo run --manifest-path packages/agent-install/Cargo.toml -- all --plugin-dir "$(PLUGIN_DIR)"

uninstall-claude:
	@target="$(CLAUDE_PLUGINS_DIR)/$(PLUGIN_NAME)"; \
	if [ -L "$$target" ] && readlink "$$target" | grep -qF "$(REPO_ROOT)/"; then \
		rm "$$target"; \
		echo "removed $$target"; \
	else \
		echo "no dev symlink at $$target — nothing to do"; \
	fi

uninstall-codex:
	@target="$(HOME)/.codex/plugins/$(PLUGIN_NAME)"; \
	if [ -L "$$target" ] && readlink "$$target" | grep -qF "$(REPO_ROOT)/"; then \
		rm "$$target"; \
		echo "removed $$target"; \
		echo "left ~/.codex/config.toml unchanged"; \
	else \
		echo "no dev symlink at $$target — nothing to do"; \
	fi
