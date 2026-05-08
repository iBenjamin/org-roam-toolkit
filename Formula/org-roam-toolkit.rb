# typed: false
# frozen_string_literal: true

# Homebrew formula for org-roam-toolkit.
#
# This file lives in the source repository for development. The version
# distributed via the tap (iwangkaimin/homebrew-tap) is a copy with the
# `url` and `sha256` updated to the latest release tarball.
#
# Local-source build (no tap required):
#   brew install --build-from-source ./Formula/org-roam-toolkit.rb
#
# Tap-based install (after publishing):
#   brew tap iwangkaimin/tap
#   brew install org-roam-toolkit

class OrgRoamToolkit < Formula
  desc "MCP server, dashboard, and Claude Code plugin for an Emacs/org-roam knowledge base"
  homepage "https://github.com/iwangkaimin/org-roam-toolkit"
  url "https://github.com/iwangkaimin/org-roam-toolkit/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "0000000000000000000000000000000000000000000000000000000000000000"
  license "MIT"
  head "https://github.com/iwangkaimin/org-roam-toolkit.git", branch: "main"

  depends_on "node"
  # Emacs is a runtime dependency for the daemon-side functionality.
  # Users without Emacs can still install the bins; they just won't work.
  depends_on "emacs" => :recommended

  def install
    # Install Node deps for all workspaces (including dev deps — needed for
    # tsc + vite during build). Use --no-audit/--no-fund to keep brew output
    # clean.
    system "npm", "install", "--no-audit", "--no-fund", "--ignore-scripts"

    # Compile every workspace's TypeScript.
    system "npm", "run", "build"

    # Build the Svelte UI bundle (depends on dashboard-server build).
    system "npm", "-w", "@org-roam-toolkit/dashboard-server", "run", "build:ui"

    # Drop dev-only deps now that build is done. (Best-effort — npm prune
    # may keep some entries due to workspace edges; the resulting tree is
    # still functional.)
    system "npm", "prune", "--omit=dev"

    # Stash the entire repo (sources + node_modules + dist) under libexec.
    # We need the layout intact at runtime because:
    #   - bin/emacs-eval is bash and resolves elisp/ via $BASH_SOURCE
    #   - mcp-org-roam imports @org-roam-toolkit/emacs from node_modules
    #   - dashboard-server reads ui/dist/ for static asset serving
    libexec.install Dir["*"]

    # Expose ortk-* bins on PATH. emacs-eval and dashboard-serve are bash
    # wrappers that resolve symlinks themselves; ortk-mcp / ortk-fetch /
    # ortk-ocr are Node entry points (Node handles symlink resolution).
    bin.install_symlink libexec/"packages/emacs/bin/emacs-eval" => "ortk-emacs-eval"
    bin.install_symlink libexec/"packages/dashboard-server/bin/dashboard-serve" => "ortk-dashboard"
    bin.install_symlink libexec/"mcp-servers/org-roam/dist/index.js" => "ortk-mcp"
    bin.install_symlink libexec/"packages/web/dist/fetch-cli.js" => "ortk-fetch"
    bin.install_symlink libexec/"packages/web/dist/ocr-cli.js" => "ortk-ocr"
  end

  service do
    run [opt_bin/"ortk-dashboard", "--port=9876", "--host=127.0.0.1"]
    keep_alive true
    log_path var/"log/org-roam-toolkit-dashboard.log"
    error_log_path var/"log/org-roam-toolkit-dashboard.err.log"
  end

  def caveats
    <<~EOS
      To enable the Claude Code plugin (commands + skills + MCP server registration):

        ln -snf #{opt_libexec}/plugins/org-roam-toolkit ~/.claude/plugins/org-roam-toolkit

      Then restart Claude Code to load the plugin.

      To start the observability dashboard at login:

        brew services start org-roam-toolkit       # http://127.0.0.1:9876

      Or run on demand:

        ortk-dashboard --port=9876

      Runtime prerequisites you must provide yourself:
        - A running Emacs daemon (`emacs --daemon`) with `org-roam` loaded
          and `org-roam-directory` set
        - For the `fetch` skill: `npx playwright install chromium` (one-time
          ~150MB Chromium download)
    EOS
  end

  test do
    # Smoke-tests: the bins should at least start without crashing.
    assert_match "ortk", shell_output("#{bin}/ortk-emacs-eval --help 2>&1", 0..1)
    # MCP server prints its protocol handshake on stdin/stdout; just check it
    # responds to --version-style probe via Node.
    system "node", "--version"
  end
end
