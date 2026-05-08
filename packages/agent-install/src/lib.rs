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

pub fn install_codex(_options: &InstallOptions) -> anyhow::Result<Vec<String>> {
    anyhow::bail!("install_codex is not implemented")
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
}
