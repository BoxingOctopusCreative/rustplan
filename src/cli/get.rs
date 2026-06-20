use std::path::Path;

use anyhow::{bail, Result};
use clap::Args;
use serde_yaml::Value;

#[derive(Debug, Args)]
pub struct GetArgs {
    /// Overrides the root directory for netplan files
    #[arg(long, value_name = "ROOT_DIR")]
    pub root_dir: Option<String>,
    /// Dotted key path to retrieve, or "all" (default)
    #[arg(default_value = "all")]
    pub key: String,
}

pub fn run(args: &GetArgs, _debug: bool) -> Result<()> {
    let root = args.root_dir.as_deref().unwrap_or("/");
    let merged = load_merged_value(Path::new(root))?;

    if args.key == "all" {
        print!("{}", serde_yaml::to_string(&merged)?);
        return Ok(());
    }

    let val = get_by_path(&merged, &args.key)?;
    // Print scalar values without YAML quoting; print structures as YAML
    match val {
        Value::String(s) => println!("{s}"),
        Value::Bool(b) => println!("{b}"),
        Value::Number(n) => println!("{n}"),
        Value::Null => println!("null"),
        other => print!("{}", serde_yaml::to_string(other)?),
    }
    Ok(())
}

fn load_merged_value(root: &Path) -> Result<Value> {
    let dirs = [
        root.join("usr/lib/netplan"),
        root.join("etc/netplan"),
        root.join("run/netplan"),
    ];

    let mut merged: Value = Value::Mapping(Default::default());
    for dir in &dirs {
        if !dir.exists() { continue; }
        let mut entries: Vec<_> = std::fs::read_dir(dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|x| x == "yaml" || x == "yml").unwrap_or(false))
            .collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let text = std::fs::read_to_string(entry.path())?;
            let doc: Value = serde_yaml::from_str(&text)?;
            deep_merge(&mut merged, doc);
        }
    }
    Ok(merged)
}

fn deep_merge(dst: &mut Value, src: Value) {
    match (dst, src) {
        (Value::Mapping(d), Value::Mapping(s)) => {
            for (k, v) in s { let e = d.entry(k).or_insert(Value::Null); deep_merge(e, v); }
        }
        (dst, src) => *dst = src,
    }
}

fn get_by_path<'a>(val: &'a Value, path: &str) -> Result<&'a Value> {
    let mut cur = val;
    for key in path.split('.') {
        match cur {
            Value::Mapping(m) => {
                cur = m.get(key)
                    .ok_or_else(|| anyhow::anyhow!("key '{key}' not found"))?;
            }
            _ => bail!("cannot descend into non-mapping at '{key}'"),
        }
    }
    Ok(cur)
}
