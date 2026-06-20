use std::path::Path;

use anyhow::Context;
use clap::Args;

use crate::backends::{Backend, OutputFile};
use crate::backends::networkd::Networkd;
use crate::backends::nm::NetworkManager;
use crate::config::types::Renderer;

#[derive(Debug, Args)]
pub struct GenerateArgs {
    /// Overrides the root directory for netplan and generated files
    #[arg(long, value_name = "ROOT_DIR")]
    pub root_dir: Option<String>,
    /// Only generate configuration for this interface
    #[arg(long, value_name = "MAPPING")]
    pub mapping: Option<String>,
}

pub fn run(args: &GenerateArgs, debug: bool) -> anyhow::Result<()> {
    let root = args.root_dir.as_deref().unwrap_or("/");
    let root_path = Path::new(root);

    let cfg = crate::config::loader::load(root_path)?;

    let renderer = cfg.renderer.as_ref().unwrap_or(&Renderer::Networkd);
    if debug {
        eprintln!("[debug] renderer: {renderer:?}");
    }

    let files: Vec<OutputFile> = match renderer {
        Renderer::Networkd => Networkd.generate(&cfg, root_path)?,
        Renderer::NetworkManager => NetworkManager.generate(&cfg, root_path)?,
    };

    for file in &files {
        if let Some(parent) = file.path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        std::fs::write(&file.path, &file.content)
            .with_context(|| format!("writing {}", file.path.display()))?;
        if debug {
            eprintln!("[debug] wrote {}", file.path.display());
        }
    }

    Ok(())
}
