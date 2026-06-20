use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Args;

use crate::config::types::Renderer;

#[derive(Debug, Args)]
pub struct TryArgs {
    /// Path to a YAML config to try
    #[arg(long, value_name = "CONFIG_FILE")]
    pub config_file: Option<String>,
    /// Rollback timeout in seconds (default: 120)
    #[arg(long, default_value = "120", value_name = "TIMEOUT")]
    pub timeout: u64,
    /// Directory with previous network state
    #[arg(long, value_name = "STATE_DIR")]
    pub state: Option<String>,
}

pub fn run(args: &TryArgs, debug: bool) -> Result<()> {
    let root = Path::new("/");
    let cfg = crate::config::loader::load(root)?;
    let renderer = cfg.renderer.as_ref().unwrap_or(&Renderer::Networkd);

    // Save snapshot of current backend files
    let snapshot_dir = tempfile::TempDir::new().context("creating snapshot dir")?;
    let backend_dir = backend_output_dir(renderer, root);
    if backend_dir.exists() {
        copy_dir_contents(&backend_dir, snapshot_dir.path())?;
    }

    // If a specific config file was provided, copy it into /etc/netplan temporarily
    let _temp_config: Option<std::fs::File> = if let Some(config_path) = &args.config_file {
        let dest = Path::new("/etc/netplan/_rustplan-try.yaml");
        std::fs::copy(config_path, dest)
            .with_context(|| format!("copying {config_path}"))?;
        Some(std::fs::File::open(dest)?)
    } else {
        None
    };

    // Apply the configuration
    let generate_args = crate::cli::generate::GenerateArgs { root_dir: None, mapping: None };
    crate::cli::generate::run(&generate_args, debug)?;

    let cfg2 = crate::config::loader::load(root)?;
    let renderer2 = cfg2.renderer.as_ref().unwrap_or(&Renderer::Networkd);
    reload_backend(renderer2, debug)?;

    eprintln!("Do you want to keep these settings? Press Enter to accept or wait {timeout}s to revert.",
        timeout = args.timeout);

    // Wait for Enter with timeout
    let accepted = wait_for_enter(Duration::from_secs(args.timeout));

    // Clean up temp config
    if args.config_file.is_some() {
        let _ = std::fs::remove_file("/etc/netplan/_rustplan-try.yaml");
    }

    if accepted {
        eprintln!("Configuration accepted.");
    } else {
        eprintln!("Reverting to previous configuration...");
        // Restore snapshot
        restore_snapshot(snapshot_dir.path(), &backend_dir)?;
        reload_backend(renderer, debug)?;
    }

    Ok(())
}

fn backend_output_dir(renderer: &Renderer, root: &Path) -> PathBuf {
    match renderer {
        Renderer::Networkd => root.join("run/systemd/network"),
        Renderer::NetworkManager => root.join("run/NetworkManager/system-connections"),
    }
}

fn copy_dir_contents(src: &Path, dst: &Path) -> Result<()> {
    for entry in std::fs::read_dir(src)?.filter_map(|e| e.ok()) {
        if entry.file_type()?.is_file() {
            std::fs::copy(entry.path(), dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}

fn restore_snapshot(snapshot: &Path, backend_dir: &Path) -> Result<()> {
    // Remove current backend files written by rustplan
    if backend_dir.exists() {
        for entry in std::fs::read_dir(backend_dir)?.filter_map(|e| e.ok()) {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with("10-netplan-") || name_str.starts_with("netplan-") {
                std::fs::remove_file(entry.path()).ok();
            }
        }
    }
    copy_dir_contents(snapshot, backend_dir)?;
    Ok(())
}

fn reload_backend(renderer: &Renderer, debug: bool) -> Result<()> {
    match renderer {
        Renderer::Networkd => {
            crate::cli::apply::run_cmd("networkctl", &["reload"], debug)
        }
        Renderer::NetworkManager => {
            crate::cli::apply::run_cmd("nmcli", &["connection", "reload"], debug)
        }
    }
}

fn wait_for_enter(timeout: Duration) -> bool {
    use std::sync::mpsc;
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut buf = [0u8; 1];
        let _ = io::stdin().read(&mut buf);
        let _ = tx.send(());
    });
    rx.recv_timeout(timeout).is_ok()
}
