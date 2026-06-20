use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::Result;
use clap::Args;

#[derive(Debug, Args)]
pub struct StatusArgs {
    /// Overrides the root directory for netplan files
    #[arg(long, value_name = "ROOT_DIR")]
    pub root_dir: Option<String>,
    /// Show all interfaces, including unmanaged
    #[arg(short, long)]
    pub all: bool,
    /// Verbose output
    #[arg(short, long)]
    pub verbose: bool,
    /// Output format: yaml or json
    #[arg(short, long, value_name = "FORMAT")]
    pub format: Option<String>,
    /// Show diff between config and running state
    #[arg(long)]
    pub diff: bool,
    /// Only show diff (no full status)
    #[arg(long)]
    pub diff_only: bool,
    /// Optional interface name to filter output
    pub interface: Option<String>,
}

struct IfaceInfo {
    name: String,
    operstate: String,
    addresses: Vec<String>,
    managed: bool,
}

pub fn run(args: &StatusArgs, _debug: bool) -> Result<()> {
    let root = args.root_dir.as_deref().unwrap_or("/");
    let cfg = crate::config::loader::load(Path::new(root))?;

    let managed_names: std::collections::HashSet<String> = cfg.ethernets.keys()
        .chain(cfg.wifis.keys())
        .chain(cfg.bridges.keys())
        .chain(cfg.bonds.keys())
        .chain(cfg.vlans.keys())
        .cloned()
        .collect();

    let mut ifaces = collect_interfaces()?;

    if let Some(filter) = &args.interface {
        ifaces.retain(|i| &i.name == filter);
    } else if !args.all {
        ifaces.retain(|i| i.managed);
    }

    for iface in &mut ifaces {
        iface.managed = managed_names.contains(&iface.name);
    }

    match args.format.as_deref() {
        Some("json") => print_json(&ifaces),
        _ => print_table(&ifaces, args.verbose),
    }

    Ok(())
}

fn collect_interfaces() -> Result<Vec<IfaceInfo>> {
    let sys_net = Path::new("/sys/class/net");
    let mut ifaces = Vec::new();

    let ip_output = Command::new("ip")
        .args(["-j", "addr"])
        .output()
        .ok();

    let addr_map = ip_output
        .and_then(|o| serde_yaml::from_slice::<serde_yaml::Value>(&o.stdout).ok())
        .and_then(|v| {
            // ip -j addr returns a JSON array
            Some(v)
        });

    if !sys_net.exists() {
        return Ok(ifaces);
    }

    for entry in fs::read_dir(sys_net)?.filter_map(|e| e.ok()) {
        let name = entry.file_name().to_string_lossy().to_string();
        let operstate = fs::read_to_string(entry.path().join("operstate"))
            .unwrap_or_default()
            .trim()
            .to_string();

        let addresses = extract_addresses(&addr_map, &name);

        ifaces.push(IfaceInfo {
            name,
            operstate,
            addresses,
            managed: false,
        });
    }

    ifaces.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(ifaces)
}

fn extract_addresses(ip_data: &Option<serde_yaml::Value>, name: &str) -> Vec<String> {
    let Some(serde_yaml::Value::Sequence(arr)) = ip_data else { return vec![]; };
    for item in arr {
        if item.get("ifname").and_then(|v| v.as_str()) == Some(name) {
            if let Some(serde_yaml::Value::Sequence(addrs)) = item.get("addr_info") {
                return addrs.iter().filter_map(|a| {
                    let local = a.get("local")?.as_str()?;
                    let prefix = a.get("prefixlen")?.as_u64()?;
                    Some(format!("{local}/{prefix}"))
                }).collect();
            }
        }
    }
    vec![]
}

fn print_table(ifaces: &[IfaceInfo], verbose: bool) {
    for i in ifaces {
        let managed = if i.managed { "managed" } else { "unmanaged" };
        println!("{:<15} {:>10}  {}", i.name, i.operstate, managed);
        if verbose {
            for addr in &i.addresses {
                println!("  {addr}");
            }
        }
    }
}

fn print_json(ifaces: &[IfaceInfo]) {
    println!("[");
    for (idx, i) in ifaces.iter().enumerate() {
        let comma = if idx + 1 < ifaces.len() { "," } else { "" };
        println!("  {{\"name\":{:?},\"operstate\":{:?},\"managed\":{},\"addresses\":{:?}}}{comma}",
            i.name, i.operstate, i.managed, i.addresses);
    }
    println!("]");
}
