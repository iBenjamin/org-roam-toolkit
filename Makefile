.PHONY: install build test lint clean elisp-test elisp-lint \
        dashboard dashboard-build install-claude uninstall-claude help

REPO_ROOT      := $(shell pwd)
DASH_PORT      ?= 9876
DASH_SCRIPT    := $(REPO_ROOT)/packages/dashboard-server/dist/server.js

help:
	@echo "Targets (development inside this monorepo — end users install via brew):"
	@echo "  install            npm install (workspaces)"
	@echo "  build              tsc -b (all TS packages, no UI)"
	@echo "  dashboard-build    build TS + Svelte UI bundle"
	@echo "  dashboard          build all + run server in foreground (Ctrl-C to stop)"
	@echo "  test               run all tests (TS + elisp)"
	@echo "  lint               run all linters"
	@echo "  clean              tsc -b --clean"
	@echo "  install-claude     symlink the plugin into ~/.claude/plugins/ (dev mode)"
	@echo "  uninstall-claude   remove the plugin symlink"
	@echo "  elisp-test/-lint   eldev test/lint in packages/emacs"
	@echo ""
	@echo "End-user install (after publishing):"
	@echo "  brew tap iwangkaimin/tap && brew install org-roam-toolkit"
	@echo "  brew services start org-roam-toolkit       # autostart dashboard"

install:
	npm install

build:
	npm run build

dashboard-build: build
	npm -w @org-roam-toolkit/dashboard-server run build:ui

dashboard: dashboard-build
	@echo "Starting dashboard on http://127.0.0.1:$(DASH_PORT) (Ctrl-C to stop)"
	@node $(DASH_SCRIPT) --port=$(DASH_PORT)

test: build
	npm test
	@$(MAKE) elisp-test

lint:
	npm run lint
	@$(MAKE) elisp-lint

clean:
	npm run clean
	@rm -f packages/dashboard-server/ui/dist/index.html 2>/dev/null || true
	@rm -rf packages/dashboard-server/ui/dist/assets 2>/dev/null || true

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
	@echo "  ln -snf $(REPO_ROOT)/packages/emacs/bin/emacs-eval                 /usr/local/bin/ortk-emacs-eval"
	@echo "  ln -snf $(REPO_ROOT)/packages/dashboard-server/bin/dashboard-serve /usr/local/bin/ortk-dashboard"
	@echo "  ln -snf $(REPO_ROOT)/mcp-servers/org-roam/dist/index.js            /usr/local/bin/ortk-mcp"
	@echo "  ln -snf $(REPO_ROOT)/packages/web/dist/fetch-cli.js                /usr/local/bin/ortk-fetch"
	@echo "  ln -snf $(REPO_ROOT)/packages/web/dist/ocr-cli.js                  /usr/local/bin/ortk-ocr"

uninstall-claude:
	@target="$(CLAUDE_PLUGINS_DIR)/$(PLUGIN_NAME)"; \
	if [ -L "$$target" ] && readlink "$$target" | grep -qF "$(REPO_ROOT)/"; then \
		rm "$$target"; \
		echo "removed $$target"; \
	else \
		echo "no dev symlink at $$target — nothing to do"; \
	fi
