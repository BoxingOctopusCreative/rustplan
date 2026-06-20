mod backends;
mod cli;
mod config;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "rustplan",
    about = "Network configuration tool (netplan-compatible)",
    version
)]
struct Cli {
    /// Enable debug output
    #[arg(long, global = true)]
    debug: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Apply current network configuration
    Apply(cli::apply::ApplyArgs),
    /// Generate backend-specific configuration files
    Generate(cli::generate::GenerateArgs),
    /// Try applying configuration with automatic rollback
    Try(cli::try_cmd::TryArgs),
    /// Get a network configuration setting
    Get(cli::get::GetArgs),
    /// Set a network configuration value
    Set(cli::set::SetArgs),
    /// Show available features
    Info(cli::info::InfoArgs),
    /// Query networking state of the running system
    Status(cli::status::StatusArgs),
    /// Retrieve IP information
    Ip(cli::ip::IpArgs),
    /// Rebind SR-IOV virtual functions to their driver
    Rebind(cli::rebind::RebindArgs),
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("Error: {e:#}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    let debug = cli.debug;
    match cli.command {
        Command::Apply(args) => cli::apply::run(&args, debug),
        Command::Generate(args) => cli::generate::run(&args, debug),
        Command::Try(args) => cli::try_cmd::run(&args, debug),
        Command::Get(args) => cli::get::run(&args, debug),
        Command::Set(args) => cli::set::run(&args, debug),
        Command::Info(args) => cli::info::run(&args, debug),
        Command::Status(args) => cli::status::run(&args, debug),
        Command::Ip(args) => cli::ip::run(&args, debug),
        Command::Rebind(args) => cli::rebind::run(&args, debug),
    }
}
