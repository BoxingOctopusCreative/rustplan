use std::path::PathBuf;

use anyhow::Result;

use crate::config::types::NetworkConfig;

pub mod networkd;
pub mod nm;

pub struct OutputFile {
    pub path: PathBuf,
    pub content: String,
}

pub trait Backend {
    fn generate(&self, cfg: &NetworkConfig, root: &std::path::Path) -> Result<Vec<OutputFile>>;
}
