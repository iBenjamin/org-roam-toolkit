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
  desc "MCP server and Claude Code plugin for Emacs org-roam"
  homepage "https://github.com/iBenjamin/org-roam-toolkit"
  url "https://github.com/iBenjamin/org-roam-toolkit/archive/refs/tags/v0.2.8.tar.gz"
  sha256 "d329f1c60f053670d7d90c92d5ce3eca23c529f7b5d2a33da6c13522acc7cd78"
  license "MIT"
  head "https://github.com/iBenjamin/org-roam-toolkit.git", branch: "main"

  depends_on "rust" => :build
  depends_on "node"

  def install
    # --- Node side: emacs / web packages -------------------------------
    # Install Node deps for all workspaces and compile the TypeScript.
    system "npm", "install", *std_npm_args(prefix: false), "--no-audit", "--no-fund"
    system "npm", "run", "build"
    # Drop dev-only deps. Best-effort — npm prune may keep some entries
    # due to workspace edges; the resulting tree is still functional.
    system "npm", "prune", "--omit=dev"

    # --- Rust side: ortk-dashboard / ortk-agent-install / ortk-mcp ------
    system "cargo", "install", *std_cargo_args(path: "packages/dashboard-server")
    system "cargo", "install", *std_cargo_args(path: "packages/agent-install")
    system "cargo", "install", *std_cargo_args(path: "mcp-servers/org-roam")

    # Keep Cargo build artifacts out of libexec; the target directories are
    # large and not needed at runtime.
    rm_r "mcp-servers/org-roam/target"
    rm_r "packages/agent-install/target"
    rm_r "packages/dashboard-server/target"

    # --- Stage everything under libexec --------------------------------
    # The bash bin (emacs-eval) resolves elisp/ via $BASH_SOURCE → libexec.
    # The Node bins (fetch, ocr) resolve packages via libexec/node_modules.
    # The Rust bins are self-contained.
    libexec.install Dir["*"]

    # --- Expose ortk-* bins on PATH ------------------------------------
    bin.install_symlink libexec/"packages/emacs/bin/emacs-eval" => "ortk-emacs-eval"
    bin.install_symlink libexec/"packages/web/dist/fetch-cli.js" => "ortk-fetch"
    bin.install_symlink libexec/"packages/web/dist/ocr-cli.js" => "ortk-ocr"
  end

  service do
    run [opt_bin/"ortk-dashboard", "--port=9876", "--host=127.0.0.1"]
    keep_alive true
    # launchd-spawned services get a bare PATH (/usr/bin:/bin:/usr/sbin:/sbin),
    # so the dashboard's child processes (ortk-emacs-eval, ortk-mcp, …) would
    # not be found. Inject the Homebrew prefix and a couple of common dirs the
    # dashboard probes might need (emacsclient lives in HOMEBREW_PREFIX/bin
    # too, but users with a custom emacs install should adjust PATH manually).
    environment_variables PATH: "#{HOMEBREW_PREFIX}/bin:#{HOMEBREW_PREFIX}/sbin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin"
    log_path var/"log/org-roam-toolkit-dashboard.log"
    error_log_path var/"log/org-roam-toolkit-dashboard.err.log"
  end

  def caveats
    <<~EOS
      The plugin source is published via GitHub. Install it through each
      agent's plugin manager (this formula only ships the runtime binaries).

      Claude Code — run inside a Claude Code session:

        /plugin marketplace add iBenjamin/org-roam-toolkit
        /plugin install org-roam-toolkit@org-roam-toolkit

      Codex — register the marketplace once, then add the MCP server:

        codex plugin marketplace add iBenjamin/org-roam-toolkit
        ortk-agent-install codex   # writes [mcp_servers.org-roam] to ~/.codex/config.toml

      `ortk-agent-install codex` only edits ~/.codex/config.toml (Codex has no
      CLI to register MCP servers). It backs up an existing config before
      changing it. `ortk-agent-install claude` only cleans up legacy installs
      and prints the slash-command instructions.

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
    assert_match "ortk-agent-install", shell_output("#{bin}/ortk-agent-install --help")
    with_env(HOME: testpath) do
      assert_match "/plugin marketplace add",
                   shell_output("#{bin}/ortk-agent-install all --dry-run")
    end
    # ortk-emacs-eval --help works without a daemon
    assert_match "emacs-eval", shell_output("#{bin}/ortk-emacs-eval --help")
  end
end
