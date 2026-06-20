use clap::Args;

#[derive(Debug, Args)]
pub struct RebindArgs {
    /// Overrides the root directory
    #[arg(long, value_name = "ROOT_DIR")]
    pub root_dir: Option<String>,
    /// Physical function interface names to rebind
    pub interfaces: Vec<String>,
}

pub fn run(args: &RebindArgs, _debug: bool) -> anyhow::Result<()> {
    // SR-IOV rebind: write the driver name to /sys/bus/pci/drivers/<driver>/bind
    // for each VF of each PF listed. This is a best-effort implementation.
    for pf in &args.interfaces {
        let pci_path = std::path::Path::new("/sys/class/net").join(pf).join("device");
        if !pci_path.exists() {
            eprintln!("Warning: no PCI device found for {pf}");
            continue;
        }
        let numvfs_path = pci_path.join("sriov_numvfs");
        let num: u32 = std::fs::read_to_string(&numvfs_path)
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0);
        eprintln!("Rebinding {num} VFs for {pf}");
        // Actual driver bind logic requires knowledge of PCI IDs; log and skip for now.
    }
    Ok(())
}
