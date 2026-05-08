use anyhow::{bail, Context};
use chrono::Local;
use std::env;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

const PLUGIN_NAME: &str = "org-roam-toolkit";
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
    let exe = env::current_exe().context("resolve current executable path")?;
    if let Some(prefix) = exe.parent().and_then(Path::parent) {
        let installed = prefix.join("libexec/plugins").join(PLUGIN_NAME);
        if installed.exists() {
            return Ok(installed);
        }
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
                return Some(rhs.trim().trim_matches('"').to_string());
            }
        }
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

pub fn install_codex(options: &InstallOptions) -> anyhow::Result<Vec<String>> {
    let codex_dir = options.home.join(".codex");
    let config_path = codex_dir.join("config.toml");
    let planned_config = if config_path.exists() {
        let current = fs::read_to_string(&config_path)
            .with_context(|| format!("read {}", config_path.display()))?;
        desired_codex_config(&current, options.force)?
    } else {
        Some(CODEX_MCP_BLOCK.to_string())
    };

    let mut summary = Vec::new();
    let (_, link_line) = install_plugin_symlink(
        &options.home,
        ".codex",
        &options.plugin_dir,
        options.dry_run,
        options.force,
    )?;
    summary.push(link_line);

    match (config_path.exists(), planned_config) {
        (false, Some(_)) if options.dry_run => {
            summary.push(format!(
                "would create: {} with org-roam MCP server",
                config_path.display()
            ));
        }
        (false, Some(next)) => {
            fs::create_dir_all(&codex_dir)
                .with_context(|| format!("create {}", codex_dir.display()))?;
            fs::write(&config_path, next)
                .with_context(|| format!("write {}", config_path.display()))?;
            summary.push(format!("created: {}", config_path.display()));
        }
        (true, None) => {
            summary.push(format!(
                "already configured: {} has [mcp_servers.org-roam]",
                config_path.display()
            ));
        }
        (true, Some(_)) if options.dry_run => {
            summary.push(format!(
                "would update: {} with [mcp_servers.org-roam]",
                config_path.display()
            ));
        }
        (true, Some(next)) => {
            let backup = write_backup(&config_path, &options.backup_suffix)?;
            fs::write(&config_path, next)
                .with_context(|| format!("write {}", config_path.display()))?;
            summary.push(format!("backup: {}", backup.display()));
            summary.push(format!("updated: {}", config_path.display()));
        }
        (false, None) => unreachable!("missing config always needs creation"),
    }
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

        assert_eq!(
            fs::read_to_string(&config_path).unwrap(),
            "[mcp_servers.org-roam]\ncommand = \"ortk-mcp\"\n",
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

        assert_eq!(
            fs::read_to_string(&config_path).unwrap(),
            "[mcp_servers.org-roam] # org-roam server\ncommand = \"ortk-mcp\"\n",
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

        assert_eq!(
            fs::read_to_string(&config_path).unwrap(),
            "[mcp_servers.org-roam]\ncommand = \"ortk-mcp\" # installed by org-roam-toolkit\n",
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
    fn codex_install_does_not_create_symlink_when_config_conflicts() {
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
        assert!(!opts.home.join(".codex/plugins/org-roam-toolkit").exists());
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
        assert!(config.contains("[mcp_servers.org-roam]\ncommand = \"ortk-mcp\""));
        assert!(!config.contains("args = [\"bad\"]"));
        assert!(config.contains("[projects.\"/tmp\"] # local project\ntrust_level = \"trusted\""));
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
        assert_eq!(
            fs::read_link(opts.home.join(".codex/plugins/org-roam-toolkit")).unwrap(),
            opts.plugin_dir
        );
        let config = fs::read_to_string(opts.home.join(".codex/config.toml")).unwrap();
        assert!(config.contains("[mcp_servers.org-roam]"));
        assert!(config.contains("command = \"ortk-mcp\""));
    }

    #[test]
    fn default_backup_suffix_has_timestamp_shape() {
        let suffix = backup_suffix_now();

        assert_eq!(suffix.len(), 14);
        assert!(suffix.chars().all(|ch| ch.is_ascii_digit()));
    }
}
