use anyhow::{bail, Context};
use chrono::Local;
use std::fs;
use std::path::{Path, PathBuf};

const PLUGIN_NAME: &str = "org-roam-toolkit";
const MARKETPLACE_GITHUB: &str = "iBenjamin/org-roam-toolkit";
const PLUGIN_KEY: &str = "org-roam-toolkit@org-roam-toolkit";
const CODEX_PLUGIN_TABLE: &str = "plugins.\"org-roam-toolkit@org-roam-toolkit\"";
const CODEX_PLUGIN_BLOCK: &str =
    "[plugins.\"org-roam-toolkit@org-roam-toolkit\"]\nenabled = true\n";
const CODEX_MCP_BLOCK: &str = "[mcp_servers.org-roam]\ncommand = \"ortk-mcp\"\n";

#[derive(Clone, Debug)]
pub struct InstallOptions {
    pub home: PathBuf,
    pub dry_run: bool,
    pub force: bool,
    pub backup_suffix: String,
}

enum CodexConfigRollback {
    None,
    RemoveNew(PathBuf),
    RestoreBackup { config: PathBuf, backup: PathBuf },
}

// ===========================================================================
// Path helpers
// ===========================================================================

fn claude_legacy_symlink_path(home: &Path) -> PathBuf {
    home.join(".claude/plugins").join(PLUGIN_NAME)
}

// ===========================================================================
// Filesystem helpers
// ===========================================================================

fn write_file_atomic(path: &Path, content: &str, suffix: &str) -> anyhow::Result<()> {
    let parent = path.parent().context("path has no parent")?;
    fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .context("path has no file name")?;
    let tmp_path = parent.join(format!(".{file_name}.tmp-{}-{suffix}", std::process::id()));

    let write_result = (|| -> anyhow::Result<()> {
        fs::write(&tmp_path, content).with_context(|| format!("write {}", tmp_path.display()))?;
        if let Ok(metadata) = fs::metadata(path) {
            fs::set_permissions(&tmp_path, metadata.permissions())
                .with_context(|| format!("set permissions on {}", tmp_path.display()))?;
        }
        fs::rename(&tmp_path, path).with_context(|| format!("replace {}", path.display()))?;
        Ok(())
    })();

    if write_result.is_err() {
        let _ = fs::remove_file(&tmp_path);
    }

    write_result
}

// ===========================================================================
// Claude
// ===========================================================================

fn cleanup_legacy_claude_symlink(home: &Path, dry_run: bool) -> anyhow::Result<Option<String>> {
    let legacy = claude_legacy_symlink_path(home);
    if !(legacy.exists() || legacy.is_symlink()) {
        return Ok(None);
    }
    let meta = fs::symlink_metadata(&legacy)
        .with_context(|| format!("inspect {}", legacy.display()))?;
    if !meta.file_type().is_symlink() {
        bail!(
            "{} exists and is not a symlink; remove it manually",
            legacy.display()
        );
    }
    let target =
        fs::read_link(&legacy).with_context(|| format!("read {}", legacy.display()))?;
    if dry_run {
        return Ok(Some(format!(
            "would remove legacy symlink: {} (was -> {})",
            legacy.display(),
            target.display()
        )));
    }
    fs::remove_file(&legacy).with_context(|| format!("remove {}", legacy.display()))?;
    Ok(Some(format!(
        "removed legacy symlink: {} (was -> {})",
        legacy.display(),
        target.display()
    )))
}

fn claude_guidance_lines() -> Vec<String> {
    vec![
        "Install the Claude Code plugin from a Claude Code session:".to_string(),
        format!("  /plugin marketplace add {MARKETPLACE_GITHUB}"),
        format!("  /plugin install {PLUGIN_KEY}"),
    ]
}

pub fn install_claude(options: &InstallOptions) -> anyhow::Result<Vec<String>> {
    let mut summary = Vec::new();
    if let Some(line) = cleanup_legacy_claude_symlink(&options.home, options.dry_run)? {
        summary.push(line);
    }
    summary.extend(claude_guidance_lines());
    Ok(summary)
}

// ===========================================================================
// Codex TOML editing
// ===========================================================================

fn table_range(content: &str, table: &str) -> Option<(usize, usize)> {
    let wanted = format!("[{table}]");
    let mut start = None;
    let mut end = content.len();
    let mut offset = 0;

    for line in content.split_inclusive('\n') {
        let uncommented = strip_inline_comment(line);
        let trimmed = uncommented.trim();
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

fn strip_inline_comment(line: &str) -> &str {
    let mut in_basic_string = false;
    let mut in_literal_string = false;
    let mut escaped = false;

    for (index, ch) in line.char_indices() {
        if in_basic_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_basic_string = false;
            }
            continue;
        }

        if in_literal_string {
            if ch == '\'' {
                in_literal_string = false;
            }
            continue;
        }

        match ch {
            '"' => in_basic_string = true,
            '\'' => in_literal_string = true,
            '#' => return &line[..index],
            _ => {}
        }
    }

    line
}

fn quoted_value_for_key(table: &str, key: &str) -> Option<String> {
    for line in table.lines() {
        let uncommented = strip_inline_comment(line);
        let trimmed = uncommented.trim();
        if let Some((lhs, rhs)) = trimmed.split_once('=') {
            if lhs.trim() == key {
                return toml_string_value(rhs.trim());
            }
        }
    }
    None
}

fn toml_string_value(value: &str) -> Option<String> {
    if let Some(rest) = value.strip_prefix('"') {
        let mut escaped = false;
        let mut out = String::new();
        for ch in rest.chars() {
            if escaped {
                out.push(ch);
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                return Some(out);
            } else {
                out.push(ch);
            }
        }
        return None;
    }

    if let Some(rest) = value.strip_prefix('\'') {
        return rest.split_once('\'').map(|(inner, _)| inner.to_string());
    }

    None
}

fn has_key(table: &str, key: &str) -> bool {
    table.lines().any(|line| {
        strip_inline_comment(line)
            .trim()
            .split_once('=')
            .map(|(lhs, _)| lhs.trim() == key)
            .unwrap_or(false)
    })
}

fn bool_value_for_key(table: &str, key: &str) -> Option<bool> {
    for line in table.lines() {
        let uncommented = strip_inline_comment(line);
        let trimmed = uncommented.trim();
        if let Some((lhs, rhs)) = trimmed.split_once('=') {
            if lhs.trim() == key {
                return match rhs.trim() {
                    "true" => Some(true),
                    "false" => Some(false),
                    _ => None,
                };
            }
        }
    }
    None
}

fn upsert_table_key(content: &str, range: (usize, usize), key: &str, value: &str) -> String {
    let table = table_body(content, range);
    let mut next_table = String::new();
    let mut replaced = false;

    for line in table.split_inclusive('\n') {
        let uncommented = strip_inline_comment(line);
        let trimmed = uncommented.trim();
        if !replaced
            && trimmed
                .split_once('=')
                .map(|(lhs, _)| lhs.trim() == key)
                .unwrap_or(false)
        {
            let indent: String = line.chars().take_while(|ch| ch.is_whitespace()).collect();
            next_table.push_str(&indent);
            next_table.push_str(key);
            next_table.push_str(" = ");
            next_table.push_str(value);
            if line.ends_with('\n') {
                next_table.push('\n');
            }
            replaced = true;
        } else {
            next_table.push_str(line);
        }
    }

    if !replaced {
        if let Some(header_end) = next_table.find('\n') {
            next_table.insert_str(header_end + 1, &format!("{key} = {value}\n"));
        } else {
            next_table.push('\n');
            next_table.push_str(key);
            next_table.push_str(" = ");
            next_table.push_str(value);
            next_table.push('\n');
        }
    }

    replace_range(content, range, &next_table)
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

fn desired_codex_plugin_config(content: &str) -> Option<String> {
    let Some(range) = table_range(content, CODEX_PLUGIN_TABLE) else {
        return Some(append_block(content.to_string(), CODEX_PLUGIN_BLOCK));
    };

    let body = table_body(content, range);
    if bool_value_for_key(body, "enabled") == Some(true) {
        return None;
    }

    Some(upsert_table_key(content, range, "enabled", "true"))
}

fn desired_codex_mcp_config(content: &str, force: bool) -> anyhow::Result<Option<String>> {
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

fn desired_codex_config(content: &str, force: bool) -> anyhow::Result<Option<String>> {
    let mut next = content.to_string();
    let mut changed = false;

    if let Some(plugin_config) = desired_codex_plugin_config(&next) {
        next = plugin_config;
        changed = true;
    }

    if let Some(mcp_config) = desired_codex_mcp_config(&next, force)? {
        next = mcp_config;
        changed = true;
    }

    Ok(changed.then_some(next))
}

fn write_codex_backup(config_path: &Path, suffix: &str) -> anyhow::Result<PathBuf> {
    let backup = config_path.with_file_name(format!("config.toml.bak-{suffix}"));
    fs::copy(config_path, &backup)
        .with_context(|| format!("backup {} to {}", config_path.display(), backup.display()))?;
    Ok(backup)
}

fn rollback_codex_config(rollback: &CodexConfigRollback, suffix: &str) -> anyhow::Result<bool> {
    match rollback {
        CodexConfigRollback::None => Ok(false),
        CodexConfigRollback::RemoveNew(path) => {
            if path.exists() {
                fs::remove_file(path).with_context(|| format!("remove {}", path.display()))?;
            }
            Ok(true)
        }
        CodexConfigRollback::RestoreBackup { config, backup } => {
            let previous = fs::read_to_string(backup)
                .with_context(|| format!("read backup {}", backup.display()))?;
            write_file_atomic(config, &previous, suffix)
                .with_context(|| format!("restore {}", config.display()))?;
            Ok(true)
        }
    }
}

fn planned_codex_config(options: &InstallOptions) -> anyhow::Result<(PathBuf, Option<String>)> {
    let config_path = options.home.join(".codex/config.toml");
    let current = if config_path.exists() {
        fs::read_to_string(&config_path)
            .with_context(|| format!("read {}", config_path.display()))?
    } else {
        String::new()
    };
    let planned_config = desired_codex_config(&current, options.force)?;
    Ok((config_path, planned_config))
}

fn codex_guidance_lines() -> Vec<String> {
    vec![
        "Register the Codex plugin marketplace (one-time):".to_string(),
        format!("  codex plugin marketplace add {MARKETPLACE_GITHUB}"),
        "Then install + enable the plugin from `codex` and `/plugins`.".to_string(),
    ]
}

pub fn install_codex(options: &InstallOptions) -> anyhow::Result<Vec<String>> {
    let codex_dir = options.home.join(".codex");
    let (config_path, planned_config) = planned_codex_config(options)?;

    let mut summary = Vec::new();
    let mut config_rollback = CodexConfigRollback::None;

    match (config_path.exists(), planned_config) {
        (false, Some(_)) if options.dry_run => {
            summary.push(format!(
                "would create: {} with org-roam plugin and MCP server",
                config_path.display()
            ));
        }
        (false, Some(next)) => {
            fs::create_dir_all(&codex_dir)
                .with_context(|| format!("create {}", codex_dir.display()))?;
            write_file_atomic(&config_path, &next, &options.backup_suffix)?;
            config_rollback = CodexConfigRollback::RemoveNew(config_path.clone());
            summary.push(format!("created: {}", config_path.display()));
        }
        (true, None) => {
            summary.push(format!(
                "already configured: {} has [{}] and [mcp_servers.org-roam]",
                config_path.display(),
                CODEX_PLUGIN_TABLE
            ));
        }
        (true, Some(_)) if options.dry_run => {
            summary.push(format!(
                "would update: {} with [{}] and [mcp_servers.org-roam]",
                config_path.display(),
                CODEX_PLUGIN_TABLE
            ));
        }
        (true, Some(next)) => {
            let backup = write_codex_backup(&config_path, &options.backup_suffix)?;
            write_file_atomic(&config_path, &next, &options.backup_suffix)?;
            config_rollback = CodexConfigRollback::RestoreBackup {
                config: config_path.clone(),
                backup: backup.clone(),
            };
            summary.push(format!("backup: {}", backup.display()));
            summary.push(format!("updated: {}", config_path.display()));
        }
        (false, None) => unreachable!("missing config always needs creation"),
    }

    // Defensive: keep the rollback API around so future failures (e.g. once we
    // shell out to `codex plugin marketplace add`) can revert config edits.
    let _ = &config_rollback;
    let _ = rollback_codex_config;

    summary.extend(codex_guidance_lines());
    Ok(summary)
}

// ===========================================================================
// install_all
// ===========================================================================

pub fn install_all(options: &InstallOptions) -> anyhow::Result<Vec<String>> {
    let mut summary = Vec::new();
    summary.push("Claude:".to_string());
    summary.extend(install_claude(options)?);
    summary.push(String::new());
    summary.push("Codex:".to_string());
    summary.extend(install_codex(options)?);
    Ok(summary)
}

// ===========================================================================
// Public helpers
// ===========================================================================

pub fn backup_suffix_now() -> String {
    Local::now().format("%Y%m%d%H%M%S").to_string()
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::symlink;
    use std::path::PathBuf;

    use tempfile::TempDir;

    use super::*;

    fn options(home: PathBuf) -> InstallOptions {
        InstallOptions {
            home,
            dry_run: false,
            force: false,
            backup_suffix: "20260508220000".to_string(),
        }
    }

    #[test]
    fn claude_install_prints_marketplace_guidance() {
        let root = TempDir::new().unwrap();
        let opts = options(root.path().join("home"));

        let summary = install_claude(&opts).unwrap();

        assert!(summary
            .iter()
            .any(|line| line.contains("/plugin marketplace add")
                && line.contains(MARKETPLACE_GITHUB)));
        assert!(summary
            .iter()
            .any(|line| line.contains("/plugin install") && line.contains(PLUGIN_KEY)));
    }

    #[test]
    fn claude_install_removes_legacy_symlink() {
        let root = TempDir::new().unwrap();
        let opts = options(root.path().join("home"));
        let target = root.path().join("anywhere");
        fs::create_dir_all(&target).unwrap();
        let legacy = opts.home.join(".claude/plugins/org-roam-toolkit");
        fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        symlink(&target, &legacy).unwrap();

        let summary = install_claude(&opts).unwrap();

        assert!(!legacy.exists() && !legacy.is_symlink());
        assert!(summary
            .iter()
            .any(|line| line.contains("removed legacy symlink")));
    }

    #[test]
    fn claude_install_refuses_legacy_path_that_is_not_a_symlink() {
        let root = TempDir::new().unwrap();
        let opts = options(root.path().join("home"));
        let target = opts.home.join(".claude/plugins/org-roam-toolkit");
        fs::create_dir_all(&target).unwrap();

        let err = install_claude(&opts).unwrap_err().to_string();

        assert!(err.contains("not a symlink"));
    }

    #[test]
    fn claude_dry_run_does_not_remove_legacy_symlink() {
        let root = TempDir::new().unwrap();
        let mut opts = options(root.path().join("home"));
        opts.dry_run = true;
        let target = root.path().join("anywhere");
        fs::create_dir_all(&target).unwrap();
        let legacy = opts.home.join(".claude/plugins/org-roam-toolkit");
        fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        symlink(&target, &legacy).unwrap();

        let summary = install_claude(&opts).unwrap();

        assert!(legacy.is_symlink());
        assert!(summary
            .iter()
            .any(|line| line.contains("would remove legacy symlink")));
    }

    #[test]
    fn codex_install_creates_config_with_org_roam_mcp() {
        let root = TempDir::new().unwrap();
        let opts = options(root.path().join("home"));

        install_codex(&opts).unwrap();

        let config = fs::read_to_string(opts.home.join(".codex/config.toml")).unwrap();
        assert!(config.contains("[mcp_servers.org-roam]"));
        assert!(config.contains("command = \"ortk-mcp\""));
        assert!(config.contains("[plugins.\"org-roam-toolkit@org-roam-toolkit\"]\nenabled = true"));
    }

    #[test]
    fn codex_install_does_not_write_plugin_cache() {
        let root = TempDir::new().unwrap();
        let opts = options(root.path().join("home"));

        install_codex(&opts).unwrap();

        // We no longer manage the plugin cache; codex CLI handles it.
        assert!(!opts.home.join(".codex/plugins/cache").exists());
    }

    #[test]
    fn codex_install_appends_mcp_without_touching_existing_content() {
        let root = TempDir::new().unwrap();
        let opts = options(root.path().join("home"));
        let config_path = opts.home.join(".codex/config.toml");
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        fs::write(
            &config_path,
            "model = \"gpt-5.5\"\n\n[mcp_servers.gitnexus]\ncommand = \"gitnexus\"\n",
        )
        .unwrap();

        install_codex(&opts).unwrap();

        let config = fs::read_to_string(&config_path).unwrap();
        assert!(config.starts_with("model = \"gpt-5.5\""));
        assert!(config.contains("[mcp_servers.gitnexus]\ncommand = \"gitnexus\""));
        assert!(config.contains("[plugins.\"org-roam-toolkit@org-roam-toolkit\"]\nenabled = true"));
        assert!(config.contains("[mcp_servers.org-roam]\ncommand = \"ortk-mcp\""));
        assert!(opts
            .home
            .join(".codex/config.toml.bak-20260508220000")
            .exists());
    }

    #[test]
    fn codex_install_is_idempotent_when_already_configured() {
        let root = TempDir::new().unwrap();
        let opts = options(root.path().join("home"));
        let config_path = opts.home.join(".codex/config.toml");
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        fs::write(
            &config_path,
            "[plugins.\"org-roam-toolkit@org-roam-toolkit\"]\nenabled = true\n\n[mcp_servers.org-roam]\ncommand = \"ortk-mcp\"\n",
        )
        .unwrap();

        let summary = install_codex(&opts).unwrap();

        assert!(!opts
            .home
            .join(".codex/config.toml.bak-20260508220000")
            .exists());
        assert!(summary
            .iter()
            .any(|line| line.contains("already configured")));
    }

    #[test]
    fn codex_install_refuses_conflicting_mcp_server() {
        let root = TempDir::new().unwrap();
        let opts = options(root.path().join("home"));
        let config_path = opts.home.join(".codex/config.toml");
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        fs::write(
            &config_path,
            "[mcp_servers.org-roam]\ncommand = \"other\"\n",
        )
        .unwrap();

        let err = install_codex(&opts).unwrap_err().to_string();

        assert!(err.contains("conflicting"));
        assert_eq!(
            fs::read_to_string(&config_path).unwrap(),
            "[mcp_servers.org-roam]\ncommand = \"other\"\n",
        );
    }

    #[test]
    fn codex_dry_run_does_not_write() {
        let root = TempDir::new().unwrap();
        let mut opts = options(root.path().join("home"));
        opts.dry_run = true;

        let summary = install_codex(&opts).unwrap();

        assert!(!opts.home.join(".codex/config.toml").exists());
        assert!(summary.iter().any(|line| line.contains("would create")));
        assert!(summary
            .iter()
            .any(|line| line.contains("codex plugin marketplace add")));
    }

    #[test]
    fn install_all_emits_both_sections() {
        let root = TempDir::new().unwrap();
        let opts = options(root.path().join("home"));

        let summary = install_all(&opts).unwrap();

        assert_eq!(summary.first().map(String::as_str), Some("Claude:"));
        assert!(summary.iter().any(|line| line == "Codex:"));
        assert!(summary
            .iter()
            .any(|line| line.contains("/plugin marketplace add")));
        assert!(summary
            .iter()
            .any(|line| line.contains("codex plugin marketplace add")));
    }

    #[test]
    fn default_backup_suffix_has_timestamp_shape() {
        let suffix = backup_suffix_now();

        assert_eq!(suffix.len(), 14);
        assert!(suffix.chars().all(|ch| ch.is_ascii_digit()));
    }
}
