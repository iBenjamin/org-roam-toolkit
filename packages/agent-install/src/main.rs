use std::path::PathBuf;

use anyhow::Context;
use clap::{Parser, Subcommand};
use ortk_agent_install::{backup_suffix_now, install_all, install_claude, install_codex, InstallOptions};

#[derive(Debug, Parser)]
#[command(
    name = "ortk-agent-install",
    about = "Install org-roam-toolkit plugin support into Claude Code and Codex"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Print actions without changing files.
    #[arg(long, global = true)]
    dry_run: bool,

    /// Replace conflicting Codex MCP config.
    #[arg(long, global = true)]
    force: bool,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Print the Claude Code plugin install instructions and clean up any
    /// legacy symlink at ~/.claude/plugins/org-roam-toolkit.
    Claude,
    /// Write [mcp_servers.org-roam] and [plugins."org-roam-toolkit@..."]
    /// into ~/.codex/config.toml.
    Codex,
    /// Run both `claude` and `codex`.
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

    Ok(InstallOptions {
        home,
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

        assert!(help.contains("--dry-run"));
        assert!(help.contains("--force"));
    }

    #[test]
    fn global_install_options_parse_after_subcommand() {
        let cli = Cli::try_parse_from(["ortk-agent-install", "all", "--dry-run"]).unwrap();

        assert!(matches!(cli.command, Command::All));
        assert!(cli.dry_run);
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
