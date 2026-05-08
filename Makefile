.PHONY: install build build-rust test test-rust lint lint-rust clean clean-rust \
        dashboard dashboard-build elisp-test elisp-lint \
        install-claude uninstall-claude help

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
	@echo "  install-claude     symlink the plugin into ~/.claude/plugins/ (dev mode)"
	@echo "  uninstall-claude   remove the plugin symlink"
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
	cargo test --manifest-path mcp-servers/org-roam/Cargo.toml

lint:
	npm run lint
	@$(MAKE) lint-rust
	@$(MAKE) elisp-lint

lint-rust:
	cd packages/dashboard-server && cargo clippy --all-targets -- -D warnings
	cargo clippy --all-targets --manifest-path mcp-servers/org-roam/Cargo.toml -- -D warnings

clean: clean-rust
	npm run clean

clean-rust:
	cd packages/dashboard-server && cargo clean
	cargo clean --manifest-path mcp-servers/org-roam/Cargo.toml

elisp-test:
	@if [ -f packages/emacs/Eldev ]; then \
		cd packages/emacs && eldev -C --unstable test; \
	else \
		echo "skip: packages/emacs/Eldev not present yet"; \
	fi

elisp-lint:
	@if [ -f packages/emacs/Eldev ]; then \
		cd packages/emacs && eldev -C --unstable lint; \
	else \
		echo "skip: packages/emacs/Eldev not present yet"; \
	fi

# ---- install-claude / uninstall-claude --------------------------------------
#
# Development-only convenience: symlink the plugin directory into
# ~/.claude/plugins/ so Claude Code picks it up. End users get this
# automatically via the Homebrew formula's caveats instructions.
#
# `uninstall-claude` only removes the symlink if it points back to this repo.

CLAUDE_PLUGINS_DIR := $(HOME)/.claude/plugins
PLUGIN_NAME        := org-roam-toolkit
PLUGIN_DIR         := $(REPO_ROOT)/plugins/$(PLUGIN_NAME)

install-claude:
	@mkdir -p $(CLAUDE_PLUGINS_DIR)
	@target="$(CLAUDE_PLUGINS_DIR)/$(PLUGIN_NAME)"; \
	if [ -e "$$target" ] && [ ! -L "$$target" ]; then \
		echo "ERROR: $$target exists and is not a symlink — refusing to overwrite"; \
		exit 1; \
	fi; \
	ln -snf "$(PLUGIN_DIR)" "$$target"; \
	echo "linked $$target → $(PLUGIN_DIR)"
	@echo ""
	@echo "Note: the plugin's .mcp.json and skill scripts call ortk-* bins on PATH."
	@echo "Either install the brew formula, or symlink the dev bins yourself, e.g.:"
	@echo "  ln -snf $(REPO_ROOT)/packages/emacs/bin/emacs-eval                       /usr/local/bin/ortk-emacs-eval"
	@echo "  ln -snf $(REPO_ROOT)/packages/dashboard-server/target/release/ortk-dashboard /usr/local/bin/ortk-dashboard"
	@echo "  ln -snf $(REPO_ROOT)/mcp-servers/org-roam/target/release/ortk-mcp        /usr/local/bin/ortk-mcp"
	@echo "  ln -snf $(REPO_ROOT)/packages/web/dist/fetch-cli.js                      /usr/local/bin/ortk-fetch"
	@echo "  ln -snf $(REPO_ROOT)/packages/web/dist/ocr-cli.js                        /usr/local/bin/ortk-ocr"

uninstall-claude:
	@target="$(CLAUDE_PLUGINS_DIR)/$(PLUGIN_NAME)"; \
	if [ -L "$$target" ] && readlink "$$target" | grep -qF "$(REPO_ROOT)/"; then \
		rm "$$target"; \
		echo "removed $$target"; \
	else \
		echo "no dev symlink at $$target — nothing to do"; \
	fi
