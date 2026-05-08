# typed: false
# frozen_string_literal: true

# Homebrew formula for org-roam-toolkit.
#
# This file lives in the source repository for development. The version
# distributed via the tap (iBenjamin/homebrew-tap) is a copy with the
# `url` and `sha256` updated to the latest release tarball.
#
# Local-source build (no tap required):
#   brew install --build-from-source ./Formula/org-roam-toolkit.rb
#
# Tap-based install (after publishing):
#   brew tap iBenjamin/tap
#   brew install org-roam-toolkit

class OrgRoamToolkit < Formula
  desc "MCP server, dashboard, and Claude Code plugin for an Emacs/org-roam knowledge base"
  homepage "https://github.com/iBenjamin/org-roam-toolkit"
  url "https://github.com/iBenjamin/org-roam-toolkit/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "70454d1341a904790c2de948c46df658671001ed12d73c216a957e59ced149ab"
  license "MIT"
  head "https://github.com/iBenjamin/org-roam-toolkit.git", branch: "main"

  depends_on "node"
  depends_on "rust" => :build
  # Emacs is a runtime dependency for the daemon-side functionality.
  # Users without Emacs can still install the bins; they just won't work.
  depends_on "emacs" => :recommended

  def install
    # --- Node side: emacs / web packages -------------------------------
    # Install Node deps for all workspaces and compile the TypeScript.
    system "npm", "install", "--no-audit", "--no-fund", "--ignore-scripts"
    system "npm", "run", "build"
    # Drop dev-only deps. Best-effort — npm prune may keep some entries
    # due to workspace edges; the resulting tree is still functional.
    system "npm", "prune", "--omit=dev"

    # --- Rust side: ortk-dashboard / ortk-mcp ---------------------------
    cd "packages/dashboard-server" do
      system "cargo", "build", "--release", "--locked"
    end
    cd "mcp-servers/org-roam" do
      system "cargo", "build", "--release", "--locked"
    end

    # --- Stage everything under libexec --------------------------------
    # The bash bin (emacs-eval) resolves elisp/ via $BASH_SOURCE → libexec.
    # The Node bins (fetch, ocr) resolve packages via libexec/node_modules.
    # The Rust bins are self-contained.
    libexec.install Dir["*"]

    # --- Expose ortk-* bins on PATH ------------------------------------
    bin.install_symlink libexec/"packages/emacs/bin/emacs-eval" => "ortk-emacs-eval"
    bin.install_symlink libexec/"mcp-servers/org-roam/target/release/ortk-mcp" => "ortk-mcp"
    bin.install_symlink libexec/"packages/web/dist/fetch-cli.js" => "ortk-fetch"
    bin.install_symlink libexec/"packages/web/dist/ocr-cli.js" => "ortk-ocr"
    bin.install_symlink libexec/"packages/dashboard-server/target/release/ortk-dashboard" => "ortk-dashboard"
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
    # ortk-dashboard responds to --version (built from cargo, has version baked in)
    assert_match version.to_s, shell_output("#{bin}/ortk-dashboard --version")
    # ortk-emacs-eval --help works without a daemon
    assert_match "emacs-eval", shell_output("#{bin}/ortk-emacs-eval --help 2>&1", 0..1)
  end
end
