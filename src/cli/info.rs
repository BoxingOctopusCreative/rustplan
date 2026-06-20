use anyhow::Result;
use clap::Args;

#[derive(Debug, Args)]
pub struct InfoArgs {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
    /// Output as YAML
    #[arg(long)]
    pub yaml: bool,
}

const FEATURES: &[(&str, &str)] = &[
    ("generate", "Generate backend-specific configuration"),
    ("apply", "Apply network configuration"),
    ("try", "Apply with automatic rollback"),
    ("get", "Read configuration values"),
    ("set", "Write configuration values"),
    ("status", "Query network interface status"),
    ("backend:networkd", "systemd-networkd backend"),
    ("backend:NetworkManager", "NetworkManager backend"),
];

pub fn run(args: &InfoArgs, _debug: bool) -> Result<()> {
    if args.json {
        let pairs: Vec<_> = FEATURES.iter()
            .map(|(k, v)| format!("  {k:?}: {v:?}"))
            .collect();
        println!("{{\n{}\n}}", pairs.join(",\n"));
    } else if args.yaml {
        for (k, v) in FEATURES {
            println!("{k}: {v}");
        }
    } else {
        println!("rustplan feature support:");
        for (k, v) in FEATURES {
            println!("  {k}: {v}");
        }
    }
    Ok(())
}
