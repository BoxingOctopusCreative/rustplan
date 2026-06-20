use std::path::Path;

use anyhow::{bail, Result};
use clap::Args;
use serde_yaml::Value;

#[derive(Debug, Args)]
pub struct SetArgs {
    /// Overrides the root directory for netplan files
    #[arg(long, value_name = "ROOT_DIR")]
    pub root_dir: Option<String>,
    /// Hint for origin file (written to etc/netplan/70-netplan-set.yaml by default)
    #[arg(long, value_name = "ORIGIN_HINT")]
    pub origin_hint: Option<String>,
    /// Key=value assignment, e.g. network.ethernets.eth0.dhcp4=true
    pub assignment: String,
}

pub fn run(args: &SetArgs, _debug: bool) -> Result<()> {
    let root = args.root_dir.as_deref().unwrap_or("/");
    let root_path = Path::new(root);

    let (key_path, raw_value) = args.assignment.split_once('=')
        .ok_or_else(|| anyhow::anyhow!("assignment must be key=value"))?;

    // Determine the target file
    let target_file = match &args.origin_hint {
        Some(hint) => root_path.join("etc/netplan").join(hint),
        None => root_path.join("etc/netplan/70-netplan-set.yaml"),
    };

    // Load existing target file or start empty
    let mut doc: Value = if target_file.exists() {
        let text = std::fs::read_to_string(&target_file)?;
        serde_yaml::from_str(&text).unwrap_or(Value::Mapping(Default::default()))
    } else {
        Value::Mapping(Default::default())
    };

    let parsed_value: Value = if raw_value.is_empty() || raw_value == "null" {
        Value::Null
    } else {
        serde_yaml::from_str(raw_value)
            .unwrap_or(Value::String(raw_value.to_string()))
    };

    set_by_path(&mut doc, key_path, parsed_value)?;

    if let Some(parent) = target_file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&target_file, serde_yaml::to_string(&doc)?)?;

    Ok(())
}

fn set_by_path(root: &mut Value, path: &str, value: Value) -> Result<()> {
    let keys: Vec<&str> = path.split('.').collect();
    let mut cur = root;
    for (i, key) in keys.iter().enumerate() {
        if i == keys.len() - 1 {
            // Leaf: set or remove the value
            match cur {
                Value::Mapping(m) => {
                    if matches!(value, Value::Null) {
                        m.remove(key);
                    } else {
                        m.insert(Value::String(key.to_string()), value);
                    }
                    return Ok(());
                }
                _ => bail!("cannot set key '{key}' on non-mapping"),
            }
        } else {
            match cur {
                Value::Mapping(m) => {
                    let k = Value::String(key.to_string());
                    if !m.contains_key(&k) {
                        m.insert(k.clone(), Value::Mapping(Default::default()));
                    }
                    cur = m.get_mut(&k).unwrap();
                }
                _ => bail!("cannot descend into non-mapping at '{key}'"),
            }
        }
    }
    Ok(())
}
