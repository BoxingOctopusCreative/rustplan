use clap::{Args, Subcommand};

#[derive(Debug, Args)]
pub struct IpArgs {
    #[command(subcommand)]
    pub command: IpCommand,
}

#[derive(Debug, Subcommand)]
pub enum IpCommand {
    /// Display IP leases for an interface
    Leases(LeasesArgs),
}

#[derive(Debug, Args)]
pub struct LeasesArgs {
    /// Overrides the root directory
    #[arg(long, value_name = "ROOT_DIR")]
    pub root_dir: Option<String>,
    /// Interface to show leases for
    pub interface: String,
}

pub fn run(args: &IpArgs, _debug: bool) -> anyhow::Result<()> {
    match &args.command {
        IpCommand::Leases(leases_args) => run_leases(leases_args),
    }
}

fn run_leases(args: &LeasesArgs) -> anyhow::Result<()> {
    let root = args.root_dir.as_deref().unwrap_or("/");
    // systemd-networkd stores leases in /run/systemd/netif/leases/<ifindex>
    // NetworkManager in /var/lib/NetworkManager/internal-*.lease
    // We look in the networkd location first
    let leases_dir = std::path::Path::new(root).join("run/systemd/netif/leases");
    if !leases_dir.exists() {
        eprintln!("No lease directory found at {}", leases_dir.display());
        return Ok(());
    }
    // Find the ifindex for the interface
    let ifindex_path = std::path::Path::new("/sys/class/net")
        .join(&args.interface)
        .join("ifindex");
    let ifindex = std::fs::read_to_string(&ifindex_path)
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let lease_file = leases_dir.join(&ifindex);
    if lease_file.exists() {
        println!("{}", std::fs::read_to_string(lease_file)?);
    } else {
        eprintln!("No lease found for {}", args.interface);
    }
    Ok(())
}
