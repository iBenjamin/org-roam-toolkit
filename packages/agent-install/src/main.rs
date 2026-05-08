use std::path::PathBuf;

use anyhow::Context;
use clap::{Args, Parser, Subcommand};
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
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Install Claude Code plugin support.
    Claude(InstallArgs),
    /// Install Codex plugin and MCP support.
    Codex(InstallArgs),
    /// Install both Claude Code and Codex support.
    All(InstallArgs),
}

#[derive(Debug, Args)]
struct InstallArgs {
    /// Path to the org-roam-toolkit plugin directory.
    #[arg(long, value_name = "DIR")]
    plugin_dir: Option<PathBuf>,

    /// Print actions without changing files.
    #[arg(long)]
    dry_run: bool,

    /// Replace conflicting existing links or Codex MCP config.
    #[arg(long)]
    force: bool,
}

fn options(args: &InstallArgs) -> anyhow::Result<InstallOptions> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .context("HOME is not set")?;
    let plugin_dir = match &args.plugin_dir {
        Some(path) => path.clone(),
        None => default_plugin_dir()?,
    };

    Ok(InstallOptions {
        home,
        plugin_dir,
        dry_run: args.dry_run,
        force: args.force,
        backup_suffix: backup_suffix_now(),
    })
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let summary = match &cli.command {
        Command::Claude(args) => install_claude(&options(args)?)?,
        Command::Codex(args) => install_codex(&options(args)?)?,
        Command::All(args) => install_all(&options(args)?)?,
    };

    for line in summary {
        println!("{line}");
    }

    Ok(())
}
