use std::path::Path;

use anyhow::{Context, Result};
use serde_yaml::Value;

use super::types::NetworkConfig;

/// Reads and deep-merges netplan YAML from the standard hierarchy under `root`.
/// Directories searched (in order, later wins): usr/lib/netplan, etc/netplan, run/netplan.
pub fn load(root: &Path) -> Result<NetworkConfig> {
    let dirs = [
        root.join("usr/lib/netplan"),
        root.join("etc/netplan"),
        root.join("run/netplan"),
    ];

    let mut merged: Value = Value::Mapping(Default::default());

    for dir in &dirs {
        if !dir.exists() {
            continue;
        }
        let mut entries: Vec<_> = std::fs::read_dir(dir)
            .with_context(|| format!("reading {}", dir.display()))?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .map(|x| x == "yaml" || x == "yml")
                    .unwrap_or(false)
            })
            .collect();
        entries.sort_by_key(|e| e.file_name());

        for entry in entries {
            let path = entry.path();
            let text = std::fs::read_to_string(&path)
                .with_context(|| format!("reading {}", path.display()))?;
            let doc: Value = serde_yaml::from_str(&text)
                .with_context(|| format!("parsing {}", path.display()))?;
            deep_merge(&mut merged, doc);
        }
    }

    // Extract just the `network` key for deserialization
    let network_val = merged
        .get("network")
        .cloned()
        .unwrap_or(Value::Mapping(Default::default()));

    let cfg: NetworkConfig = serde_yaml::from_value(network_val)
        .context("deserializing network config")?;

    Ok(cfg)
}

/// Recursively merge `src` into `dst`. Maps are merged key-by-key; scalars and
/// sequences from `src` overwrite those in `dst`.
fn deep_merge(dst: &mut Value, src: Value) {
    match (dst, src) {
        (Value::Mapping(d), Value::Mapping(s)) => {
            for (k, v) in s {
                let entry = d.entry(k).or_insert(Value::Null);
                deep_merge(entry, v);
            }
        }
        (dst, src) => *dst = src,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_yaml(dir: &Path, name: &str, content: &str) {
        std::fs::write(dir.join(name), content).unwrap();
    }

    fn setup(files: &[(&str, &str)]) -> TempDir {
        let tmp = TempDir::new().unwrap();
        let netplan_dir = tmp.path().join("etc/netplan");
        std::fs::create_dir_all(&netplan_dir).unwrap();
        for (name, content) in files {
            write_yaml(&netplan_dir, name, content);
        }
        tmp
    }

    #[test]
    fn test_basic_ethernet_dhcp() {
        let tmp = setup(&[(
            "01-eth.yaml",
            "network:\n  version: 2\n  ethernets:\n    eth0:\n      dhcp4: true\n",
        )]);
        let cfg = load(tmp.path()).unwrap();
        assert!(cfg.ethernets.contains_key("eth0"));
        assert_eq!(cfg.ethernets["eth0"].common.dhcp4, Some(true));
    }

    #[test]
    fn test_static_address() {
        let tmp = setup(&[(
            "01-static.yaml",
            "network:\n  version: 2\n  ethernets:\n    eth0:\n      addresses:\n        - 192.168.1.10/24\n",
        )]);
        let cfg = load(tmp.path()).unwrap();
        assert_eq!(cfg.ethernets["eth0"].common.addresses, vec!["192.168.1.10/24"]);
    }

    #[test]
    fn test_merge_later_file_wins() {
        let tmp = setup(&[
            (
                "01-base.yaml",
                "network:\n  version: 2\n  ethernets:\n    eth0:\n      dhcp4: true\n",
            ),
            (
                "02-override.yaml",
                "network:\n  version: 2\n  ethernets:\n    eth0:\n      dhcp4: false\n",
            ),
        ]);
        let cfg = load(tmp.path()).unwrap();
        assert_eq!(cfg.ethernets["eth0"].common.dhcp4, Some(false));
    }

    #[test]
    fn test_merge_adds_new_interfaces() {
        let tmp = setup(&[
            (
                "01-base.yaml",
                "network:\n  version: 2\n  ethernets:\n    eth0:\n      dhcp4: true\n",
            ),
            (
                "02-extra.yaml",
                "network:\n  version: 2\n  ethernets:\n    eth1:\n      dhcp4: true\n",
            ),
        ]);
        let cfg = load(tmp.path()).unwrap();
        assert!(cfg.ethernets.contains_key("eth0"));
        assert!(cfg.ethernets.contains_key("eth1"));
    }

    #[test]
    fn test_empty_dir_returns_default() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("etc/netplan")).unwrap();
        let cfg = load(tmp.path()).unwrap();
        assert!(cfg.ethernets.is_empty());
    }

    #[test]
    fn test_bridge_config() {
        let tmp = setup(&[(
            "01-br.yaml",
            "network:\n  version: 2\n  bridges:\n    br0:\n      interfaces: [eth0, eth1]\n      dhcp4: true\n",
        )]);
        let cfg = load(tmp.path()).unwrap();
        assert!(cfg.bridges.contains_key("br0"));
        assert_eq!(cfg.bridges["br0"].interfaces, vec!["eth0", "eth1"]);
    }

    #[test]
    fn test_vlan_config() {
        let tmp = setup(&[(
            "01-vlan.yaml",
            "network:\n  version: 2\n  vlans:\n    vlan10:\n      id: 10\n      link: eth0\n",
        )]);
        let cfg = load(tmp.path()).unwrap();
        assert_eq!(cfg.vlans["vlan10"].id, Some(10));
        assert_eq!(cfg.vlans["vlan10"].link, Some("eth0".to_string()));
    }
}
