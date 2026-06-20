use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context};
use clap::Args;

use crate::config::types::Renderer;

#[derive(Debug, Args)]
pub struct ApplyArgs {
    /// Only apply SR-IOV configuration
    #[arg(long)]
    pub sriov_only: bool,
    /// Only clean up OVS configuration
    #[arg(long)]
    pub only_ovs_cleanup: bool,
    /// Directory with previous network state
    #[arg(long, value_name = "STATE_DIR")]
    pub state: Option<String>,
}

pub fn run(args: &ApplyArgs, debug: bool) -> anyhow::Result<()> {
    // generate always uses the real root
    let generate_args = crate::cli::generate::GenerateArgs {
        root_dir: None,
        mapping: None,
    };
    crate::cli::generate::run(&generate_args, debug)?;

    if args.sriov_only || args.only_ovs_cleanup {
        return Ok(());
    }

    let cfg = crate::config::loader::load(Path::new("/"))?;
    let renderer = cfg.renderer.as_ref().unwrap_or(&Renderer::Networkd);

    match renderer {
        Renderer::Networkd => reload_networkd(debug),
        Renderer::NetworkManager => reload_nm(debug),
    }
}

fn reload_networkd(debug: bool) -> anyhow::Result<()> {
    run_cmd("networkctl", &["reload"], debug)?;
    Ok(())
}

fn reload_nm(debug: bool) -> anyhow::Result<()> {
    run_cmd("nmcli", &["connection", "reload"], debug)?;
    Ok(())
}

pub fn run_cmd(program: &str, args: &[&str], debug: bool) -> anyhow::Result<()> {
    if debug {
        eprintln!("[debug] running: {program} {}", args.join(" "));
    }
    let status = Command::new(program)
        .args(args)
        .status()
        .with_context(|| format!("failed to run {program}"))?;
    if !status.success() {
        bail!("{program} exited with {status}");
    }
    Ok(())
}
