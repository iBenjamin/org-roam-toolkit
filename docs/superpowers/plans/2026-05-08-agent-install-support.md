# Agent Install Support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add explicit Claude Code and Codex installation support for `org-roam-toolkit` skills and MCP server.

**Architecture:** Add a focused Rust installer crate, `packages/agent-install`, that safely links the plugin directory and updates Codex MCP config. Keep Claude and Codex plugin metadata in `plugins/org-roam-toolkit`, and update Homebrew, Makefile, and docs to expose the new `ortk-agent-install` flow.

**Tech Stack:** Rust 2021, `clap`, `anyhow`, `chrono`, `tempfile` for tests, existing Homebrew formula, existing Claude plugin directory.

---

## File Structure

- Create `packages/agent-install/Cargo.toml`
  - Defines the `ortk-agent-install` binary crate and test dependencies.
- Create `packages/agent-install/src/lib.rs`
  - Owns all install behavior: plugin path resolution, symlink safety, Codex TOML editing, backups, dry-run summaries.
- Create `packages/agent-install/src/main.rs`
  - Thin CLI wrapper around library functions.
- Create `plugins/org-roam-toolkit/.codex-plugin/plugin.json`
  - Codex plugin manifest pointing at `./skills/`; `ortk-agent-install codex` owns Codex MCP config.
- Modify `Formula/org-roam-toolkit.rb`
  - Build/install `ortk-agent-install`, remove its Cargo target artifacts, add a Homebrew smoke test, update caveats.
- Modify `Makefile`
  - Include `packages/agent-install` in Rust build/test/lint/clean targets and add agent install helpers.
- Modify `README.md`
  - Replace manual Claude-only setup with `ortk-agent-install all`.
- Modify `plugins/org-roam-toolkit/README.md`
  - Document both Claude and Codex install flows.
- Modify `docs/developing-skills.md`
  - Update Claude-only discovery language to agent plugin discovery where relevant.

## Task 1: Scaffold Installer Crate And Failing Symlink Tests

**Files:**
- Create: `packages/agent-install/Cargo.toml`
- Create: `packages/agent-install/src/lib.rs`
- Create: `packages/agent-install/src/main.rs`

- [ ] **Step 1: Write the failing tests**

Create `packages/agent-install/Cargo.toml`:

```toml
[package]
name = "ortk-agent-install"
version = "0.1.0"
edition = "2021"
license = "MIT"
description = "Install org-roam-toolkit plugin support into Claude Code and Codex"
authors = ["Benjamin Wong <iwangkaimin@gmail.com>"]
repository = "https://github.com/iBenjamin/org-roam-toolkit"
homepage = "https://github.com/iBenjamin/org-roam-toolkit"

[[bin]]
name = "ortk-agent-install"
path = "src/main.rs"

[dependencies]
anyhow = "1"
chrono = { version = "0.4", default-features = false, features = ["clock"] }
clap = { version = "4", features = ["derive"] }

[dev-dependencies]
tempfile = "3"

[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
strip = true
```

Create `packages/agent-install/src/lib.rs` with test-first API stubs and symlink tests:

```rust
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct InstallOptions {
    pub home: PathBuf,
    pub plugin_dir: PathBuf,
    pub dry_run: bool,
    pub force: bool,
    pub backup_suffix: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum InstallOutcome {
    Created,
    AlreadyInstalled,
    Replaced,
    WouldCreate,
    WouldReplace,
}

pub fn install_claude(_options: &InstallOptions) -> anyhow::Result<Vec<String>> {
    anyhow::bail!("install_claude is not implemented")
}

pub fn install_codex(_options: &InstallOptions) -> anyhow::Result<Vec<String>> {
    anyhow::bail!("install_codex is not implemented")
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::symlink;

    use tempfile::TempDir;

    use super::*;

    fn temp_plugin(root: &TempDir) -> PathBuf {
        let plugin = root.path().join("plugin");
        fs::create_dir_all(&plugin).unwrap();
        fs::write(plugin.join("marker"), "ok").unwrap();
        plugin
    }

    fn options(home: PathBuf, plugin_dir: PathBuf) -> InstallOptions {
        InstallOptions {
            home,
            plugin_dir,
            dry_run: false,
            force: false,
            backup_suffix: "20260508220000".to_string(),
        }
    }

    #[test]
    fn claude_install_creates_plugin_symlink() {
        let root = TempDir::new().unwrap();
        let plugin = temp_plugin(&root);
        let opts = options(root.path().join("home"), plugin.clone());

        let summary = install_claude(&opts).unwrap();

        let link = opts.home.join(".claude/plugins/org-roam-toolkit");
        assert_eq!(fs::read_link(link).unwrap(), plugin);
        assert!(summary.iter().any(|line| line.contains("linked")));
    }

    #[test]
    fn claude_install_is_idempotent_for_matching_symlink() {
        let root = TempDir::new().unwrap();
        let plugin = temp_plugin(&root);
        let opts = options(root.path().join("home"), plugin.clone());
        let link = opts.home.join(".claude/plugins/org-roam-toolkit");
        fs::create_dir_all(link.parent().unwrap()).unwrap();
        symlink(&plugin, &link).unwrap();

        let summary = install_claude(&opts).unwrap();

        assert_eq!(fs::read_link(link).unwrap(), plugin);
        assert!(summary.iter().any(|line| line.contains("already installed")));
    }

    #[test]
    fn claude_install_refuses_non_symlink_target() {
        let root = TempDir::new().unwrap();
        let plugin = temp_plugin(&root);
        let opts = options(root.path().join("home"), plugin);
        let target = opts.home.join(".claude/plugins/org-roam-toolkit");
        fs::create_dir_all(&target).unwrap();

        let err = install_claude(&opts).unwrap_err().to_string();

        assert!(err.contains("not a symlink"));
    }

    #[test]
    fn claude_dry_run_does_not_create_symlink() {
        let root = TempDir::new().unwrap();
        let plugin = temp_plugin(&root);
        let mut opts = options(root.path().join("home"), plugin);
        opts.dry_run = true;

        let summary = install_claude(&opts).unwrap();

        assert!(!opts.home.join(".claude/plugins/org-roam-toolkit").exists());
        assert!(summary.iter().any(|line| line.contains("would link")));
    }
}
```

Create `packages/agent-install/src/main.rs` as a temporary stub:

```rust
fn main() {
    eprintln!("ortk-agent-install CLI is not implemented yet");
    std::process::exit(2);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test --manifest-path packages/agent-install/Cargo.toml
```

Expected: tests compile and fail because `install_claude` is not implemented.

- [ ] **Step 3: Implement minimal symlink behavior**

Replace the top of `src/lib.rs` with real symlink helpers:

```rust
use anyhow::{bail, Context};
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

const PLUGIN_NAME: &str = "org-roam-toolkit";

#[derive(Clone, Debug)]
pub struct InstallOptions {
    pub home: PathBuf,
    pub plugin_dir: PathBuf,
    pub dry_run: bool,
    pub force: bool,
    pub backup_suffix: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum InstallOutcome {
    Created,
    AlreadyInstalled,
    Replaced,
    WouldCreate,
    WouldReplace,
}

fn plugin_link_path(home: &Path, agent_dir: &str) -> PathBuf {
    home.join(agent_dir).join("plugins").join(PLUGIN_NAME)
}

fn same_link_target(link: &Path, plugin_dir: &Path) -> bool {
    fs::read_link(link)
        .map(|target| target == plugin_dir)
        .unwrap_or(false)
}

fn install_plugin_symlink(
    home: &Path,
    agent_dir: &str,
    plugin_dir: &Path,
    dry_run: bool,
    force: bool,
) -> anyhow::Result<(InstallOutcome, String)> {
    let target = plugin_link_path(home, agent_dir);
    let parent = target.parent().context("plugin target has no parent")?;

    if target.exists() || target.is_symlink() {
        let meta = fs::symlink_metadata(&target)
            .with_context(|| format!("inspect {}", target.display()))?;
        if !meta.file_type().is_symlink() {
            bail!("{} exists and is not a symlink", target.display());
        }
        if same_link_target(&target, plugin_dir) {
            return Ok((
                InstallOutcome::AlreadyInstalled,
                format!("already installed: {} -> {}", target.display(), plugin_dir.display()),
            ));
        }
        if !force {
            bail!(
                "{} points elsewhere; pass --force to replace it",
                target.display()
            );
        }
        if dry_run {
            return Ok((
                InstallOutcome::WouldReplace,
                format!("would replace: {} -> {}", target.display(), plugin_dir.display()),
            ));
        }
        fs::remove_file(&target).with_context(|| format!("remove {}", target.display()))?;
        symlink(plugin_dir, &target).with_context(|| format!("link {}", target.display()))?;
        return Ok((
            InstallOutcome::Replaced,
            format!("replaced: {} -> {}", target.display(), plugin_dir.display()),
        ));
    }

    if dry_run {
        return Ok((
            InstallOutcome::WouldCreate,
            format!("would link: {} -> {}", target.display(), plugin_dir.display()),
        ));
    }

    fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    symlink(plugin_dir, &target).with_context(|| format!("link {}", target.display()))?;
    Ok((
        InstallOutcome::Created,
        format!("linked: {} -> {}", target.display(), plugin_dir.display()),
    ))
}

pub fn install_claude(options: &InstallOptions) -> anyhow::Result<Vec<String>> {
    let (_, line) = install_plugin_symlink(
        &options.home,
        ".claude",
        &options.plugin_dir,
        options.dry_run,
        options.force,
    )?;
    Ok(vec![line])
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run:

```bash
cargo test --manifest-path packages/agent-install/Cargo.toml
```

Expected: Claude symlink tests pass; Codex still has no tests.

- [ ] **Step 5: Commit**

```bash
git add packages/agent-install
git commit -m "feat: add agent installer crate"
```

## Task 2: Add Codex Config Update Tests And Implementation

**Files:**
- Modify: `packages/agent-install/src/lib.rs`

- [ ] **Step 1: Write failing Codex tests**

Append these tests inside `#[cfg(test)] mod tests`:

```rust
    #[test]
    fn codex_install_creates_config_with_org_roam_mcp() {
        let root = TempDir::new().unwrap();
        let plugin = temp_plugin(&root);
        let opts = options(root.path().join("home"), plugin);

        install_codex(&opts).unwrap();

        let config = fs::read_to_string(opts.home.join(".codex/config.toml")).unwrap();
        assert!(config.contains("[mcp_servers.org-roam]"));
        assert!(config.contains("command = \"ortk-mcp\""));
    }

    #[test]
    fn codex_install_appends_mcp_without_touching_existing_content() {
        let root = TempDir::new().unwrap();
        let plugin = temp_plugin(&root);
        let opts = options(root.path().join("home"), plugin);
        let config_path = opts.home.join(".codex/config.toml");
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        fs::write(&config_path, "model = \"gpt-5.5\"\n\n[mcp_servers.gitnexus]\ncommand = \"gitnexus\"\n").unwrap();

        install_codex(&opts).unwrap();

        let config = fs::read_to_string(&config_path).unwrap();
        assert!(config.starts_with("model = \"gpt-5.5\""));
        assert!(config.contains("[mcp_servers.gitnexus]\ncommand = \"gitnexus\""));
        assert!(config.contains("[mcp_servers.org-roam]\ncommand = \"ortk-mcp\""));
        assert!(opts.home.join(".codex/config.toml.bak-20260508220000").exists());
    }

    #[test]
    fn codex_install_is_idempotent_when_mcp_is_already_correct() {
        let root = TempDir::new().unwrap();
        let plugin = temp_plugin(&root);
        let opts = options(root.path().join("home"), plugin);
        let config_path = opts.home.join(".codex/config.toml");
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        fs::write(&config_path, "[mcp_servers.org-roam]\ncommand = \"ortk-mcp\"\n").unwrap();

        let summary = install_codex(&opts).unwrap();

        assert_eq!(
            fs::read_to_string(&config_path).unwrap(),
            "[mcp_servers.org-roam]\ncommand = \"ortk-mcp\"\n",
        );
        assert!(!opts.home.join(".codex/config.toml.bak-20260508220000").exists());
        assert!(summary.iter().any(|line| line.contains("already configured")));
    }

    #[test]
    fn codex_install_refuses_conflicting_mcp_server() {
        let root = TempDir::new().unwrap();
        let plugin = temp_plugin(&root);
        let opts = options(root.path().join("home"), plugin);
        let config_path = opts.home.join(".codex/config.toml");
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        fs::write(&config_path, "[mcp_servers.org-roam]\ncommand = \"other\"\n").unwrap();

        let err = install_codex(&opts).unwrap_err().to_string();

        assert!(err.contains("conflicting"));
        assert_eq!(
            fs::read_to_string(&config_path).unwrap(),
            "[mcp_servers.org-roam]\ncommand = \"other\"\n",
        );
    }

    #[test]
    fn codex_force_replaces_conflicting_mcp_server() {
        let root = TempDir::new().unwrap();
        let plugin = temp_plugin(&root);
        let mut opts = options(root.path().join("home"), plugin);
        opts.force = true;
        let config_path = opts.home.join(".codex/config.toml");
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        fs::write(
            &config_path,
            "model = \"gpt-5.5\"\n\n[mcp_servers.org-roam]\ncommand = \"other\"\nargs = [\"bad\"]\n\n[projects.\"/tmp\"]\ntrust_level = \"trusted\"\n",
        )
        .unwrap();

        install_codex(&opts).unwrap();

        let config = fs::read_to_string(&config_path).unwrap();
        assert!(config.contains("model = \"gpt-5.5\""));
        assert!(config.contains("[mcp_servers.org-roam]\ncommand = \"ortk-mcp\""));
        assert!(!config.contains("args = [\"bad\"]"));
        assert!(config.contains("[projects.\"/tmp\"]\ntrust_level = \"trusted\""));
    }

    #[test]
    fn codex_dry_run_does_not_create_config_or_link() {
        let root = TempDir::new().unwrap();
        let plugin = temp_plugin(&root);
        let mut opts = options(root.path().join("home"), plugin);
        opts.dry_run = true;

        let summary = install_codex(&opts).unwrap();

        assert!(!opts.home.join(".codex/config.toml").exists());
        assert!(!opts.home.join(".codex/plugins/org-roam-toolkit").exists());
        assert!(summary.iter().any(|line| line.contains("would link")));
        assert!(summary.iter().any(|line| line.contains("would create")));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test --manifest-path packages/agent-install/Cargo.toml
```

Expected: Codex tests fail because `install_codex` is not implemented.

- [ ] **Step 3: Implement Codex config editing**

Add these helpers to `src/lib.rs`:

```rust
const CODEX_MCP_BLOCK: &str = "[mcp_servers.org-roam]\ncommand = \"ortk-mcp\"\n";

fn table_range(content: &str, table: &str) -> Option<(usize, usize)> {
    let wanted = format!("[{table}]");
    let mut start = None;
    let mut end = content.len();
    let mut offset = 0;

    for line in content.split_inclusive('\n') {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            if start.is_some() {
                end = offset;
                break;
            }
            if trimmed == wanted {
                start = Some(offset);
            }
        }
        offset += line.len();
    }

    start.map(|s| (s, end))
}

fn table_body(content: &str, range: (usize, usize)) -> &str {
    &content[range.0..range.1]
}

fn quoted_value_for_key(table: &str, key: &str) -> Option<String> {
    for line in table.lines() {
        let trimmed = line.trim();
        if let Some((lhs, rhs)) = trimmed.split_once('=') {
            if lhs.trim() == key {
                let value = rhs.trim().trim_matches('"').to_string();
                return Some(value);
            }
        }
    }
    None
}

fn has_key(table: &str, key: &str) -> bool {
    table.lines().any(|line| {
        line.trim()
            .split_once('=')
            .map(|(lhs, _)| lhs.trim() == key)
            .unwrap_or(false)
    })
}

fn append_block(mut content: String, block: &str) -> String {
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    if !content.is_empty() {
        content.push('\n');
    }
    content.push_str(block);
    content
}

fn replace_range(content: &str, range: (usize, usize), block: &str) -> String {
    let mut next = String::new();
    next.push_str(&content[..range.0]);
    next.push_str(block);
    if !block.ends_with('\n') {
        next.push('\n');
    }
    if range.1 < content.len() && !content[range.1..].starts_with('\n') {
        next.push('\n');
    }
    next.push_str(content[range.1..].trim_start_matches('\n'));
    next
}

fn desired_codex_config(content: &str, force: bool) -> anyhow::Result<Option<String>> {
    let Some(range) = table_range(content, "mcp_servers.org-roam") else {
        return Ok(Some(append_block(content.to_string(), CODEX_MCP_BLOCK)));
    };

    let body = table_body(content, range);
    let command = quoted_value_for_key(body, "command");
    let has_args = has_key(body, "args");
    if command.as_deref() == Some("ortk-mcp") && !has_args {
        return Ok(None);
    }

    if !force {
        bail!("conflicting [mcp_servers.org-roam] already exists; pass --force to replace it");
    }

    Ok(Some(replace_range(content, range, CODEX_MCP_BLOCK)))
}

fn write_backup(config_path: &Path, suffix: &str) -> anyhow::Result<PathBuf> {
    let backup = config_path.with_file_name(format!("config.toml.bak-{suffix}"));
    fs::copy(config_path, &backup)
        .with_context(|| format!("backup {} to {}", config_path.display(), backup.display()))?;
    Ok(backup)
}
```

Replace `install_codex`:

```rust
pub fn install_codex(options: &InstallOptions) -> anyhow::Result<Vec<String>> {
    let mut summary = Vec::new();
    let (_, link_line) = install_plugin_symlink(
        &options.home,
        ".codex",
        &options.plugin_dir,
        options.dry_run,
        options.force,
    )?;
    summary.push(link_line);

    let codex_dir = options.home.join(".codex");
    let config_path = codex_dir.join("config.toml");

    if !config_path.exists() {
        if options.dry_run {
            summary.push(format!(
                "would create: {} with org-roam MCP server",
                config_path.display()
            ));
            return Ok(summary);
        }
        fs::create_dir_all(&codex_dir).with_context(|| format!("create {}", codex_dir.display()))?;
        fs::write(&config_path, CODEX_MCP_BLOCK)
            .with_context(|| format!("write {}", config_path.display()))?;
        summary.push(format!("created: {}", config_path.display()));
        return Ok(summary);
    }

    let current = fs::read_to_string(&config_path)
        .with_context(|| format!("read {}", config_path.display()))?;
    let Some(next) = desired_codex_config(&current, options.force)? else {
        summary.push(format!(
            "already configured: {} has [mcp_servers.org-roam]",
            config_path.display()
        ));
        return Ok(summary);
    };

    if options.dry_run {
        summary.push(format!(
            "would update: {} with [mcp_servers.org-roam]",
            config_path.display()
        ));
        return Ok(summary);
    }

    let backup = write_backup(&config_path, &options.backup_suffix)?;
    fs::write(&config_path, next).with_context(|| format!("write {}", config_path.display()))?;
    summary.push(format!("backup: {}", backup.display()));
    summary.push(format!("updated: {}", config_path.display()));
    Ok(summary)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run:

```bash
cargo test --manifest-path packages/agent-install/Cargo.toml
```

Expected: all installer library tests pass.

- [ ] **Step 5: Commit**

```bash
git add packages/agent-install/src/lib.rs
git commit -m "feat: configure codex mcp install"
```

## Task 3: Add CLI And Plugin Directory Resolution

**Files:**
- Modify: `packages/agent-install/src/lib.rs`
- Modify: `packages/agent-install/src/main.rs`

- [ ] **Step 1: Write failing CLI-facing tests**

Add tests for plugin resolution and installing both agents:

```rust
    #[test]
    fn install_all_configures_claude_and_codex() {
        let root = TempDir::new().unwrap();
        let plugin = temp_plugin(&root);
        let opts = options(root.path().join("home"), plugin.clone());

        let summary = install_all(&opts).unwrap();

        assert_eq!(
            fs::read_link(opts.home.join(".claude/plugins/org-roam-toolkit")).unwrap(),
            plugin,
        );
        assert!(opts.home.join(".codex/config.toml").exists());
        assert!(summary.iter().any(|line| line.contains("Claude")));
        assert!(summary.iter().any(|line| line.contains("Codex")));
    }

    #[test]
    fn default_backup_suffix_has_timestamp_shape() {
        let suffix = backup_suffix_now();
        assert_eq!(suffix.len(), 14);
        assert!(suffix.chars().all(|ch| ch.is_ascii_digit()));
    }
```

Declare these stubs above the tests:

```rust
pub fn install_all(_options: &InstallOptions) -> anyhow::Result<Vec<String>> {
    anyhow::bail!("install_all is not implemented")
}

pub fn backup_suffix_now() -> String {
    "not-implemented".to_string()
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test --manifest-path packages/agent-install/Cargo.toml
```

Expected: new tests fail because `install_all` and timestamp formatting are incomplete.

- [ ] **Step 3: Implement CLI helpers and main**

In `src/lib.rs`, add:

```rust
use chrono::Local;

pub fn install_all(options: &InstallOptions) -> anyhow::Result<Vec<String>> {
    let mut summary = Vec::new();
    summary.push("Claude:".to_string());
    summary.extend(install_claude(options)?);
    summary.push("Codex:".to_string());
    summary.extend(install_codex(options)?);
    Ok(summary)
}

pub fn backup_suffix_now() -> String {
    Local::now().format("%Y%m%d%H%M%S").to_string()
}

pub fn default_plugin_dir() -> anyhow::Result<PathBuf> {
    let exe = std::env::current_exe().context("resolve current executable")?;
    if let Some(prefix) = exe.parent().and_then(|bin| bin.parent()) {
        let installed = prefix.join("libexec/plugins").join(PLUGIN_NAME);
        if installed.exists() {
            return Ok(installed);
        }
    }

    let dev = std::env::current_dir()
        .context("resolve current directory")?
        .join("plugins")
        .join(PLUGIN_NAME);
    if dev.exists() {
        return Ok(dev);
    }

    bail!(
        "could not find plugin directory; pass --plugin-dir /path/to/plugins/{}",
        PLUGIN_NAME
    )
}
```

Replace `src/main.rs`:

```rust
use std::path::PathBuf;

use anyhow::Context;
use clap::{Parser, Subcommand};
use ortk_agent_install::{
    backup_suffix_now, default_plugin_dir, install_all, install_claude, install_codex,
    InstallOptions,
};

#[derive(Debug, Parser)]
#[command(name = "ortk-agent-install")]
#[command(about = "Install org-roam-toolkit support into Claude Code and Codex")]
struct Cli {
    #[command(subcommand)]
    command: Command,

    #[arg(long)]
    plugin_dir: Option<PathBuf>,

    #[arg(long)]
    dry_run: bool,

    #[arg(long)]
    force: bool,
}

#[derive(Debug, Subcommand)]
enum Command {
    Claude,
    Codex,
    All,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .context("HOME is not set")?;
    let plugin_dir = match cli.plugin_dir {
        Some(path) => path,
        None => default_plugin_dir()?,
    };

    let options = InstallOptions {
        home,
        plugin_dir,
        dry_run: cli.dry_run,
        force: cli.force,
        backup_suffix: backup_suffix_now(),
    };

    let summary = match cli.command {
        Command::Claude => install_claude(&options)?,
        Command::Codex => install_codex(&options)?,
        Command::All => install_all(&options)?,
    };

    for line in summary {
        println!("{line}");
    }

    Ok(())
}
```

- [ ] **Step 4: Verify CLI behavior**

Run:

```bash
cargo test --manifest-path packages/agent-install/Cargo.toml
cargo run --manifest-path packages/agent-install/Cargo.toml -- --help
cargo run --manifest-path packages/agent-install/Cargo.toml -- all --dry-run --plugin-dir ./plugins/org-roam-toolkit
```

Expected:
- tests pass
- `--help` lists `claude`, `codex`, `all`, `--plugin-dir`, `--dry-run`, and `--force`
- dry-run prints planned Claude and Codex actions without modifying real home if run with dry-run

- [ ] **Step 5: Commit**

```bash
git add packages/agent-install/src/lib.rs packages/agent-install/src/main.rs
git commit -m "feat: add agent installer cli"
```

## Task 4: Add Codex Plugin Manifest

**Files:**
- Create: `plugins/org-roam-toolkit/.codex-plugin/plugin.json`

- [ ] **Step 1: Write manifest**

Create `plugins/org-roam-toolkit/.codex-plugin/plugin.json`:

```json
{
  "name": "org-roam-toolkit",
  "version": "0.1.0",
  "description": "End-to-end org-roam workflows for Codex: shared skills plus installer-managed MCP setup. Requires the Homebrew-installed bins (ortk-mcp, ortk-emacs-eval, ortk-fetch, ortk-ocr).",
  "author": {
    "name": "Benjamin Wong",
    "email": "iwangkaimin@gmail.com",
    "url": "https://github.com/iBenjamin"
  },
  "homepage": "https://github.com/iBenjamin/org-roam-toolkit",
  "repository": "https://github.com/iBenjamin/org-roam-toolkit",
  "license": "MIT",
  "keywords": [
    "org-roam",
    "org-mode",
    "emacs",
    "mcp",
    "codex",
    "knowledge-base",
    "zettelkasten"
  ],
  "skills": "./skills/",
  "interface": {
    "displayName": "org-roam toolkit",
    "shortDescription": "Org-roam skills for Codex.",
    "longDescription": "Use org-roam-toolkit from Codex with skills for org-mode, org-roam note management, web fetching, OCR, and local dashboard diagnostics. Run ortk-agent-install codex to register the org-roam MCP server in Codex config.",
    "developerName": "Benjamin Wong",
    "category": "Productivity",
    "capabilities": [
      "Skills"
    ],
    "websiteURL": "https://github.com/iBenjamin/org-roam-toolkit",
    "defaultPrompt": [
      "Search my org-roam notes for this topic.",
      "Create an atomic org-roam note from this.",
      "Check my org-roam dashboard health."
    ],
    "brandColor": "#2563EB"
  }
}
```

- [ ] **Step 2: Validate JSON**

Run:

```bash
node -e "JSON.parse(require('fs').readFileSync('plugins/org-roam-toolkit/.codex-plugin/plugin.json','utf8')); console.log('ok')"
```

Expected: prints `ok`.

- [ ] **Step 3: Commit**

```bash
git add plugins/org-roam-toolkit/.codex-plugin/plugin.json
git commit -m "feat: add codex plugin manifest"
```

## Task 5: Wire Installer Into Build, Test, And Homebrew

**Files:**
- Modify: `Makefile`
- Modify: `Formula/org-roam-toolkit.rb`

- [ ] **Step 1: Update Makefile**

Make these concrete changes:

```make
.PHONY: install build build-rust test test-rust lint lint-rust clean clean-rust \
        dashboard dashboard-build elisp-test elisp-lint \
        install-claude uninstall-claude install-codex uninstall-codex install-agents help
```

In `help`, add:

```make
	@echo "  install-agents     install Claude + Codex plugin symlinks/config (dev mode)"
	@echo "  install-codex      symlink plugin into ~/.codex/plugins/ and add org-roam MCP"
	@echo "  uninstall-codex    remove the Codex plugin symlink"
```

Update Rust targets:

```make
build-rust:
	cd packages/dashboard-server && cargo build --release
	cargo build --release --manifest-path packages/agent-install/Cargo.toml
	cargo build --release --manifest-path mcp-servers/org-roam/Cargo.toml

test-rust:
	cd packages/dashboard-server && cargo test
	cargo test --manifest-path packages/agent-install/Cargo.toml
	cargo test --manifest-path mcp-servers/org-roam/Cargo.toml

lint-rust:
	cd packages/dashboard-server && cargo clippy --all-targets -- -D warnings
	cargo clippy --all-targets --manifest-path packages/agent-install/Cargo.toml -- -D warnings
	cargo clippy --all-targets --manifest-path mcp-servers/org-roam/Cargo.toml -- -D warnings

clean-rust:
	cd packages/dashboard-server && cargo clean
	cargo clean --manifest-path packages/agent-install/Cargo.toml
	cargo clean --manifest-path mcp-servers/org-roam/Cargo.toml
```

Replace `install-claude` with the installer:

```make
install-claude:
	cargo run --manifest-path packages/agent-install/Cargo.toml -- claude --plugin-dir "$(PLUGIN_DIR)"
```

Add Codex helpers:

```make
install-codex:
	cargo run --manifest-path packages/agent-install/Cargo.toml -- codex --plugin-dir "$(PLUGIN_DIR)"

install-agents:
	cargo run --manifest-path packages/agent-install/Cargo.toml -- all --plugin-dir "$(PLUGIN_DIR)"

uninstall-codex:
	@target="$(HOME)/.codex/plugins/$(PLUGIN_NAME)"; \
	if [ -L "$$target" ] && readlink "$$target" | grep -qF "$(REPO_ROOT)/"; then \
		rm "$$target"; \
		echo "removed $$target"; \
		echo "left ~/.codex/config.toml unchanged"; \
	else \
		echo "no dev symlink at $$target — nothing to do"; \
	fi
```

- [ ] **Step 2: Update Homebrew formula**

In `Formula/org-roam-toolkit.rb`, add the installer cargo install after dashboard:

```ruby
    system "cargo", "install", *std_cargo_args(path: "packages/dashboard-server")
    system "cargo", "install", *std_cargo_args(path: "packages/agent-install")
    system "cargo", "install", *std_cargo_args(path: "mcp-servers/org-roam")
```

Remove the new target after build:

```ruby
    rm_r "packages/agent-install/target"
```

Update caveats to lead with:

```ruby
      To enable Claude Code and Codex integrations:

        ortk-agent-install all

      Or install one agent at a time:

        ortk-agent-install claude
        ortk-agent-install codex

      The installer links the plugin directory into the agent config directory.
      For Codex, it also adds [mcp_servers.org-roam] to ~/.codex/config.toml
      after writing a backup.
```

Update formula test:

```ruby
    assert_match "ortk-agent-install", shell_output("#{bin}/ortk-agent-install --help")
    assert_match "would link", shell_output("#{bin}/ortk-agent-install all --dry-run --plugin-dir #{opt_libexec}/plugins/org-roam-toolkit")
```

- [ ] **Step 3: Verify build wiring**

Run:

```bash
make test-rust
ruby -c Formula/org-roam-toolkit.rb
```

Expected:
- dashboard, agent installer, and MCP tests pass
- formula syntax is OK

- [ ] **Step 4: Commit**

```bash
git add Makefile Formula/org-roam-toolkit.rb packages/agent-install/Cargo.lock
git commit -m "build: install agent installer binary"
```

## Task 6: Update README And Plugin Docs

**Files:**
- Modify: `README.md`
- Modify: `plugins/org-roam-toolkit/README.md`
- Modify: `docs/developing-skills.md`

- [ ] **Step 1: Update root README install section**

Replace the manual Claude symlink block with:

```markdown
# 2. Install agent integrations
ortk-agent-install all

# Or choose one:
ortk-agent-install claude
ortk-agent-install codex
```

Add a paragraph:

```markdown
`ortk-agent-install` links the plugin directory into the selected agent. For Codex, it also adds `[mcp_servers.org-roam]` to `~/.codex/config.toml` and writes a timestamped backup before changing an existing config file.
```

Update "What you get" to say:

```markdown
Plus the agent plugin under `plugins/org-roam-toolkit/`, usable from Claude Code and Codex:
```

- [ ] **Step 2: Update plugin README**

Change the title to:

```markdown
# org-roam-toolkit agent plugin
```

Update "What's in here":

```markdown
- **Claude Code commands** (`commands/`) — `/note`, `/study`, `/deep_note`, `/reference`, `/ref-extract`, `/to-read`, `/read-history`, `/add-toolkit`, `/gen-commit-msg`
- **Agent skills** (`skills/`) — `atomic-notes`, `org`, `org-roam`, `fetch`, `dashboard`
- **Claude MCP server registration** (`.mcp.json`) — registers `org-roam`
- **Codex manifest** (`.codex-plugin/plugin.json`) — points Codex at the shared skills; `ortk-agent-install codex` writes MCP config
```

Replace install instructions with:

```markdown
# 1. Install the bins
brew tap iBenjamin/tap
brew install org-roam-toolkit

# 2. Install into both agents
ortk-agent-install all

# Or install one side only
ortk-agent-install claude
ortk-agent-install codex
```

- [ ] **Step 3: Update developing-skills**

Replace the Claude-only discovery sentence with:

```markdown
The plugin is discovered once `plugins/org-roam-toolkit/` is linked into the target agent plugin directory. Use `make install-claude`, `make install-codex`, or `make install-agents` in development; end users should use `ortk-agent-install`.
```

- [ ] **Step 4: Verify docs references**

Run:

```bash
rg -n "install-claude|ortk-agent-install|Codex|Claude Code plugin|agent plugin" README.md plugins/org-roam-toolkit/README.md docs/developing-skills.md Formula/org-roam-toolkit.rb Makefile
```

Expected:
- README and plugin README document `ortk-agent-install`
- Makefile still documents dev helper targets
- No stale instruction says manual Claude symlink is the primary install path

- [ ] **Step 5: Commit**

```bash
git add README.md plugins/org-roam-toolkit/README.md docs/developing-skills.md
git commit -m "docs: document agent installer flow"
```

## Task 7: Full Verification And Release Readiness Check

**Files:**
- No planned source edits unless verification exposes a defect.

- [ ] **Step 1: Run full local verification**

Run:

```bash
npm test
npm run build
make test-rust
cargo clippy --all-targets --manifest-path packages/agent-install/Cargo.toml -- -D warnings
cargo fmt --manifest-path packages/agent-install/Cargo.toml --check
cargo clippy --all-targets --manifest-path packages/dashboard-server/Cargo.toml -- -D warnings
cargo clippy --all-targets --manifest-path mcp-servers/org-roam/Cargo.toml -- -D warnings
brew audit --formula ibenjamin/tap/org-roam-toolkit
```

Expected:
- all tests pass
- all clippy checks pass
- formatting check passes
- formula audit passes

- [ ] **Step 2: Run installer dry-run smoke test**

Run:

```bash
cargo run --manifest-path packages/agent-install/Cargo.toml -- all --dry-run --plugin-dir ./plugins/org-roam-toolkit
```

Expected:
- output includes Claude and Codex sections
- output includes planned symlink action
- output includes planned Codex config action
- no real `~/.claude`, `~/.codex`, or config files are changed because this is dry-run

- [ ] **Step 3: Inspect uncommitted diff**

Run:

```bash
git status --short
git diff --stat
```

Expected:
- working tree is clean after the task commits
- if generated Cargo target files appear, clean them with `cargo clean --manifest-path packages/agent-install/Cargo.toml` before final status

- [ ] **Step 4: Prepare final summary**

Include:

- new command: `ortk-agent-install all`
- Codex manifest path: `plugins/org-roam-toolkit/.codex-plugin/plugin.json`
- Codex config behavior: appends `[mcp_servers.org-roam]`, backs up existing config before changes
- verification commands that passed
- any verification that could not run and why

## Self-Review

Spec coverage:

- Explicit installer binary: covered by Tasks 1-3.
- Claude install behavior: covered by Task 1 and Task 6 docs.
- Codex install behavior and config safety: covered by Task 2 and Task 6 docs.
- Codex manifest: covered by Task 4.
- Homebrew integration: covered by Task 5.
- Makefile and docs: covered by Tasks 5-6.
- Verification: covered by Task 7.

Completeness scan:

- The plan contains no empty markers, no deferred implementation steps, and no cross-task shorthand.

Type consistency:

- The plan consistently uses `InstallOptions`, `install_claude`, `install_codex`, `install_all`, `backup_suffix_now`, and `default_plugin_dir`.
- The command name is consistently `ortk-agent-install`.
