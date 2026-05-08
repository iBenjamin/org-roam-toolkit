use anyhow::{bail, Context};
use chrono::Local;
use std::env;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

const PLUGIN_NAME: &str = "org-roam-toolkit";
const CODEX_PLUGIN_TABLE: &str = "plugins.\"org-roam-toolkit@org-roam-toolkit\"";
const CODEX_PLUGIN_BLOCK: &str =
    "[plugins.\"org-roam-toolkit@org-roam-toolkit\"]\nenabled = true\n";
const CODEX_CACHE_REVISION: &str = "local";
const CODEX_MCP_BLOCK: &str = "[mcp_servers.org-roam]\ncommand = \"ortk-mcp\"\n";

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

#[derive(Debug, PartialEq, Eq)]
enum PluginLinkSnapshot {
    Missing,
    Symlink(PathBuf),
}

enum CodexConfigRollback {
    None,
    RemoveNew(PathBuf),
    RestoreBackup { config: PathBuf, backup: PathBuf },
}

fn plugin_link_path(home: &Path, agent_dir: &str) -> PathBuf {
    home.join(agent_dir).join("plugins").join(PLUGIN_NAME)
}

fn codex_plugin_cache_path(home: &Path) -> PathBuf {
    home.join(".codex")
        .join("plugins/cache")
        .join(PLUGIN_NAME)
        .join(PLUGIN_NAME)
        .join(CODEX_CACHE_REVISION)
}

fn same_link_target(link: &Path, plugin_dir: &Path) -> bool {
    fs::read_link(link)
        .map(|target| target == plugin_dir)
        .unwrap_or(false)
}

fn snapshot_plugin_symlink(home: &Path, agent_dir: &str) -> anyhow::Result<PluginLinkSnapshot> {
    let target = plugin_link_path(home, agent_dir);

    if target.exists() || target.is_symlink() {
        let meta = fs::symlink_metadata(&target)
            .with_context(|| format!("inspect {}", target.display()))?;
        if !meta.file_type().is_symlink() {
            bail!("{} exists and is not a symlink", target.display());
        }
        return Ok(PluginLinkSnapshot::Symlink(
            fs::read_link(&target).with_context(|| format!("read {}", target.display()))?,
        ));
    }

    Ok(PluginLinkSnapshot::Missing)
}

fn restore_plugin_symlink(
    home: &Path,
    agent_dir: &str,
    snapshot: &PluginLinkSnapshot,
) -> anyhow::Result<()> {
    let target = plugin_link_path(home, agent_dir);

    if target.exists() || target.is_symlink() {
        let meta = fs::symlink_metadata(&target)
            .with_context(|| format!("inspect {}", target.display()))?;
        if !meta.file_type().is_symlink() {
            bail!("{} exists and is not a symlink", target.display());
        }
        fs::remove_file(&target).with_context(|| format!("remove {}", target.display()))?;
    }

    if let PluginLinkSnapshot::Symlink(previous_target) = snapshot {
        let parent = target.parent().context("plugin target has no parent")?;
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
        symlink(previous_target, &target).with_context(|| format!("link {}", target.display()))?;
    }

    Ok(())
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
                format!(
                    "already installed: {} -> {}",
                    target.display(),
                    plugin_dir.display()
                ),
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
                format!(
                    "would replace: {} -> {}",
                    target.display(),
                    plugin_dir.display()
                ),
            ));
        }
        let previous_target =
            fs::read_link(&target).with_context(|| format!("read {}", target.display()))?;
        fs::remove_file(&target).with_context(|| format!("remove {}", target.display()))?;
        if let Err(err) =
            symlink(plugin_dir, &target).with_context(|| format!("link {}", target.display()))
        {
            if let Err(rollback_err) = symlink(&previous_target, &target)
                .with_context(|| format!("restore {}", target.display()))
            {
                bail!(
                    "failed to replace {}: {err:#}; additionally failed to restore previous link: {rollback_err:#}",
                    target.display()
                );
            }
            bail!(
                "failed to replace {} and restored previous link: {err:#}",
                target.display()
            );
        }
        return Ok((
            InstallOutcome::Replaced,
            format!("replaced: {} -> {}", target.display(), plugin_dir.display()),
        ));
    }

    if dry_run {
        return Ok((
            InstallOutcome::WouldCreate,
            format!(
                "would link: {} -> {}",
                target.display(),
                plugin_dir.display()
            ),
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

pub fn install_all(options: &InstallOptions) -> anyhow::Result<Vec<String>> {
    preflight_plugin_symlink(&options.home, ".claude", &options.plugin_dir, options.force)?;
    preflight_codex(options)?;
    let claude_before = snapshot_plugin_symlink(&options.home, ".claude")?;

    let mut summary = Vec::new();
    summary.push("Claude:".to_string());
    let (claude_outcome, claude_line) = install_plugin_symlink(
        &options.home,
        ".claude",
        &options.plugin_dir,
        options.dry_run,
        options.force,
    )?;
    summary.push(claude_line);
    summary.push("Codex:".to_string());
    match install_codex(options) {
        Ok(lines) => summary.extend(lines),
        Err(err) => {
            if !options.dry_run
                && matches!(
                    claude_outcome,
                    InstallOutcome::Created | InstallOutcome::Replaced
                )
            {
                if let Err(rollback_err) =
                    restore_plugin_symlink(&options.home, ".claude", &claude_before)
                {
                    bail!(
                        "failed to install Codex after installing Claude: {err:#}; \
                         additionally failed to roll back Claude plugin link: {rollback_err:#}"
                    );
                }
                return Err(err).context(
                    "failed to install Codex after installing Claude; rolled back Claude plugin link",
                );
            }
            return Err(err);
        }
    }
    Ok(summary)
}

pub fn backup_suffix_now() -> String {
    Local::now().format("%Y%m%d%H%M%S").to_string()
}

fn installed_plugin_dir_from_exe(exe: &Path) -> Option<PathBuf> {
    let exe = exe.canonicalize().unwrap_or_else(|_| exe.to_path_buf());
    let installed = exe
        .parent()
        .and_then(Path::parent)?
        .join("libexec/plugins")
        .join(PLUGIN_NAME);
    installed.exists().then_some(installed)
}

pub fn default_plugin_dir() -> anyhow::Result<PathBuf> {
    let exe = env::current_exe().context("resolve current executable path")?;
    if let Some(installed) = installed_plugin_dir_from_exe(&exe) {
        return Ok(installed);
    }

    let dev = env::current_dir()
        .context("resolve current directory")?
        .join("plugins")
        .join(PLUGIN_NAME);
    if dev.exists() {
        return Ok(dev);
    }

    bail!("could not infer org-roam-toolkit plugin directory; pass --plugin-dir");
}

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

fn write_backup(config_path: &Path, suffix: &str) -> anyhow::Result<PathBuf> {
    let backup = config_path.with_file_name(format!("config.toml.bak-{suffix}"));
    fs::copy(config_path, &backup)
        .with_context(|| format!("backup {} to {}", config_path.display(), backup.display()))?;
    Ok(backup)
}

fn write_file_atomic(path: &Path, content: &str, suffix: &str) -> anyhow::Result<()> {
    let parent = path.parent().context("config path has no parent")?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .context("config path has no file name")?;
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

fn preflight_plugin_symlink(
    home: &Path,
    agent_dir: &str,
    plugin_dir: &Path,
    force: bool,
) -> anyhow::Result<()> {
    let target = plugin_link_path(home, agent_dir);

    if target.exists() || target.is_symlink() {
        let meta = fs::symlink_metadata(&target)
            .with_context(|| format!("inspect {}", target.display()))?;
        if !meta.file_type().is_symlink() {
            bail!("{} exists and is not a symlink", target.display());
        }
        if !same_link_target(&target, plugin_dir) && !force {
            bail!(
                "{} points elsewhere; pass --force to replace it",
                target.display()
            );
        }
    }

    Ok(())
}

fn preflight_codex_cache(options: &InstallOptions) -> anyhow::Result<()> {
    let target = codex_plugin_cache_path(&options.home);
    let Some(parent) = target.parent() else {
        bail!("Codex plugin cache path has no parent");
    };

    if parent.exists() {
        let meta = fs::symlink_metadata(parent)
            .with_context(|| format!("inspect {}", parent.display()))?;
        if !meta.is_dir() {
            bail!("{} exists and is not a directory", parent.display());
        }
    }

    if target.exists() || target.is_symlink() {
        let meta = fs::symlink_metadata(&target)
            .with_context(|| format!("inspect {}", target.display()))?;
        if !meta.is_dir() && !options.force {
            bail!(
                "{} exists and is not a directory; pass --force to replace it",
                target.display()
            );
        }
    }

    Ok(())
}

fn preflight_codex(options: &InstallOptions) -> anyhow::Result<()> {
    planned_codex_config(options)?;
    preflight_codex_cache(options)?;
    Ok(())
}

fn remove_path(path: &Path) -> anyhow::Result<()> {
    let meta = fs::symlink_metadata(path).with_context(|| format!("inspect {}", path.display()))?;
    if meta.is_dir() {
        fs::remove_dir_all(path).with_context(|| format!("remove {}", path.display()))?;
    } else {
        fs::remove_file(path).with_context(|| format!("remove {}", path.display()))?;
    }
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> anyhow::Result<()> {
    let meta = fs::symlink_metadata(src).with_context(|| format!("inspect {}", src.display()))?;
    let file_type = meta.file_type();

    if file_type.is_symlink() {
        let link_target = fs::read_link(src).with_context(|| format!("read {}", src.display()))?;
        symlink(link_target, dst).with_context(|| format!("link {}", dst.display()))?;
        return Ok(());
    }

    if meta.is_dir() {
        fs::create_dir_all(dst).with_context(|| format!("create {}", dst.display()))?;
        fs::set_permissions(dst, meta.permissions())
            .with_context(|| format!("set permissions on {}", dst.display()))?;
        for entry in fs::read_dir(src).with_context(|| format!("read {}", src.display()))? {
            let entry = entry.with_context(|| format!("read entry in {}", src.display()))?;
            copy_dir_recursive(&entry.path(), &dst.join(entry.file_name()))?;
        }
        return Ok(());
    }

    if meta.is_file() {
        fs::copy(src, dst)
            .with_context(|| format!("copy {} to {}", src.display(), dst.display()))?;
        fs::set_permissions(dst, meta.permissions())
            .with_context(|| format!("set permissions on {}", dst.display()))?;
        return Ok(());
    }

    bail!(
        "unsupported file type in plugin directory: {}",
        src.display()
    );
}

fn install_codex_plugin_cache(
    options: &InstallOptions,
) -> anyhow::Result<(InstallOutcome, String)> {
    let target = codex_plugin_cache_path(&options.home);
    let parent = target
        .parent()
        .context("Codex plugin cache path has no parent")?;
    let exists = target.exists() || target.is_symlink();

    if exists {
        let meta = fs::symlink_metadata(&target)
            .with_context(|| format!("inspect {}", target.display()))?;
        if !meta.is_dir() && !options.force {
            bail!(
                "{} exists and is not a directory; pass --force to replace it",
                target.display()
            );
        }
    }

    if options.dry_run {
        let outcome = if exists {
            InstallOutcome::WouldReplace
        } else {
            InstallOutcome::WouldCreate
        };
        let action = if exists {
            "would update"
        } else {
            "would cache"
        };
        return Ok((
            outcome,
            format!(
                "{action}: {} from {}",
                target.display(),
                options.plugin_dir.display()
            ),
        ));
    }

    fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    let tmp = parent.join(format!(
        ".{CODEX_CACHE_REVISION}.tmp-{}-{}",
        std::process::id(),
        options.backup_suffix
    ));
    let backup = parent.join(format!(
        ".{CODEX_CACHE_REVISION}.bak-{}-{}",
        std::process::id(),
        options.backup_suffix
    ));

    if tmp.exists() || tmp.is_symlink() {
        remove_path(&tmp)?;
    }
    if backup.exists() || backup.is_symlink() {
        remove_path(&backup)?;
    }

    let copy_result = (|| -> anyhow::Result<InstallOutcome> {
        copy_dir_recursive(&options.plugin_dir, &tmp)?;
        if exists {
            fs::rename(&target, &backup)
                .with_context(|| format!("move {} to {}", target.display(), backup.display()))?;
            if let Err(err) = fs::rename(&tmp, &target)
                .with_context(|| format!("move {} to {}", tmp.display(), target.display()))
            {
                if let Err(rollback_err) = fs::rename(&backup, &target).with_context(|| {
                    format!("restore {} from {}", target.display(), backup.display())
                }) {
                    bail!(
                        "failed to update Codex plugin cache: {err:#}; additionally failed to restore previous cache: {rollback_err:#}"
                    );
                }
                return Err(err)
                    .context("failed to update Codex plugin cache; restored previous cache");
            }
            remove_path(&backup)?;
            Ok(InstallOutcome::Replaced)
        } else {
            fs::rename(&tmp, &target)
                .with_context(|| format!("move {} to {}", tmp.display(), target.display()))?;
            Ok(InstallOutcome::Created)
        }
    })();

    if copy_result.is_err() && (tmp.exists() || tmp.is_symlink()) {
        let _ = remove_path(&tmp);
    }
    if copy_result.is_err() && (backup.exists() || backup.is_symlink()) && !target.exists() {
        let _ = fs::rename(&backup, &target);
    }

    let outcome = copy_result?;
    let action = if outcome == InstallOutcome::Replaced {
        "updated"
    } else {
        "cached"
    };
    Ok((
        outcome,
        format!(
            "{action}: {} from {}",
            target.display(),
            options.plugin_dir.display()
        ),
    ))
}

pub fn install_codex(options: &InstallOptions) -> anyhow::Result<Vec<String>> {
    let codex_dir = options.home.join(".codex");
    let (config_path, planned_config) = planned_codex_config(options)?;
    preflight_codex_cache(options)?;

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
            let backup = write_backup(&config_path, &options.backup_suffix)?;
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

    let cache_result = install_codex_plugin_cache(options);
    let (_, cache_line) = match cache_result {
        Ok(cache) => cache,
        Err(err) => match rollback_codex_config(&config_rollback, &options.backup_suffix) {
            Ok(true) => {
                return Err(err)
                    .context("failed to install Codex plugin cache; rolled back Codex config");
            }
            Ok(false) => return Err(err),
            Err(rollback_err) => {
                bail!(
                    "failed to install Codex plugin cache: {err:#}; \
                         additionally failed to roll back Codex config: {rollback_err:#}"
                );
            }
        },
    };
    summary.insert(0, cache_line);

    Ok(summary)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::symlink;
    use std::path::PathBuf;

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
        assert!(summary
            .iter()
            .any(|line| line.contains("already installed")));
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
    fn codex_install_caches_and_enables_plugin_for_discovery() {
        let root = TempDir::new().unwrap();
        let plugin = temp_plugin(&root);
        let opts = options(root.path().join("home"), plugin);

        install_codex(&opts).unwrap();

        let cache = opts
            .home
            .join(".codex/plugins/cache/org-roam-toolkit/org-roam-toolkit/local");
        assert!(cache.join("marker").exists());
        assert!(!cache.is_symlink());
        let config = fs::read_to_string(opts.home.join(".codex/config.toml")).unwrap();
        assert!(config.contains("[plugins.\"org-roam-toolkit@org-roam-toolkit\"]\nenabled = true"));
        assert!(!opts.home.join(".codex/plugins/org-roam-toolkit").exists());
    }

    #[test]
    fn codex_install_appends_mcp_without_touching_existing_content() {
        let root = TempDir::new().unwrap();
        let plugin = temp_plugin(&root);
        let opts = options(root.path().join("home"), plugin);
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
    fn codex_install_is_idempotent_when_mcp_is_already_correct() {
        let root = TempDir::new().unwrap();
        let plugin = temp_plugin(&root);
        let opts = options(root.path().join("home"), plugin);
        let config_path = opts.home.join(".codex/config.toml");
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        fs::write(
            &config_path,
            "[mcp_servers.org-roam]\ncommand = \"ortk-mcp\"\n",
        )
        .unwrap();

        let summary = install_codex(&opts).unwrap();

        let config = fs::read_to_string(&config_path).unwrap();
        assert!(config.contains("[mcp_servers.org-roam]\ncommand = \"ortk-mcp\""));
        assert!(config.contains("[plugins.\"org-roam-toolkit@org-roam-toolkit\"]\nenabled = true"));
        assert!(opts
            .home
            .join(".codex/config.toml.bak-20260508220000")
            .exists());
        assert!(summary.iter().any(|line| line.contains("updated")));
    }

    #[test]
    fn codex_install_is_idempotent_when_target_header_has_trailing_comment() {
        let root = TempDir::new().unwrap();
        let plugin = temp_plugin(&root);
        let opts = options(root.path().join("home"), plugin);
        let config_path = opts.home.join(".codex/config.toml");
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        fs::write(
            &config_path,
            "[mcp_servers.org-roam] # org-roam server\ncommand = \"ortk-mcp\"\n",
        )
        .unwrap();

        let summary = install_codex(&opts).unwrap();

        let config = fs::read_to_string(&config_path).unwrap();
        assert!(config.contains("[mcp_servers.org-roam] # org-roam server\ncommand = \"ortk-mcp\""));
        assert!(config.contains("[plugins.\"org-roam-toolkit@org-roam-toolkit\"]\nenabled = true"));
        assert!(opts
            .home
            .join(".codex/config.toml.bak-20260508220000")
            .exists());
        assert!(summary.iter().any(|line| line.contains("updated")));
    }

    #[test]
    fn codex_install_is_idempotent_when_command_has_trailing_comment() {
        let root = TempDir::new().unwrap();
        let plugin = temp_plugin(&root);
        let opts = options(root.path().join("home"), plugin);
        let config_path = opts.home.join(".codex/config.toml");
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        fs::write(
            &config_path,
            "[mcp_servers.org-roam]\ncommand = \"ortk-mcp\" # installed by org-roam-toolkit\n",
        )
        .unwrap();

        let summary = install_codex(&opts).unwrap();

        let config = fs::read_to_string(&config_path).unwrap();
        assert!(config.contains(
            "[mcp_servers.org-roam]\ncommand = \"ortk-mcp\" # installed by org-roam-toolkit"
        ));
        assert!(config.contains("[plugins.\"org-roam-toolkit@org-roam-toolkit\"]\nenabled = true"));
        assert!(opts
            .home
            .join(".codex/config.toml.bak-20260508220000")
            .exists());
        assert!(summary.iter().any(|line| line.contains("updated")));
    }

    #[test]
    fn codex_install_is_idempotent_when_command_uses_single_quoted_string() {
        let root = TempDir::new().unwrap();
        let plugin = temp_plugin(&root);
        let opts = options(root.path().join("home"), plugin);
        let config_path = opts.home.join(".codex/config.toml");
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        fs::write(
            &config_path,
            "[mcp_servers.org-roam]\ncommand = 'ortk-mcp'\n",
        )
        .unwrap();

        let summary = install_codex(&opts).unwrap();

        let config = fs::read_to_string(&config_path).unwrap();
        assert!(config.contains("[mcp_servers.org-roam]\ncommand = 'ortk-mcp'"));
        assert!(config.contains("[plugins.\"org-roam-toolkit@org-roam-toolkit\"]\nenabled = true"));
        assert!(opts
            .home
            .join(".codex/config.toml.bak-20260508220000")
            .exists());
        assert!(summary.iter().any(|line| line.contains("updated")));
    }

    #[test]
    fn codex_install_is_idempotent_when_plugin_and_mcp_are_already_configured() {
        let root = TempDir::new().unwrap();
        let plugin = temp_plugin(&root);
        let opts = options(root.path().join("home"), plugin);
        let config_path = opts.home.join(".codex/config.toml");
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        fs::write(
            &config_path,
            "[plugins.\"org-roam-toolkit@org-roam-toolkit\"]\nenabled = true\n\n[mcp_servers.org-roam]\ncommand = \"ortk-mcp\"\n",
        )
        .unwrap();

        let summary = install_codex(&opts).unwrap();

        assert_eq!(
            fs::read_to_string(&config_path).unwrap(),
            "[plugins.\"org-roam-toolkit@org-roam-toolkit\"]\nenabled = true\n\n[mcp_servers.org-roam]\ncommand = \"ortk-mcp\"\n",
        );
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
        let plugin = temp_plugin(&root);
        let opts = options(root.path().join("home"), plugin);
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
    fn codex_install_does_not_cache_plugin_when_config_conflicts() {
        let root = TempDir::new().unwrap();
        let plugin = temp_plugin(&root);
        let opts = options(root.path().join("home"), plugin);
        let config_path = opts.home.join(".codex/config.toml");
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        fs::write(
            &config_path,
            "[mcp_servers.org-roam]\ncommand = \"other\"\n",
        )
        .unwrap();

        let err = install_codex(&opts).unwrap_err().to_string();

        assert!(err.contains("conflicting"));
        assert!(!codex_plugin_cache_path(&opts.home).exists());
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
        assert!(config.contains("[plugins.\"org-roam-toolkit@org-roam-toolkit\"]\nenabled = true"));
        assert!(config.contains("[mcp_servers.org-roam]\ncommand = \"ortk-mcp\""));
        assert!(!config.contains("args = [\"bad\"]"));
        assert!(config.contains("[projects.\"/tmp\"]\ntrust_level = \"trusted\""));
    }

    #[test]
    fn codex_force_replaces_conflict_without_eating_following_commented_header() {
        let root = TempDir::new().unwrap();
        let plugin = temp_plugin(&root);
        let mut opts = options(root.path().join("home"), plugin);
        opts.force = true;
        let config_path = opts.home.join(".codex/config.toml");
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        fs::write(
            &config_path,
            "model = \"gpt-5.5\"\n\n[mcp_servers.org-roam]\ncommand = \"other\"\nargs = [\"bad\"]\n\n[projects.\"/tmp\"] # local project\ntrust_level = \"trusted\"\n",
        )
        .unwrap();

        install_codex(&opts).unwrap();

        let config = fs::read_to_string(&config_path).unwrap();
        assert!(config.contains("[plugins.\"org-roam-toolkit@org-roam-toolkit\"]\nenabled = true"));
        assert!(config.contains("[mcp_servers.org-roam]\ncommand = \"ortk-mcp\""));
        assert!(!config.contains("args = [\"bad\"]"));
        assert!(config.contains("[projects.\"/tmp\"] # local project\ntrust_level = \"trusted\""));
    }

    #[test]
    fn codex_dry_run_does_not_create_config_or_cache() {
        let root = TempDir::new().unwrap();
        let plugin = temp_plugin(&root);
        let mut opts = options(root.path().join("home"), plugin);
        opts.dry_run = true;

        let summary = install_codex(&opts).unwrap();

        assert!(!opts.home.join(".codex/config.toml").exists());
        assert!(!codex_plugin_cache_path(&opts.home).exists());
        assert!(summary.iter().any(|line| line.contains("would cache")));
        assert!(summary.iter().any(|line| line.contains("would create")));
    }

    #[test]
    fn codex_install_rolls_back_config_update_when_cache_fails() {
        let root = TempDir::new().unwrap();
        let plugin = temp_plugin(&root);
        let opts = options(root.path().join("home"), plugin);
        let config_path = opts.home.join(".codex/config.toml");
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        fs::write(&config_path, "model = \"gpt-5.5\"\n").unwrap();
        fs::write(opts.home.join(".codex/plugins"), "not a directory").unwrap();

        let err = install_codex(&opts).unwrap_err().to_string();

        assert!(err.contains("rolled back Codex config"));
        assert_eq!(
            fs::read_to_string(&config_path).unwrap(),
            "model = \"gpt-5.5\"\n"
        );
        assert!(opts
            .home
            .join(".codex/config.toml.bak-20260508220000")
            .exists());
    }

    #[test]
    fn install_all_configures_claude_and_codex() {
        let root = TempDir::new().unwrap();
        let plugin = temp_plugin(&root);
        let opts = options(root.path().join("home"), plugin.clone());

        let summary = install_all(&opts).unwrap();

        assert_eq!(summary.first().map(String::as_str), Some("Claude:"));
        assert!(summary.iter().any(|line| line == "Codex:"));
        assert_eq!(
            fs::read_link(opts.home.join(".claude/plugins/org-roam-toolkit")).unwrap(),
            plugin
        );
        assert!(codex_plugin_cache_path(&opts.home).join("marker").exists());
        let config = fs::read_to_string(opts.home.join(".codex/config.toml")).unwrap();
        assert!(config.contains("[plugins.\"org-roam-toolkit@org-roam-toolkit\"]\nenabled = true"));
        assert!(config.contains("[mcp_servers.org-roam]"));
        assert!(config.contains("command = \"ortk-mcp\""));
    }

    #[test]
    fn install_all_does_not_create_claude_link_when_codex_config_conflicts() {
        let root = TempDir::new().unwrap();
        let plugin = temp_plugin(&root);
        let opts = options(root.path().join("home"), plugin);
        let config_path = opts.home.join(".codex/config.toml");
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        fs::write(
            &config_path,
            "[mcp_servers.org-roam]\ncommand = \"other\"\n",
        )
        .unwrap();

        let err = install_all(&opts).unwrap_err().to_string();

        assert!(err.contains("conflicting"));
        assert!(!opts.home.join(".claude/plugins/org-roam-toolkit").exists());
        assert!(!codex_plugin_cache_path(&opts.home).exists());
    }

    #[test]
    fn install_all_rolls_back_created_claude_link_when_codex_update_fails() {
        let root = TempDir::new().unwrap();
        let plugin = temp_plugin(&root);
        let opts = options(root.path().join("home"), plugin);
        let config_path = opts.home.join(".codex/config.toml");
        let backup_path = opts.home.join(".codex/config.toml.bak-20260508220000");
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        fs::write(&config_path, "model = \"gpt-5.5\"\n").unwrap();
        fs::create_dir_all(&backup_path).unwrap();

        let err = install_all(&opts).unwrap_err().to_string();

        assert!(err.contains("rolled back Claude plugin link"));
        assert!(!opts.home.join(".claude/plugins/org-roam-toolkit").exists());
        assert!(!codex_plugin_cache_path(&opts.home).exists());
        assert_eq!(
            fs::read_to_string(&config_path).unwrap(),
            "model = \"gpt-5.5\"\n"
        );
    }

    #[test]
    fn install_all_restores_replaced_claude_link_when_codex_update_fails() {
        let root = TempDir::new().unwrap();
        let plugin = temp_plugin(&root);
        let previous_plugin = root.path().join("previous-plugin");
        fs::create_dir_all(&previous_plugin).unwrap();
        let mut opts = options(root.path().join("home"), plugin);
        opts.force = true;
        let claude_link = opts.home.join(".claude/plugins/org-roam-toolkit");
        fs::create_dir_all(claude_link.parent().unwrap()).unwrap();
        symlink(&previous_plugin, &claude_link).unwrap();
        let config_path = opts.home.join(".codex/config.toml");
        let backup_path = opts.home.join(".codex/config.toml.bak-20260508220000");
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        fs::write(&config_path, "model = \"gpt-5.5\"\n").unwrap();
        fs::create_dir_all(&backup_path).unwrap();

        let err = install_all(&opts).unwrap_err().to_string();

        assert!(err.contains("rolled back Claude plugin link"));
        assert_eq!(fs::read_link(claude_link).unwrap(), previous_plugin);
        assert!(!codex_plugin_cache_path(&opts.home).exists());
        assert_eq!(
            fs::read_to_string(&config_path).unwrap(),
            "model = \"gpt-5.5\"\n"
        );
    }

    #[test]
    fn install_all_rolls_back_claude_and_codex_config_when_codex_cache_fails() {
        let root = TempDir::new().unwrap();
        let plugin = temp_plugin(&root);
        let opts = options(root.path().join("home"), plugin);
        let config_path = opts.home.join(".codex/config.toml");
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        fs::write(&config_path, "model = \"gpt-5.5\"\n").unwrap();
        fs::write(opts.home.join(".codex/plugins"), "not a directory").unwrap();

        let err = install_all(&opts).unwrap_err().to_string();

        assert!(err.contains("rolled back Claude plugin link"));
        assert!(!opts.home.join(".claude/plugins/org-roam-toolkit").exists());
        assert_eq!(
            fs::read_to_string(&config_path).unwrap(),
            "model = \"gpt-5.5\"\n"
        );
    }

    #[test]
    fn default_backup_suffix_has_timestamp_shape() {
        let suffix = backup_suffix_now();

        assert_eq!(suffix.len(), 14);
        assert!(suffix.chars().all(|ch| ch.is_ascii_digit()));
    }

    #[test]
    fn installed_plugin_dir_from_exe_resolves_homebrew_bin_symlink() {
        let root = TempDir::new().unwrap();
        let cellar = root.path().join("Cellar/org-roam-toolkit/0.2.1");
        let bin = cellar.join("bin");
        let plugin = cellar.join("libexec/plugins/org-roam-toolkit");
        fs::create_dir_all(&bin).unwrap();
        fs::create_dir_all(&plugin).unwrap();
        fs::write(bin.join("ortk-agent-install"), "").unwrap();
        let prefix_bin = root.path().join("bin");
        fs::create_dir_all(&prefix_bin).unwrap();
        let symlink_path = prefix_bin.join("ortk-agent-install");
        symlink(cellar.join("bin/ortk-agent-install"), &symlink_path).unwrap();

        assert_eq!(
            installed_plugin_dir_from_exe(&symlink_path).unwrap(),
            plugin.canonicalize().unwrap()
        );
    }
}
