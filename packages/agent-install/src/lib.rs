use anyhow::{bail, Context};
use chrono::{Local, SecondsFormat, Utc};
use serde_json::{json, Map, Value};
use std::env;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

const PLUGIN_NAME: &str = "org-roam-toolkit";
const MARKETPLACE_NAME: &str = "org-roam-toolkit";
const PLUGIN_KEY: &str = "org-roam-toolkit@org-roam-toolkit";
const CLAUDE_CACHE_REVISION: &str = "local";
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

/// A reversible filesystem mutation. Steps push these onto a list as the
/// install progresses; on overall failure we run `undo` in reverse, on
/// success we run `commit` to clean up backups.
enum StepAction {
    RemoveOnUndo(PathBuf),
    RestoreOnUndo { target: PathBuf, backup: PathBuf },
    RestoreSymlinkOnUndo { path: PathBuf, target: PathBuf },
}

impl StepAction {
    fn undo(self) -> anyhow::Result<()> {
        match self {
            Self::RemoveOnUndo(path) => {
                if path.exists() || path.is_symlink() {
                    remove_path(&path)?;
                }
                Ok(())
            }
            Self::RestoreOnUndo { target, backup } => restore_from_backup(&target, &backup),
            Self::RestoreSymlinkOnUndo { path, target } => {
                if path.exists() || path.is_symlink() {
                    let meta = fs::symlink_metadata(&path)
                        .with_context(|| format!("inspect {}", path.display()))?;
                    if !meta.file_type().is_symlink() {
                        bail!("{} is not a symlink", path.display());
                    }
                    fs::remove_file(&path)
                        .with_context(|| format!("remove {}", path.display()))?;
                }
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)
                        .with_context(|| format!("create {}", parent.display()))?;
                }
                symlink(&target, &path).with_context(|| format!("link {}", path.display()))?;
                Ok(())
            }
        }
    }

    fn commit(self) -> anyhow::Result<()> {
        match self {
            Self::RemoveOnUndo(_) | Self::RestoreSymlinkOnUndo { .. } => Ok(()),
            Self::RestoreOnUndo { backup, .. } => {
                if backup.exists() || backup.is_symlink() {
                    remove_path(&backup)?;
                }
                Ok(())
            }
        }
    }
}

fn run_undo(actions: Vec<StepAction>) {
    for action in actions.into_iter().rev() {
        let _ = action.undo();
    }
}

fn run_commit(actions: Vec<StepAction>) {
    for action in actions {
        let _ = action.commit();
    }
}

enum CodexConfigRollback {
    None,
    RemoveNew(PathBuf),
    RestoreBackup { config: PathBuf, backup: PathBuf },
}

// ===========================================================================
// Path helpers
// ===========================================================================

fn claude_plugin_cache_path(home: &Path) -> PathBuf {
    home.join(".claude")
        .join("plugins/cache")
        .join(MARKETPLACE_NAME)
        .join(PLUGIN_NAME)
        .join(CLAUDE_CACHE_REVISION)
}

fn claude_marketplace_dir(home: &Path) -> PathBuf {
    home.join(".claude")
        .join("plugins/marketplaces")
        .join(MARKETPLACE_NAME)
}

fn claude_marketplace_file(home: &Path) -> PathBuf {
    claude_marketplace_dir(home)
        .join(".claude-plugin")
        .join("marketplace.json")
}

fn claude_installed_plugins_path(home: &Path) -> PathBuf {
    home.join(".claude/plugins/installed_plugins.json")
}

fn claude_known_marketplaces_path(home: &Path) -> PathBuf {
    home.join(".claude/plugins/known_marketplaces.json")
}

fn claude_legacy_symlink_path(home: &Path) -> PathBuf {
    home.join(".claude/plugins").join(PLUGIN_NAME)
}

fn codex_plugin_cache_path(home: &Path) -> PathBuf {
    home.join(".codex")
        .join("plugins/cache")
        .join(PLUGIN_NAME)
        .join(PLUGIN_NAME)
        .join(CODEX_CACHE_REVISION)
}

fn marketplace_source_root(plugin_dir: &Path) -> Option<PathBuf> {
    plugin_dir.parent()?.parent().map(Path::to_path_buf)
}

fn marketplace_source_file(plugin_dir: &Path) -> Option<PathBuf> {
    Some(
        marketplace_source_root(plugin_dir)?
            .join(".claude-plugin")
            .join("marketplace.json"),
    )
}

fn plugin_metadata_file(plugin_dir: &Path) -> PathBuf {
    plugin_dir.join(".claude-plugin").join("plugin.json")
}

// ===========================================================================
// Filesystem helpers
// ===========================================================================

fn remove_path(path: &Path) -> anyhow::Result<()> {
    let meta = fs::symlink_metadata(path).with_context(|| format!("inspect {}", path.display()))?;
    if meta.file_type().is_symlink() || meta.is_file() {
        fs::remove_file(path).with_context(|| format!("remove {}", path.display()))?;
    } else if meta.is_dir() {
        fs::remove_dir_all(path).with_context(|| format!("remove {}", path.display()))?;
    } else {
        bail!("unsupported file type at {}", path.display());
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

fn backup_alongside(path: &Path, suffix: &str) -> anyhow::Result<PathBuf> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .context("backup target has no file name")?;
    let backup = path.with_file_name(format!("{file_name}.bak-{suffix}"));
    if backup.exists() || backup.is_symlink() {
        remove_path(&backup)?;
    }
    Ok(backup)
}

fn restore_from_backup(target: &Path, backup: &Path) -> anyhow::Result<()> {
    if target.exists() || target.is_symlink() {
        remove_path(target)?;
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    fs::rename(backup, target)
        .with_context(|| format!("restore {} from {}", target.display(), backup.display()))?;
    Ok(())
}

// ===========================================================================
// JSON helpers
// ===========================================================================

fn read_json_file(path: &Path) -> anyhow::Result<Option<Value>> {
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let value = serde_json::from_str(&raw)
        .with_context(|| format!("parse JSON in {}", path.display()))?;
    Ok(Some(value))
}

fn write_json_file(path: &Path, value: &Value, suffix: &str) -> anyhow::Result<()> {
    let mut serialised = serde_json::to_string_pretty(value)
        .with_context(|| format!("serialise JSON for {}", path.display()))?;
    serialised.push('\n');
    write_file_atomic(path, &serialised, suffix)
}

fn now_iso() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

// ===========================================================================
// Plugin metadata
// ===========================================================================

fn read_plugin_version(plugin_dir: &Path) -> anyhow::Result<String> {
    let path = plugin_metadata_file(plugin_dir);
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("read plugin metadata at {}", path.display()))?;
    let value: Value = serde_json::from_str(&raw)
        .with_context(|| format!("parse plugin metadata in {}", path.display()))?;
    let version = value
        .get("version")
        .and_then(Value::as_str)
        .with_context(|| format!("missing \"version\" in {}", path.display()))?;
    Ok(version.to_string())
}

fn rewrite_marketplace_json(source_file: &Path, cache_path: &Path) -> anyhow::Result<String> {
    let raw = fs::read_to_string(source_file)
        .with_context(|| format!("read {}", source_file.display()))?;
    let mut value: Value = serde_json::from_str(&raw)
        .with_context(|| format!("parse marketplace.json in {}", source_file.display()))?;

    let cache_str = cache_path
        .to_str()
        .context("cache path is not UTF-8")?
        .to_string();

    let plugins = value
        .get_mut("plugins")
        .and_then(Value::as_array_mut)
        .with_context(|| {
            format!(
                "marketplace.json {} has no plugins array",
                source_file.display()
            )
        })?;

    for plugin in plugins {
        let matches = plugin.get("name").and_then(Value::as_str) == Some(PLUGIN_NAME);
        if !matches {
            continue;
        }
        let entry = plugin
            .as_object_mut()
            .context("plugin entry is not an object")?;
        entry.insert("source".to_string(), Value::String(cache_str.clone()));
    }

    let mut serialised = serde_json::to_string_pretty(&value)
        .with_context(|| format!("serialise marketplace.json from {}", source_file.display()))?;
    serialised.push('\n');
    Ok(serialised)
}

// ===========================================================================
// Shared cache copy (claude + codex)
// ===========================================================================

struct CacheStep {
    outcome: InstallOutcome,
    line: String,
    action: Option<StepAction>,
}

fn install_plugin_cache(
    target: PathBuf,
    plugin_dir: &Path,
    backup_suffix: &str,
    dry_run: bool,
    force: bool,
) -> anyhow::Result<CacheStep> {
    let parent = target
        .parent()
        .map(Path::to_path_buf)
        .context("plugin cache path has no parent")?;
    let exists = target.exists() || target.is_symlink();

    if exists {
        let meta = fs::symlink_metadata(&target)
            .with_context(|| format!("inspect {}", target.display()))?;
        if !meta.is_dir() && !force {
            bail!(
                "{} exists and is not a directory; pass --force to replace it",
                target.display()
            );
        }
    }

    if dry_run {
        let outcome = if exists {
            InstallOutcome::WouldReplace
        } else {
            InstallOutcome::WouldCreate
        };
        let action_label = if exists { "would update" } else { "would cache" };
        return Ok(CacheStep {
            outcome,
            line: format!(
                "{action_label}: {} from {}",
                target.display(),
                plugin_dir.display()
            ),
            action: None,
        });
    }

    fs::create_dir_all(&parent).with_context(|| format!("create {}", parent.display()))?;
    let revision_label = target
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("cache");
    let pid = std::process::id();
    let tmp = parent.join(format!(".{revision_label}.tmp-{pid}-{backup_suffix}"));
    let backup = parent.join(format!(".{revision_label}.bak-{pid}-{backup_suffix}"));

    if tmp.exists() || tmp.is_symlink() {
        remove_path(&tmp)?;
    }
    if backup.exists() || backup.is_symlink() {
        remove_path(&backup)?;
    }

    let copy_result = (|| -> anyhow::Result<InstallOutcome> {
        copy_dir_recursive(plugin_dir, &tmp)?;
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
                        "failed to update plugin cache: {err:#}; \
                         additionally failed to restore previous cache: {rollback_err:#}"
                    );
                }
                return Err(err)
                    .context("failed to update plugin cache; restored previous cache");
            }
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
    let action_label = if outcome == InstallOutcome::Replaced {
        "updated"
    } else {
        "cached"
    };
    let line = format!(
        "{action_label}: {} from {}",
        target.display(),
        plugin_dir.display()
    );
    let action = match outcome {
        InstallOutcome::Replaced => StepAction::RestoreOnUndo {
            target: target.clone(),
            backup,
        },
        InstallOutcome::Created => StepAction::RemoveOnUndo(target.clone()),
        _ => unreachable!(),
    };
    Ok(CacheStep {
        outcome,
        line,
        action: Some(action),
    })
}

// ===========================================================================
// Claude install steps
// ===========================================================================

fn preflight_claude(options: &InstallOptions) -> anyhow::Result<()> {
    let plugin_meta = plugin_metadata_file(&options.plugin_dir);
    if !plugin_meta.exists() {
        bail!(
            "plugin metadata not found: {} (expected {} alongside the plugin)",
            plugin_meta.display(),
            ".claude-plugin/plugin.json"
        );
    }
    let marketplace_source = marketplace_source_file(&options.plugin_dir).with_context(|| {
        format!(
            "cannot infer marketplace.json location for plugin dir {}",
            options.plugin_dir.display()
        )
    })?;
    if !marketplace_source.exists() {
        bail!(
            "marketplace metadata not found: {} (expected {} two levels above the plugin)",
            marketplace_source.display(),
            ".claude-plugin/marketplace.json"
        );
    }

    let legacy = claude_legacy_symlink_path(&options.home);
    if legacy.exists() || legacy.is_symlink() {
        let meta = fs::symlink_metadata(&legacy)
            .with_context(|| format!("inspect {}", legacy.display()))?;
        if !meta.file_type().is_symlink() && !options.force {
            bail!(
                "{} exists and is not a symlink; pass --force to replace it",
                legacy.display()
            );
        }
    }

    let cache = claude_plugin_cache_path(&options.home);
    if let Some(parent) = cache.parent() {
        if parent.exists() {
            let meta = fs::symlink_metadata(parent)
                .with_context(|| format!("inspect {}", parent.display()))?;
            if !meta.is_dir() {
                bail!("{} exists and is not a directory", parent.display());
            }
        }
    }
    if cache.exists() || cache.is_symlink() {
        let meta =
            fs::symlink_metadata(&cache).with_context(|| format!("inspect {}", cache.display()))?;
        if !meta.is_dir() && !options.force {
            bail!(
                "{} exists and is not a directory; pass --force to replace it",
                cache.display()
            );
        }
    }

    let marketplace_dir = claude_marketplace_dir(&options.home);
    if marketplace_dir.exists() || marketplace_dir.is_symlink() {
        let meta = fs::symlink_metadata(&marketplace_dir)
            .with_context(|| format!("inspect {}", marketplace_dir.display()))?;
        if !meta.is_dir() && !options.force {
            bail!(
                "{} exists and is not a directory; pass --force to replace it",
                marketplace_dir.display()
            );
        }
    }

    Ok(())
}

fn cleanup_legacy_claude_symlink(
    home: &Path,
    dry_run: bool,
) -> anyhow::Result<(Option<String>, Option<StepAction>)> {
    let legacy = claude_legacy_symlink_path(home);
    if !(legacy.exists() || legacy.is_symlink()) {
        return Ok((None, None));
    }
    let meta = fs::symlink_metadata(&legacy)
        .with_context(|| format!("inspect {}", legacy.display()))?;
    if !meta.file_type().is_symlink() {
        bail!("{} is not a symlink", legacy.display());
    }
    let previous_target =
        fs::read_link(&legacy).with_context(|| format!("read {}", legacy.display()))?;
    if dry_run {
        return Ok((
            Some(format!(
                "would remove legacy symlink: {} -> {}",
                legacy.display(),
                previous_target.display()
            )),
            None,
        ));
    }
    fs::remove_file(&legacy).with_context(|| format!("remove {}", legacy.display()))?;
    let line = format!(
        "removed legacy symlink: {} (was -> {})",
        legacy.display(),
        previous_target.display()
    );
    let action = StepAction::RestoreSymlinkOnUndo {
        path: legacy,
        target: previous_target,
    };
    Ok((Some(line), Some(action)))
}

fn install_claude_marketplace_dir(
    options: &InstallOptions,
) -> anyhow::Result<(String, Option<StepAction>)> {
    let target_dir = claude_marketplace_dir(&options.home);
    let target_file = claude_marketplace_file(&options.home);
    let cache_path = claude_plugin_cache_path(&options.home);
    let source_file = marketplace_source_file(&options.plugin_dir)
        .context("cannot resolve marketplace.json source")?;

    let desired = rewrite_marketplace_json(&source_file, &cache_path)?;

    let current = if target_file.exists() {
        Some(
            fs::read_to_string(&target_file)
                .with_context(|| format!("read {}", target_file.display()))?,
        )
    } else {
        None
    };

    if current.as_deref() == Some(desired.as_str()) {
        return Ok((
            format!("already configured: {}", target_file.display()),
            None,
        ));
    }

    if options.dry_run {
        let label = if current.is_some() {
            "would update"
        } else {
            "would write"
        };
        return Ok((format!("{label}: {}", target_file.display()), None));
    }

    let dir_existed_before = target_dir.exists() || target_dir.is_symlink();
    let file_existed_before = target_file.exists();

    let (action, label) = if file_existed_before {
        let backup = backup_alongside(&target_file, &options.backup_suffix)?;
        fs::rename(&target_file, &backup).with_context(|| {
            format!(
                "move {} to {}",
                target_file.display(),
                backup.display()
            )
        })?;
        write_file_atomic(&target_file, &desired, &options.backup_suffix)?;
        (
            StepAction::RestoreOnUndo {
                target: target_file.clone(),
                backup,
            },
            "updated",
        )
    } else {
        write_file_atomic(&target_file, &desired, &options.backup_suffix)?;
        let undo_target = if dir_existed_before {
            target_file.clone()
        } else {
            target_dir.clone()
        };
        (StepAction::RemoveOnUndo(undo_target), "wrote")
    };

    Ok((format!("{label}: {}", target_file.display()), Some(action)))
}

fn upsert_claude_installed_plugins(
    options: &InstallOptions,
    version: &str,
) -> anyhow::Result<(String, Option<StepAction>)> {
    let path = claude_installed_plugins_path(&options.home);
    let cache_path = claude_plugin_cache_path(&options.home);
    let install_path = cache_path
        .to_str()
        .context("cache path is not UTF-8")?
        .to_string();

    let current = read_json_file(&path)?;
    let mut next = match current.clone() {
        Some(value) => value,
        None => json!({ "version": 2, "plugins": {} }),
    };

    if next.get("version").is_none() {
        if let Some(map) = next.as_object_mut() {
            map.insert("version".to_string(), json!(2));
        }
    }

    let plugins_obj = next
        .get_mut("plugins")
        .and_then(Value::as_object_mut)
        .with_context(|| format!("\"plugins\" in {} is not an object", path.display()))?;

    let existing_array: Vec<Value> = plugins_obj
        .get(PLUGIN_KEY)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let prior_user_entry = existing_array
        .iter()
        .find(|entry| entry.get("scope").and_then(Value::as_str) == Some("user"))
        .cloned();
    let other_entries: Vec<Value> = existing_array
        .iter()
        .filter(|entry| entry.get("scope").and_then(Value::as_str) != Some("user"))
        .cloned()
        .collect();

    let now = now_iso();
    let installed_at = prior_user_entry
        .as_ref()
        .and_then(|entry| entry.get("installedAt").cloned())
        .unwrap_or_else(|| Value::String(now.clone()));

    let mut entry = Map::new();
    entry.insert("scope".to_string(), Value::String("user".to_string()));
    entry.insert("installPath".to_string(), Value::String(install_path));
    entry.insert("version".to_string(), Value::String(version.to_string()));
    entry.insert("installedAt".to_string(), installed_at);
    entry.insert("lastUpdated".to_string(), Value::String(now));
    let new_entry = Value::Object(entry);

    let mut new_entries = other_entries;
    new_entries.push(new_entry);
    plugins_obj.insert(PLUGIN_KEY.to_string(), Value::Array(new_entries));

    let already_correct = match (current.as_ref(), prior_user_entry.as_ref()) {
        (Some(_), Some(prev)) => {
            prev.get("installPath").and_then(Value::as_str)
                == cache_path.to_str()
                && prev.get("version").and_then(Value::as_str) == Some(version)
        }
        _ => false,
    };
    if already_correct {
        return Ok((
            format!("already configured: {} entry for {}", path.display(), PLUGIN_KEY),
            None,
        ));
    }

    if options.dry_run {
        let label = if current.is_some() {
            "would update"
        } else {
            "would create"
        };
        return Ok((
            format!("{label}: {} entry for {}", path.display(), PLUGIN_KEY),
            None,
        ));
    }

    let (action, label) = if current.is_some() {
        let backup = backup_alongside(&path, &options.backup_suffix)?;
        fs::rename(&path, &backup)
            .with_context(|| format!("move {} to {}", path.display(), backup.display()))?;
        write_json_file(&path, &next, &options.backup_suffix)?;
        (
            StepAction::RestoreOnUndo {
                target: path.clone(),
                backup,
            },
            "updated",
        )
    } else {
        write_json_file(&path, &next, &options.backup_suffix)?;
        (StepAction::RemoveOnUndo(path.clone()), "created")
    };

    Ok((
        format!("{label}: {} entry for {}", path.display(), PLUGIN_KEY),
        Some(action),
    ))
}

fn upsert_claude_known_marketplaces(
    options: &InstallOptions,
) -> anyhow::Result<(String, Option<StepAction>)> {
    let path = claude_known_marketplaces_path(&options.home);
    let marketplace_dir = claude_marketplace_dir(&options.home);
    let install_location = marketplace_dir
        .to_str()
        .context("marketplace dir path is not UTF-8")?
        .to_string();
    let source_root = marketplace_source_root(&options.plugin_dir)
        .context("cannot resolve marketplace source root")?;
    let source_path = source_root
        .to_str()
        .context("marketplace source root is not UTF-8")?
        .to_string();

    let current = read_json_file(&path)?;
    let mut next = current
        .clone()
        .unwrap_or_else(|| Value::Object(Map::new()));
    if !next.is_object() {
        bail!("{} is not a JSON object", path.display());
    }

    let prior_entry = next.get(MARKETPLACE_NAME).cloned();
    let now = now_iso();

    let mut entry = Map::new();
    entry.insert(
        "source".to_string(),
        json!({ "source": "local", "path": source_path }),
    );
    entry.insert(
        "installLocation".to_string(),
        Value::String(install_location.clone()),
    );
    entry.insert("lastUpdated".to_string(), Value::String(now));
    let new_entry = Value::Object(entry);

    next.as_object_mut()
        .expect("checked above")
        .insert(MARKETPLACE_NAME.to_string(), new_entry);

    let already_correct = match prior_entry.as_ref() {
        Some(prev) => {
            let same_install = prev
                .get("installLocation")
                .and_then(Value::as_str)
                == Some(install_location.as_str());
            let same_source = prev
                .get("source")
                .and_then(|s| s.get("path"))
                .and_then(Value::as_str)
                == Some(source_root.to_str().unwrap_or(""));
            same_install && same_source
        }
        None => false,
    };
    if already_correct {
        return Ok((
            format!(
                "already configured: {} entry for {}",
                path.display(),
                MARKETPLACE_NAME
            ),
            None,
        ));
    }

    if options.dry_run {
        let label = if current.is_some() {
            "would update"
        } else {
            "would create"
        };
        return Ok((
            format!(
                "{label}: {} entry for {}",
                path.display(),
                MARKETPLACE_NAME
            ),
            None,
        ));
    }

    let (action, label) = if current.is_some() {
        let backup = backup_alongside(&path, &options.backup_suffix)?;
        fs::rename(&path, &backup)
            .with_context(|| format!("move {} to {}", path.display(), backup.display()))?;
        write_json_file(&path, &next, &options.backup_suffix)?;
        (
            StepAction::RestoreOnUndo {
                target: path.clone(),
                backup,
            },
            "updated",
        )
    } else {
        write_json_file(&path, &next, &options.backup_suffix)?;
        (StepAction::RemoveOnUndo(path.clone()), "created")
    };

    Ok((
        format!(
            "{label}: {} entry for {}",
            path.display(),
            MARKETPLACE_NAME
        ),
        Some(action),
    ))
}

fn install_claude_steps(
    options: &InstallOptions,
) -> anyhow::Result<(Vec<String>, Vec<StepAction>)> {
    let mut undo: Vec<StepAction> = Vec::new();
    let mut summary: Vec<String> = Vec::new();

    let result = (|| -> anyhow::Result<()> {
        let version = read_plugin_version(&options.plugin_dir)?;

        if let (Some(line), action) =
            cleanup_legacy_claude_symlink(&options.home, options.dry_run)?
        {
            summary.push(line);
            if let Some(act) = action {
                undo.push(act);
            }
        }

        let cache_step = install_plugin_cache(
            claude_plugin_cache_path(&options.home),
            &options.plugin_dir,
            &options.backup_suffix,
            options.dry_run,
            options.force,
        )?;
        summary.push(cache_step.line);
        if let Some(action) = cache_step.action {
            undo.push(action);
        }

        let (line, action) = install_claude_marketplace_dir(options)?;
        summary.push(line);
        if let Some(action) = action {
            undo.push(action);
        }

        let (line, action) = upsert_claude_installed_plugins(options, &version)?;
        summary.push(line);
        if let Some(action) = action {
            undo.push(action);
        }

        let (line, action) = upsert_claude_known_marketplaces(options)?;
        summary.push(line);
        if let Some(action) = action {
            undo.push(action);
        }

        Ok(())
    })();

    match result {
        Ok(()) => Ok((summary, undo)),
        Err(err) => {
            run_undo(undo);
            Err(err)
        }
    }
}

pub fn install_claude(options: &InstallOptions) -> anyhow::Result<Vec<String>> {
    preflight_claude(options)?;
    let (summary, undo) = install_claude_steps(options)?;
    run_commit(undo);
    Ok(summary)
}

// ===========================================================================
// Codex install (TOML editing)
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

fn install_codex_plugin_cache(
    options: &InstallOptions,
) -> anyhow::Result<(InstallOutcome, String)> {
    let step = install_plugin_cache(
        codex_plugin_cache_path(&options.home),
        &options.plugin_dir,
        &options.backup_suffix,
        options.dry_run,
        options.force,
    )?;
    if let Some(action) = step.action {
        // codex_install_codex handles its own atomicity. The cache step is the
        // last write, so commit immediately to clean up any backup.
        let _ = action.commit();
    }
    Ok((step.outcome, step.line))
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

// ===========================================================================
// install_all
// ===========================================================================

pub fn install_all(options: &InstallOptions) -> anyhow::Result<Vec<String>> {
    preflight_claude(options)?;
    preflight_codex(options)?;

    let mut summary = Vec::new();
    summary.push("Claude:".to_string());
    let (claude_lines, claude_undo) = install_claude_steps(options)?;
    summary.extend(claude_lines);
    summary.push("Codex:".to_string());
    match install_codex(options) {
        Ok(lines) => {
            summary.extend(lines);
            run_commit(claude_undo);
            Ok(summary)
        }
        Err(err) => {
            if !options.dry_run {
                run_undo(claude_undo);
                return Err(err)
                    .context("failed to install Codex; rolled back Claude plugin install");
            }
            Err(err)
        }
    }
}

// ===========================================================================
// Public helpers
// ===========================================================================

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

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::symlink;
    use std::path::PathBuf;

    use serde_json::{json, Value};
    use tempfile::TempDir;

    use super::*;

    /// Build a `<root>/source/plugins/org-roam-toolkit` plugin layout with the
    /// minimum metadata files the installer requires.
    fn temp_plugin_with(root: &TempDir, version: &str) -> PathBuf {
        let source = root.path().join("source");
        let plugin = source.join("plugins").join(PLUGIN_NAME);
        fs::create_dir_all(plugin.join(".claude-plugin")).unwrap();
        fs::write(plugin.join("marker"), "ok").unwrap();
        fs::write(
            plugin.join(".claude-plugin").join("plugin.json"),
            format!(
                "{{\"name\":\"{}\",\"version\":\"{}\"}}\n",
                PLUGIN_NAME, version
            ),
        )
        .unwrap();
        let market_dir = source.join(".claude-plugin");
        fs::create_dir_all(&market_dir).unwrap();
        fs::write(
            market_dir.join("marketplace.json"),
            format!(
                "{{\"name\":\"{}\",\"plugins\":[{{\"name\":\"{}\",\"source\":\"./plugins/{}\"}}]}}\n",
                MARKETPLACE_NAME, PLUGIN_NAME, PLUGIN_NAME
            ),
        )
        .unwrap();
        plugin
    }

    fn temp_plugin(root: &TempDir) -> PathBuf {
        temp_plugin_with(root, "0.0.1")
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

    fn read_installed_plugins(home: &Path) -> Value {
        let raw =
            fs::read_to_string(home.join(".claude/plugins/installed_plugins.json")).unwrap();
        serde_json::from_str(&raw).unwrap()
    }

    fn read_known_marketplaces(home: &Path) -> Value {
        let raw =
            fs::read_to_string(home.join(".claude/plugins/known_marketplaces.json")).unwrap();
        serde_json::from_str(&raw).unwrap()
    }

    #[test]
    fn claude_install_creates_cache_and_metadata() {
        let root = TempDir::new().unwrap();
        let plugin = temp_plugin_with(&root, "1.2.3");
        let opts = options(root.path().join("home"), plugin.clone());

        install_claude(&opts).unwrap();

        let cache = opts
            .home
            .join(".claude/plugins/cache/org-roam-toolkit/org-roam-toolkit/local");
        assert!(cache.join("marker").exists());
        assert!(!cache.is_symlink());

        let installed = read_installed_plugins(&opts.home);
        let entry = installed["plugins"][PLUGIN_KEY][0].clone();
        assert_eq!(entry["scope"], "user");
        assert_eq!(entry["installPath"], cache.to_str().unwrap());
        assert_eq!(entry["version"], "1.2.3");
        assert!(entry["installedAt"].is_string());
        assert!(entry["lastUpdated"].is_string());

        let known = read_known_marketplaces(&opts.home);
        assert_eq!(known[MARKETPLACE_NAME]["source"]["source"], "local");
        let market_dir = opts
            .home
            .join(".claude/plugins/marketplaces/org-roam-toolkit");
        assert_eq!(
            known[MARKETPLACE_NAME]["installLocation"]
                .as_str()
                .unwrap(),
            market_dir.to_str().unwrap()
        );

        let market_file = market_dir.join(".claude-plugin/marketplace.json");
        let market_value: Value =
            serde_json::from_str(&fs::read_to_string(&market_file).unwrap()).unwrap();
        assert_eq!(
            market_value["plugins"][0]["source"].as_str().unwrap(),
            cache.to_str().unwrap()
        );
    }

    #[test]
    fn claude_install_removes_legacy_symlink() {
        let root = TempDir::new().unwrap();
        let plugin = temp_plugin(&root);
        let opts = options(root.path().join("home"), plugin.clone());
        let legacy = opts.home.join(".claude/plugins/org-roam-toolkit");
        fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        symlink(&plugin, &legacy).unwrap();

        install_claude(&opts).unwrap();

        assert!(!legacy.exists() && !legacy.is_symlink());
        let cache = opts
            .home
            .join(".claude/plugins/cache/org-roam-toolkit/org-roam-toolkit/local");
        assert!(cache.join("marker").exists());
    }

    #[test]
    fn claude_install_refuses_legacy_path_that_is_a_directory() {
        let root = TempDir::new().unwrap();
        let plugin = temp_plugin(&root);
        let opts = options(root.path().join("home"), plugin);
        let legacy = opts.home.join(".claude/plugins/org-roam-toolkit");
        fs::create_dir_all(&legacy).unwrap();

        let err = install_claude(&opts).unwrap_err().to_string();

        assert!(err.contains("not a symlink"));
    }

    #[test]
    fn claude_install_is_idempotent_when_already_installed() {
        let root = TempDir::new().unwrap();
        let plugin = temp_plugin_with(&root, "0.9.0");
        let opts = options(root.path().join("home"), plugin);

        install_claude(&opts).unwrap();
        let summary = install_claude(&opts).unwrap();

        assert!(summary
            .iter()
            .any(|line| line.contains("already configured") && line.contains(PLUGIN_KEY)));
        assert!(summary
            .iter()
            .any(|line| line.contains("already configured") && line.contains(MARKETPLACE_NAME)));
    }

    #[test]
    fn claude_dry_run_does_not_write_anything() {
        let root = TempDir::new().unwrap();
        let plugin = temp_plugin(&root);
        let mut opts = options(root.path().join("home"), plugin);
        opts.dry_run = true;

        let summary = install_claude(&opts).unwrap();

        assert!(!opts.home.join(".claude/plugins").exists());
        assert!(summary.iter().any(|line| line.contains("would cache")));
        assert!(summary.iter().any(|line| line.contains("would write")));
        assert!(summary.iter().any(|line| line.contains("would create")));
    }

    #[test]
    fn claude_install_replaces_existing_cache() {
        let root = TempDir::new().unwrap();
        let plugin = temp_plugin_with(&root, "1.0.0");
        let opts = options(root.path().join("home"), plugin.clone());

        install_claude(&opts).unwrap();

        // Bump the plugin version + rebuild the source plugin.
        fs::write(
            plugin.join(".claude-plugin/plugin.json"),
            format!("{{\"name\":\"{}\",\"version\":\"1.1.0\"}}\n", PLUGIN_NAME),
        )
        .unwrap();
        fs::write(plugin.join("marker"), "updated").unwrap();

        install_claude(&opts).unwrap();

        let cache = opts
            .home
            .join(".claude/plugins/cache/org-roam-toolkit/org-roam-toolkit/local");
        assert_eq!(fs::read_to_string(cache.join("marker")).unwrap(), "updated");
        let installed = read_installed_plugins(&opts.home);
        assert_eq!(
            installed["plugins"][PLUGIN_KEY][0]["version"]
                .as_str()
                .unwrap(),
            "1.1.0"
        );
    }

    #[test]
    fn claude_install_preserves_unrelated_installed_plugins() {
        let root = TempDir::new().unwrap();
        let plugin = temp_plugin(&root);
        let opts = options(root.path().join("home"), plugin);
        let installed_path = opts.home.join(".claude/plugins/installed_plugins.json");
        fs::create_dir_all(installed_path.parent().unwrap()).unwrap();
        let existing = json!({
            "version": 2,
            "plugins": {
                "other@elsewhere": [
                    { "scope": "user", "installPath": "/tmp/other" }
                ]
            }
        });
        fs::write(
            &installed_path,
            serde_json::to_string_pretty(&existing).unwrap(),
        )
        .unwrap();

        install_claude(&opts).unwrap();

        let installed = read_installed_plugins(&opts.home);
        assert_eq!(installed["version"], 2);
        assert_eq!(
            installed["plugins"]["other@elsewhere"][0]["installPath"]
                .as_str()
                .unwrap(),
            "/tmp/other"
        );
        assert!(installed["plugins"][PLUGIN_KEY][0]["installPath"]
            .is_string());
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
    fn install_all_configures_claude_and_codex() {
        let root = TempDir::new().unwrap();
        let plugin = temp_plugin(&root);
        let opts = options(root.path().join("home"), plugin.clone());

        let summary = install_all(&opts).unwrap();

        assert_eq!(summary.first().map(String::as_str), Some("Claude:"));
        assert!(summary.iter().any(|line| line == "Codex:"));

        let claude_cache = opts
            .home
            .join(".claude/plugins/cache/org-roam-toolkit/org-roam-toolkit/local");
        assert!(claude_cache.join("marker").exists());
        let installed = read_installed_plugins(&opts.home);
        assert_eq!(
            installed["plugins"][PLUGIN_KEY][0]["installPath"]
                .as_str()
                .unwrap(),
            claude_cache.to_str().unwrap()
        );

        assert!(codex_plugin_cache_path(&opts.home).join("marker").exists());
        let config = fs::read_to_string(opts.home.join(".codex/config.toml")).unwrap();
        assert!(config.contains("[plugins.\"org-roam-toolkit@org-roam-toolkit\"]\nenabled = true"));
        assert!(config.contains("[mcp_servers.org-roam]"));
    }

    #[test]
    fn install_all_does_not_touch_claude_when_codex_config_conflicts() {
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
        assert!(!opts
            .home
            .join(".claude/plugins/cache/org-roam-toolkit/org-roam-toolkit/local")
            .exists());
        assert!(!opts
            .home
            .join(".claude/plugins/installed_plugins.json")
            .exists());
        assert!(!codex_plugin_cache_path(&opts.home).exists());
    }

    #[test]
    fn install_all_rolls_back_claude_install_when_codex_cache_fails() {
        let root = TempDir::new().unwrap();
        let plugin = temp_plugin(&root);
        let opts = options(root.path().join("home"), plugin);
        let config_path = opts.home.join(".codex/config.toml");
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        fs::write(&config_path, "model = \"gpt-5.5\"\n").unwrap();
        fs::write(opts.home.join(".codex/plugins"), "not a directory").unwrap();

        let err = install_all(&opts).unwrap_err().to_string();

        assert!(err.contains("rolled back Claude plugin install"));
        assert!(!opts
            .home
            .join(".claude/plugins/cache/org-roam-toolkit/org-roam-toolkit/local")
            .exists());
        assert!(!opts
            .home
            .join(".claude/plugins/installed_plugins.json")
            .exists());
        assert!(!opts
            .home
            .join(".claude/plugins/known_marketplaces.json")
            .exists());
        assert_eq!(
            fs::read_to_string(&config_path).unwrap(),
            "model = \"gpt-5.5\"\n"
        );
    }

    #[test]
    fn install_all_restores_existing_claude_metadata_when_codex_fails() {
        let root = TempDir::new().unwrap();
        let plugin = temp_plugin(&root);
        let opts = options(root.path().join("home"), plugin);
        let installed_path = opts.home.join(".claude/plugins/installed_plugins.json");
        fs::create_dir_all(installed_path.parent().unwrap()).unwrap();
        let prior = json!({
            "version": 2,
            "plugins": {
                "other@elsewhere": [{ "scope": "user", "installPath": "/tmp/other" }]
            }
        });
        fs::write(
            &installed_path,
            serde_json::to_string_pretty(&prior).unwrap(),
        )
        .unwrap();
        let config_path = opts.home.join(".codex/config.toml");
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        fs::write(&config_path, "model = \"gpt-5.5\"\n").unwrap();
        fs::write(opts.home.join(".codex/plugins"), "not a directory").unwrap();

        let err = install_all(&opts).unwrap_err().to_string();

        assert!(err.contains("rolled back Claude plugin install"));
        let restored: Value =
            serde_json::from_str(&fs::read_to_string(&installed_path).unwrap()).unwrap();
        assert_eq!(restored, prior);
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
