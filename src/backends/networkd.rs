use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::Result;

use super::{Backend, OutputFile};
use crate::config::types::*;

pub struct Networkd;

impl Backend for Networkd {
    fn generate(&self, cfg: &NetworkConfig, root: &Path) -> Result<Vec<OutputFile>> {
        let out_dir = root.join("run/systemd/network");
        let mut files = Vec::new();

        for (id, dev) in &cfg.ethernets {
            files.extend(ethernet_files(&out_dir, id, dev));
        }
        for (id, dev) in &cfg.wifis {
            files.extend(wifi_files(&out_dir, id, dev));
        }
        for (id, dev) in &cfg.bridges {
            files.extend(bridge_files(&out_dir, id, dev));
        }
        for (id, dev) in &cfg.bonds {
            files.extend(bond_files(&out_dir, id, dev));
        }
        for (id, dev) in &cfg.vlans {
            files.extend(vlan_files(&out_dir, id, dev));
        }
        for (id, dev) in &cfg.tunnels {
            files.extend(tunnel_files(&out_dir, id, dev));
        }
        for (id, dev) in &cfg.vrfs {
            files.extend(vrf_files(&out_dir, id, dev));
        }
        for (id, dev) in &cfg.dummy_devices {
            files.extend(dummy_files(&out_dir, id, dev));
        }

        Ok(files)
    }
}

fn net_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("10-netplan-{id}.network"))
}

fn netdev_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("10-netplan-{id}.netdev"))
}

// --- shared helpers ---

fn write_match_section(out: &mut String, id: &str, common: &CommonDef) {
    out.push_str("[Match]\n");
    if let Some(m) = &common.match_ {
        if let Some(name) = &m.name {
            writeln!(out, "Name={name}").unwrap();
        }
        if let Some(mac) = &m.macaddress {
            writeln!(out, "MACAddress={mac}").unwrap();
        }
        if let Some(drivers) = &m.driver {
            writeln!(out, "Driver={}", drivers.join(" ")).unwrap();
        }
    } else {
        writeln!(out, "Name={id}").unwrap();
    }
    out.push('\n');
}

fn write_network_section(out: &mut String, common: &CommonDef, extra_member_of: Option<&str>) {
    out.push_str("[Network]\n");
    if let Some(name) = &common.set_name {
        writeln!(out, "Name={name}").unwrap();
    }
    if let Some(true) = common.dhcp4 {
        out.push_str("DHCP=ipv4\n");
    }
    if let Some(true) = common.dhcp6 {
        // If dhcp4 already set DHCP, upgrade to yes; otherwise set ipv6
        if out.contains("DHCP=ipv4") {
            let replaced = out.replace("DHCP=ipv4\n", "DHCP=yes\n");
            *out = replaced;
        } else {
            out.push_str("DHCP=ipv6\n");
        }
    }
    // link-local
    if let Some(ll) = &common.link_local {
        let val: Vec<&str> = ll.iter().map(|s| s.as_str()).collect();
        writeln!(out, "LinkLocalAddressing={}", val.join(" ")).unwrap();
    }
    if let Some(bridge) = extra_member_of {
        writeln!(out, "Bridge={bridge}").unwrap();
    }
    if let Some(ns) = &common.nameservers {
        if !ns.addresses.is_empty() {
            writeln!(out, "DNS={}", ns.addresses.join(" ")).unwrap();
        }
        if !ns.search.is_empty() {
            writeln!(out, "Domains={}", ns.search.join(" ")).unwrap();
        }
    }
    out.push('\n');
}

fn write_address_sections(out: &mut String, common: &CommonDef) {
    for addr in &common.addresses {
        out.push_str("[Address]\n");
        writeln!(out, "Address={addr}").unwrap();
        out.push('\n');
    }
}

fn write_route_sections(out: &mut String, common: &CommonDef) {
    for route in &common.routes {
        out.push_str("[Route]\n");
        if let Some(to) = &route.to {
            if to == "default" {
                out.push_str("Gateway=_dhcp\n");
            } else {
                writeln!(out, "Destination={to}").unwrap();
                if let Some(via) = &route.via {
                    writeln!(out, "Gateway={via}").unwrap();
                }
            }
        }
        if let Some(metric) = route.metric {
            writeln!(out, "Metric={metric}").unwrap();
        }
        if let Some(table) = route.table {
            writeln!(out, "Table={table}").unwrap();
        }
        out.push('\n');
    }
}

fn write_dhcp_sections(out: &mut String, common: &CommonDef) {
    if let Some(true) = common.dhcp4 {
        out.push_str("[DHCPv4]\n");
        if let Some(ov) = &common.dhcp4_overrides {
            if let Some(v) = ov.use_dns {
                writeln!(out, "UseDNS={}", bool_yn(v)).unwrap();
            }
            if let Some(v) = ov.use_ntp {
                writeln!(out, "UseNTP={}", bool_yn(v)).unwrap();
            }
            if let Some(v) = ov.use_hostname {
                writeln!(out, "UseHostname={}", bool_yn(v)).unwrap();
            }
            if let Some(v) = ov.use_mtu {
                writeln!(out, "UseMTU={}", bool_yn(v)).unwrap();
            }
            if let Some(v) = ov.use_routes {
                writeln!(out, "UseRoutes={}", bool_yn(v)).unwrap();
            }
            if let Some(h) = &ov.hostname {
                writeln!(out, "Hostname={h}").unwrap();
            }
            if let Some(m) = ov.route_metric {
                writeln!(out, "RouteMetric={m}").unwrap();
            }
        }
        out.push('\n');
    }
}

fn write_link_section(out: &mut String, common: &CommonDef) {
    let has_mtu = common.mtu.is_some();
    let has_mac = common.macaddress.is_some();
    if has_mtu || has_mac {
        out.push_str("[Link]\n");
        if let Some(mac) = &common.macaddress {
            writeln!(out, "MACAddress={mac}").unwrap();
        }
        if let Some(mtu) = common.mtu {
            writeln!(out, "MTUBytes={mtu}").unwrap();
        }
        out.push('\n');
    }
}

fn bool_yn(v: bool) -> &'static str {
    if v { "yes" } else { "no" }
}

fn common_network_file(dir: &Path, id: &str, common: &CommonDef) -> OutputFile {
    let mut out = String::new();
    write_match_section(&mut out, id, common);
    write_network_section(&mut out, common, None);
    write_address_sections(&mut out, common);
    write_route_sections(&mut out, common);
    write_dhcp_sections(&mut out, common);
    write_link_section(&mut out, common);
    OutputFile { path: net_path(dir, id), content: out }
}

// --- per-device-type generators ---

fn ethernet_files(dir: &Path, id: &str, dev: &EthernetDef) -> Vec<OutputFile> {
    vec![common_network_file(dir, id, &dev.common)]
}

fn wifi_files(dir: &Path, id: &str, dev: &WifiDef) -> Vec<OutputFile> {
    vec![common_network_file(dir, id, &dev.common)]
}

fn bridge_files(dir: &Path, id: &str, dev: &BridgeDef) -> Vec<OutputFile> {
    let mut files = Vec::new();

    // .netdev for the bridge device itself
    let mut netdev = String::new();
    netdev.push_str("[NetDev]\n");
    writeln!(netdev, "Name={id}").unwrap();
    netdev.push_str("Kind=bridge\n");
    if let Some(params) = &dev.parameters {
        netdev.push('\n');
        netdev.push_str("[Bridge]\n");
        if let Some(v) = params.stp {
            writeln!(netdev, "STP={}", bool_yn(v)).unwrap();
        }
        if let Some(v) = params.forward_delay {
            writeln!(netdev, "ForwardDelaySec={v}").unwrap();
        }
        if let Some(v) = params.hello_time {
            writeln!(netdev, "HelloTimeSec={v}").unwrap();
        }
        if let Some(v) = params.max_age {
            writeln!(netdev, "MaxAgeSec={v}").unwrap();
        }
        if let Some(v) = params.ageing_time {
            writeln!(netdev, "AgeingTimeSec={v}").unwrap();
        }
    }
    files.push(OutputFile { path: netdev_path(dir, id), content: netdev });

    // .network for the bridge interface itself
    files.push(common_network_file(dir, id, &dev.common));

    files
}

fn bond_files(dir: &Path, id: &str, dev: &BondDef) -> Vec<OutputFile> {
    let mut files = Vec::new();

    let mut netdev = String::new();
    netdev.push_str("[NetDev]\n");
    writeln!(netdev, "Name={id}").unwrap();
    netdev.push_str("Kind=bond\n");
    if let Some(params) = &dev.parameters {
        netdev.push('\n');
        netdev.push_str("[Bond]\n");
        if let Some(mode) = &params.mode {
            writeln!(netdev, "Mode={mode}").unwrap();
        }
        if let Some(v) = params.mii_monitor_interval {
            writeln!(netdev, "MIIMonitorSec={}", v as f64 / 1000.0).unwrap();
        }
        if let Some(v) = params.up_delay {
            writeln!(netdev, "UpDelaySec={}", v as f64 / 1000.0).unwrap();
        }
        if let Some(v) = params.down_delay {
            writeln!(netdev, "DownDelaySec={}", v as f64 / 1000.0).unwrap();
        }
        if let Some(v) = &params.lacp_rate {
            writeln!(netdev, "LACPTransmitRate={v}").unwrap();
        }
        if let Some(v) = &params.transmit_hash_policy {
            writeln!(netdev, "TransmitHashPolicy={v}").unwrap();
        }
        if let Some(v) = &params.ad_select {
            writeln!(netdev, "AdSelect={v}").unwrap();
        }
    }
    files.push(OutputFile { path: netdev_path(dir, id), content: netdev });
    files.push(common_network_file(dir, id, &dev.common));

    files
}

fn vlan_files(dir: &Path, id: &str, dev: &VlanDef) -> Vec<OutputFile> {
    let mut files = Vec::new();

    let mut netdev = String::new();
    netdev.push_str("[NetDev]\n");
    writeln!(netdev, "Name={id}").unwrap();
    netdev.push_str("Kind=vlan\n");
    netdev.push('\n');
    netdev.push_str("[VLAN]\n");
    if let Some(vid) = dev.id {
        writeln!(netdev, "Id={vid}").unwrap();
    }
    files.push(OutputFile { path: netdev_path(dir, id), content: netdev });

    // The parent link needs a [Network] section that references this VLAN
    // We generate the VLAN's own .network file for its IP config
    files.push(common_network_file(dir, id, &dev.common));

    files
}

fn tunnel_files(dir: &Path, id: &str, dev: &TunnelDef) -> Vec<OutputFile> {
    let mut files = Vec::new();

    let mut netdev = String::new();
    netdev.push_str("[NetDev]\n");
    writeln!(netdev, "Name={id}").unwrap();
    if let Some(mode) = &dev.mode {
        let kind = tunnel_mode_to_kind(mode);
        writeln!(netdev, "Kind={kind}").unwrap();
        netdev.push('\n');
        writeln!(netdev, "[{kind}]").unwrap(); // section name matches kind for most tunnels
        if let Some(local) = &dev.local {
            writeln!(netdev, "Local={local}").unwrap();
        }
        if let Some(remote) = &dev.remote {
            writeln!(netdev, "Remote={remote}").unwrap();
        }
        if let Some(ttl) = dev.ttl {
            writeln!(netdev, "TTL={ttl}").unwrap();
        }
    }
    files.push(OutputFile { path: netdev_path(dir, id), content: netdev });
    files.push(common_network_file(dir, id, &dev.common));

    files
}

fn tunnel_mode_to_kind(mode: &str) -> &str {
    match mode {
        "sit" => "sit",
        "gre" => "gre",
        "ip6gre" => "ip6gre",
        "ipip" => "ipip",
        "ipip6" => "ipip6",
        "ip6ip6" => "ip6ip6",
        "vxlan" => "vxlan",
        "wireguard" => "wireguard",
        other => other,
    }
}

fn vrf_files(dir: &Path, id: &str, dev: &VrfDef) -> Vec<OutputFile> {
    let mut files = Vec::new();

    let mut netdev = String::new();
    netdev.push_str("[NetDev]\n");
    writeln!(netdev, "Name={id}").unwrap();
    netdev.push_str("Kind=vrf\n");
    netdev.push('\n');
    netdev.push_str("[VRF]\n");
    if let Some(table) = dev.table {
        writeln!(netdev, "Table={table}").unwrap();
    }
    files.push(OutputFile { path: netdev_path(dir, id), content: netdev });
    files.push(common_network_file(dir, id, &dev.common));

    files
}

fn dummy_files(dir: &Path, id: &str, dev: &CommonDef) -> Vec<OutputFile> {
    let mut files = Vec::new();

    let mut netdev = String::new();
    netdev.push_str("[NetDev]\n");
    writeln!(netdev, "Name={id}").unwrap();
    netdev.push_str("Kind=dummy\n");
    files.push(OutputFile { path: netdev_path(dir, id), content: netdev });
    files.push(common_network_file(dir, id, dev));

    files
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::loader;
    use tempfile::TempDir;

    fn setup_config(yaml: &str) -> (TempDir, NetworkConfig) {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("etc/netplan");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("01-test.yaml"), yaml).unwrap();
        let cfg = loader::load(tmp.path()).unwrap();
        (tmp, cfg)
    }

    #[test]
    fn test_ethernet_dhcp4_generates_network_file() {
        let (_tmp, cfg) = setup_config(
            "network:\n  version: 2\n  ethernets:\n    eth0:\n      dhcp4: true\n",
        );
        let out_tmp = TempDir::new().unwrap();
        let backend = Networkd;
        let files = backend.generate(&cfg, out_tmp.path()).unwrap();
        assert_eq!(files.len(), 1);
        let f = &files[0];
        assert!(f.path.ends_with("10-netplan-eth0.network"));
        assert!(f.content.contains("[Match]\nName=eth0"));
        assert!(f.content.contains("DHCP=ipv4"));
    }

    #[test]
    fn test_static_address_generates_address_section() {
        let (_tmp, cfg) = setup_config(
            "network:\n  version: 2\n  ethernets:\n    eth0:\n      addresses:\n        - 10.0.0.5/24\n",
        );
        let out_tmp = TempDir::new().unwrap();
        let files = Networkd.generate(&cfg, out_tmp.path()).unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].content.contains("[Address]\nAddress=10.0.0.5/24"));
    }

    #[test]
    fn test_bridge_generates_netdev_and_network() {
        let (_tmp, cfg) = setup_config(
            "network:\n  version: 2\n  bridges:\n    br0:\n      interfaces: [eth0]\n      dhcp4: true\n",
        );
        let out_tmp = TempDir::new().unwrap();
        let files = Networkd.generate(&cfg, out_tmp.path()).unwrap();
        assert_eq!(files.len(), 2);
        let names: Vec<_> = files.iter().map(|f| f.path.file_name().unwrap().to_str().unwrap()).collect();
        assert!(names.contains(&"10-netplan-br0.netdev"));
        assert!(names.contains(&"10-netplan-br0.network"));
        let netdev = files.iter().find(|f| f.path.extension().unwrap() == "netdev").unwrap();
        assert!(netdev.content.contains("Kind=bridge"));
    }

    #[test]
    fn test_vlan_generates_netdev_with_id() {
        let (_tmp, cfg) = setup_config(
            "network:\n  version: 2\n  vlans:\n    vlan10:\n      id: 10\n      link: eth0\n",
        );
        let out_tmp = TempDir::new().unwrap();
        let files = Networkd.generate(&cfg, out_tmp.path()).unwrap();
        let netdev = files.iter().find(|f| f.path.extension().unwrap() == "netdev").unwrap();
        assert!(netdev.content.contains("Kind=vlan"));
        assert!(netdev.content.contains("Id=10"));
    }

    #[test]
    fn test_nameservers_in_network_section() {
        let (_tmp, cfg) = setup_config(
            "network:\n  version: 2\n  ethernets:\n    eth0:\n      dhcp4: true\n      nameservers:\n        addresses: [8.8.8.8, 8.8.4.4]\n        search: [example.com]\n",
        );
        let out_tmp = TempDir::new().unwrap();
        let files = Networkd.generate(&cfg, out_tmp.path()).unwrap();
        let content = &files[0].content;
        assert!(content.contains("DNS=8.8.8.8 8.8.4.4"));
        assert!(content.contains("Domains=example.com"));
    }
}
