use std::path::PathBuf;

use anyhow::Context;
use clap::{Parser, Subcommand};
use ortk_agent_install::{
    backup_suffix_now, default_plugin_dir, install_all, install_claude, install_codex,
    InstallOptions,
};

#[derive(Debug, Parser)]
#[command(
    name = "ortk-agent-install",
    about = "Install org-roam-toolkit plugin support into Claude Code and Codex"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Path to the org-roam-toolkit plugin directory.
    #[arg(long, value_name = "DIR", global = true)]
    plugin_dir: Option<PathBuf>,

    /// Print actions without changing files.
    #[arg(long, global = true)]
    dry_run: bool,

    /// Replace conflicting existing links or Codex MCP config.
    #[arg(long, global = true)]
    force: bool,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Install Claude Code plugin support.
    Claude,
    /// Install Codex plugin and MCP support.
    Codex,
    /// Install both Claude Code and Codex support.
    All,
}

fn options(cli: &Cli) -> anyhow::Result<InstallOptions> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .context("HOME is not set")?;
    options_with_home(cli, home)
}

fn options_with_home(cli: &Cli, home: PathBuf) -> anyhow::Result<InstallOptions> {
    if home.as_os_str().is_empty() {
        anyhow::bail!("HOME is empty");
    }

    let plugin_dir = match &cli.plugin_dir {
        Some(path) => {
            let canonical = path
                .canonicalize()
                .with_context(|| format!("plugin directory does not exist: {}", path.display()))?;
            if !canonical.is_dir() {
                anyhow::bail!("plugin directory is not a directory: {}", path.display());
            }
            canonical
        }
        None => default_plugin_dir()?,
    };

    Ok(InstallOptions {
        home,
        plugin_dir,
        dry_run: cli.dry_run,
        force: cli.force,
        backup_suffix: backup_suffix_now(),
    })
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let options = options(&cli)?;
    let summary = match &cli.command {
        Command::Claude => install_claude(&options)?,
        Command::Codex => install_codex(&options)?,
        Command::All => install_all(&options)?,
    };

    for line in summary {
        println!("{line}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    #[test]
    fn top_level_help_lists_global_install_options() {
        let help = Cli::command().render_long_help().to_string();

        assert!(help.contains("--plugin-dir <DIR>"));
        assert!(help.contains("--dry-run"));
        assert!(help.contains("--force"));
    }

    #[test]
    fn global_install_options_parse_after_subcommand() {
        let cli = Cli::try_parse_from([
            "ortk-agent-install",
            "all",
            "--dry-run",
            "--plugin-dir",
            "./plugins/org-roam-toolkit",
        ])
        .unwrap();

        assert!(matches!(cli.command, Command::All));
        assert!(cli.dry_run);
        assert_eq!(
            cli.plugin_dir,
            Some(PathBuf::from("./plugins/org-roam-toolkit"))
        );
    }

    #[test]
    fn options_canonicalizes_explicit_plugin_dir() {
        let cli = Cli::try_parse_from([
            "ortk-agent-install",
            "all",
            "--plugin-dir",
            "../../plugins/org-roam-toolkit",
        ])
        .unwrap();

        let opts = options_with_home(&cli, PathBuf::from("/tmp/home")).unwrap();

        assert!(opts.plugin_dir.is_absolute());
        assert_eq!(
            opts.plugin_dir,
            PathBuf::from("../../plugins/org-roam-toolkit")
                .canonicalize()
                .unwrap()
        );
    }

    #[test]
    fn options_rejects_missing_explicit_plugin_dir() {
        let root = tempfile::TempDir::new().unwrap();
        let missing = root.path().join("plugins/org-roam-toolkit");
        let cli = Cli::try_parse_from([
            "ortk-agent-install",
            "all",
            "--plugin-dir",
            missing.to_str().unwrap(),
        ])
        .unwrap();

        let err = options_with_home(&cli, PathBuf::from("/tmp/home"))
            .unwrap_err()
            .to_string();

        assert!(err.contains("plugin directory"));
    }

    #[test]
    fn options_rejects_empty_home() {
        let cli = Cli::try_parse_from(["ortk-agent-install", "all"]).unwrap();

        let err = options_with_home(&cli, PathBuf::new())
            .unwrap_err()
            .to_string();

        assert!(err.contains("HOME is empty"));
    }
}
