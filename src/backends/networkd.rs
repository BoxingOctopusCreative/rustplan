use std::collections::HashSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::Result;
use indexmap::IndexMap;

use super::{Backend, OutputFile};
use crate::config::types::*;

pub struct Networkd;

#[derive(Default, Clone)]
struct Membership {
    bridge: Option<String>,
    bond: Option<String>,
    vrf: Option<String>,
    vlans: Vec<String>,
}

impl Backend for Networkd {
    fn generate(&self, cfg: &NetworkConfig, root: &Path) -> Result<Vec<OutputFile>> {
        let out_dir = root.join("run/systemd/network");

        // Pre-collect membership so each member gets the right [Network] entries
        let mut memberships: IndexMap<String, Membership> = IndexMap::new();
        for (id, dev) in &cfg.bridges {
            for member in &dev.interfaces {
                memberships.entry(member.clone()).or_default().bridge = Some(id.clone());
            }
        }
        for (id, dev) in &cfg.bonds {
            for member in &dev.interfaces {
                memberships.entry(member.clone()).or_default().bond = Some(id.clone());
            }
        }
        for (id, dev) in &cfg.vlans {
            if let Some(parent) = &dev.link {
                memberships.entry(parent.clone()).or_default().vlans.push(id.clone());
            }
        }
        for (id, dev) in &cfg.vrfs {
            for member in &dev.interfaces {
                memberships.entry(member.clone()).or_default().vrf = Some(id.clone());
            }
        }

        let no_membership = Membership::default();
        let membership_for = |id: &str| memberships.get(id).unwrap_or(&no_membership);

        let mut files = Vec::new();

        for (id, dev) in &cfg.ethernets {
            files.extend(ethernet_files(&out_dir, id, dev, membership_for(id)));
        }
        for (id, dev) in &cfg.wifis {
            files.extend(wifi_files(&out_dir, id, dev, membership_for(id)));
        }
        for (id, dev) in &cfg.bridges {
            files.extend(bridge_files(&out_dir, id, dev, membership_for(id)));
        }
        for (id, dev) in &cfg.bonds {
            files.extend(bond_files(&out_dir, id, dev, membership_for(id)));
        }
        for (id, dev) in &cfg.vlans {
            files.extend(vlan_files(&out_dir, id, dev, membership_for(id)));
        }
        for (id, dev) in &cfg.tunnels {
            files.extend(tunnel_files(&out_dir, id, dev, membership_for(id)));
        }
        for (id, dev) in &cfg.vrfs {
            files.extend(vrf_files(&out_dir, id, dev, membership_for(id)));
        }
        for (id, dev) in &cfg.dummy_devices {
            files.extend(dummy_files(&out_dir, id, dev, membership_for(id)));
        }

        // Stub .network files for members not explicitly declared as any device type
        let declared: HashSet<&str> = cfg.ethernets.keys()
            .chain(cfg.wifis.keys())
            .chain(cfg.bridges.keys())
            .chain(cfg.bonds.keys())
            .chain(cfg.vlans.keys())
            .chain(cfg.tunnels.keys())
            .chain(cfg.vrfs.keys())
            .chain(cfg.dummy_devices.keys())
            .map(|s| s.as_str())
            .collect();

        for (member_id, membership) in &memberships {
            if !declared.contains(member_id.as_str()) {
                files.push(member_stub_file(&out_dir, member_id, membership));
            }
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

fn write_network_section(out: &mut String, common: &CommonDef, membership: &Membership) {
    out.push_str("[Network]\n");
    if let Some(name) = &common.set_name {
        writeln!(out, "Name={name}").unwrap();
    }
    match (common.dhcp4, common.dhcp6) {
        (Some(true), Some(true)) => out.push_str("DHCP=yes\n"),
        (Some(true), _) => out.push_str("DHCP=ipv4\n"),
        (_, Some(true)) => out.push_str("DHCP=ipv6\n"),
        _ => {}
    }
    if let Some(ll) = &common.link_local {
        let val: Vec<&str> = ll.iter().map(|s| s.as_str()).collect();
        writeln!(out, "LinkLocalAddressing={}", val.join(" ")).unwrap();
    }
    if let Some(br) = &membership.bridge {
        writeln!(out, "Bridge={br}").unwrap();
    }
    if let Some(bond) = &membership.bond {
        writeln!(out, "Bond={bond}").unwrap();
    }
    if let Some(vrf) = &membership.vrf {
        writeln!(out, "VRF={vrf}").unwrap();
    }
    for vlan in &membership.vlans {
        writeln!(out, "VLAN={vlan}").unwrap();
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
                // Default route: Gateway only, no Destination
                if let Some(via) = &route.via {
                    writeln!(out, "Gateway={via}").unwrap();
                } else {
                    // Can't express a default route without a gateway; skip
                    out.truncate(out.len() - "[Route]\n".len());
                    continue;
                }
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

fn write_routing_policy_sections(out: &mut String, common: &CommonDef) {
    for rule in &common.routing_policy {
        out.push_str("[RoutingPolicyRule]\n");
        if let Some(from) = &rule.from {
            writeln!(out, "From={from}").unwrap();
        }
        if let Some(to) = &rule.to {
            writeln!(out, "To={to}").unwrap();
        }
        if let Some(t) = rule.table {
            writeln!(out, "Table={t}").unwrap();
        }
        if let Some(p) = rule.priority {
            writeln!(out, "Priority={p}").unwrap();
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

fn common_network_file(dir: &Path, id: &str, common: &CommonDef, membership: &Membership) -> OutputFile {
    let mut out = String::new();
    write_match_section(&mut out, id, common);
    write_network_section(&mut out, common, membership);
    write_address_sections(&mut out, common);
    write_route_sections(&mut out, common);
    write_routing_policy_sections(&mut out, common);
    write_dhcp_sections(&mut out, common);
    write_link_section(&mut out, common);
    OutputFile { path: net_path(dir, id), content: out }
}

/// Minimal .network for a member interface not explicitly declared as any device type.
fn member_stub_file(dir: &Path, id: &str, membership: &Membership) -> OutputFile {
    let mut out = String::new();
    writeln!(out, "[Match]\nName={id}\n").unwrap();
    out.push_str("[Network]\n");
    if let Some(br) = &membership.bridge {
        writeln!(out, "Bridge={br}").unwrap();
    }
    if let Some(bond) = &membership.bond {
        writeln!(out, "Bond={bond}").unwrap();
    }
    if let Some(vrf) = &membership.vrf {
        writeln!(out, "VRF={vrf}").unwrap();
    }
    out.push('\n');
    OutputFile { path: net_path(dir, id), content: out }
}

// --- per-device-type generators ---

fn ethernet_files(dir: &Path, id: &str, dev: &EthernetDef, m: &Membership) -> Vec<OutputFile> {
    vec![common_network_file(dir, id, &dev.common, m)]
}

fn wifi_files(dir: &Path, id: &str, dev: &WifiDef, m: &Membership) -> Vec<OutputFile> {
    vec![common_network_file(dir, id, &dev.common, m)]
}

fn bridge_files(dir: &Path, id: &str, dev: &BridgeDef, m: &Membership) -> Vec<OutputFile> {
    let mut files = Vec::new();

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
    files.push(common_network_file(dir, id, &dev.common, m));

    files
}

fn bond_files(dir: &Path, id: &str, dev: &BondDef, m: &Membership) -> Vec<OutputFile> {
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
    files.push(common_network_file(dir, id, &dev.common, m));

    files
}

fn vlan_files(dir: &Path, id: &str, dev: &VlanDef, m: &Membership) -> Vec<OutputFile> {
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
    files.push(common_network_file(dir, id, &dev.common, m));

    files
}

fn tunnel_files(dir: &Path, id: &str, dev: &TunnelDef, m: &Membership) -> Vec<OutputFile> {
    let mut files = Vec::new();

    let mut netdev = String::new();
    netdev.push_str("[NetDev]\n");
    writeln!(netdev, "Name={id}").unwrap();
    if let Some(mode) = &dev.mode {
        let kind = tunnel_mode_to_kind(mode);
        writeln!(netdev, "Kind={kind}").unwrap();
        netdev.push('\n');
        let section = tunnel_section_name(kind);
        writeln!(netdev, "[{section}]").unwrap();
        if let Some(local) = &dev.local {
            writeln!(netdev, "Local={local}").unwrap();
        }
        if let Some(remote) = &dev.remote {
            writeln!(netdev, "Remote={remote}").unwrap();
        }
        if let Some(ttl) = dev.ttl {
            writeln!(netdev, "TTL={ttl}").unwrap();
        }
        if kind == "wireguard" {
            for peer in &dev.peers {
                netdev.push('\n');
                netdev.push_str("[WireGuardPeer]\n");
                if let Some(keys) = &peer.keys {
                    if let Some(pub_key) = &keys.public {
                        writeln!(netdev, "PublicKey={pub_key}").unwrap();
                    }
                    if let Some(shared) = &keys.shared {
                        writeln!(netdev, "PresharedKey={shared}").unwrap();
                    }
                }
                if let Some(ips) = &peer.allowed_ips {
                    writeln!(netdev, "AllowedIPs={}", ips.join(",")).unwrap();
                }
                if let Some(ep) = &peer.endpoint {
                    writeln!(netdev, "Endpoint={ep}").unwrap();
                }
                if let Some(ka) = peer.keepalive {
                    writeln!(netdev, "PersistentKeepalive={ka}").unwrap();
                }
            }
        }
    }
    files.push(OutputFile { path: netdev_path(dir, id), content: netdev });
    files.push(common_network_file(dir, id, &dev.common, m));

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

fn tunnel_section_name(kind: &str) -> &str {
    match kind {
        "wireguard" => "WireGuard",
        "vxlan" => "VXLAN",
        "sit" | "gre" | "ip6gre" | "ipip" | "ipip6" | "ip6ip6" => "Tunnel",
        other => other,
    }
}

fn vrf_files(dir: &Path, id: &str, dev: &VrfDef, m: &Membership) -> Vec<OutputFile> {
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
    files.push(common_network_file(dir, id, &dev.common, m));

    files
}

fn dummy_files(dir: &Path, id: &str, dev: &CommonDef, m: &Membership) -> Vec<OutputFile> {
    let mut files = Vec::new();

    let mut netdev = String::new();
    netdev.push_str("[NetDev]\n");
    writeln!(netdev, "Name={id}").unwrap();
    netdev.push_str("Kind=dummy\n");
    files.push(OutputFile { path: netdev_path(dir, id), content: netdev });
    files.push(common_network_file(dir, id, dev, m));

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

    fn file_content<'a>(files: &'a [OutputFile], suffix: &str) -> Option<&'a str> {
        files.iter()
            .find(|f| f.path.to_string_lossy().ends_with(suffix))
            .map(|f| f.content.as_str())
    }

    #[test]
    fn test_ethernet_dhcp4_generates_network_file() {
        let (_tmp, cfg) = setup_config(
            "network:\n  version: 2\n  ethernets:\n    eth0:\n      dhcp4: true\n",
        );
        let out_tmp = TempDir::new().unwrap();
        let files = Networkd.generate(&cfg, out_tmp.path()).unwrap();
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
    fn test_bridge_generates_netdev_network_and_member_stub() {
        let (_tmp, cfg) = setup_config(
            "network:\n  version: 2\n  bridges:\n    br0:\n      interfaces: [eth0]\n      dhcp4: true\n",
        );
        let out_tmp = TempDir::new().unwrap();
        let files = Networkd.generate(&cfg, out_tmp.path()).unwrap();
        // br0.netdev, br0.network, eth0.network (stub)
        assert_eq!(files.len(), 3);
        let names: Vec<_> = files.iter()
            .map(|f| f.path.file_name().unwrap().to_str().unwrap())
            .collect();
        assert!(names.contains(&"10-netplan-br0.netdev"));
        assert!(names.contains(&"10-netplan-br0.network"));
        assert!(names.contains(&"10-netplan-eth0.network"));
        let eth0 = file_content(&files, "eth0.network").unwrap();
        assert!(eth0.contains("Bridge=br0"));
    }

    #[test]
    fn test_bridge_member_declared_in_ethernets_gets_bridge_entry() {
        let (_tmp, cfg) = setup_config(
            "network:\n  version: 2\n  ethernets:\n    eth0:\n      dhcp4: false\n  bridges:\n    br0:\n      interfaces: [eth0]\n      dhcp4: true\n",
        );
        let out_tmp = TempDir::new().unwrap();
        let files = Networkd.generate(&cfg, out_tmp.path()).unwrap();
        // eth0 is declared → no extra stub, but eth0.network must contain Bridge=br0
        let eth0 = file_content(&files, "eth0.network").unwrap();
        assert!(eth0.contains("Bridge=br0"), "eth0.network missing Bridge=br0:\n{eth0}");
    }

    #[test]
    fn test_bond_member_gets_bond_entry() {
        let (_tmp, cfg) = setup_config(
            "network:\n  version: 2\n  bonds:\n    bond0:\n      interfaces: [eth0, eth1]\n      parameters:\n        mode: active-backup\n",
        );
        let out_tmp = TempDir::new().unwrap();
        let files = Networkd.generate(&cfg, out_tmp.path()).unwrap();
        let eth0 = file_content(&files, "eth0.network").unwrap();
        let eth1 = file_content(&files, "eth1.network").unwrap();
        assert!(eth0.contains("Bond=bond0"));
        assert!(eth1.contains("Bond=bond0"));
    }

    #[test]
    fn test_vlan_parent_gets_vlan_ref() {
        let (_tmp, cfg) = setup_config(
            "network:\n  version: 2\n  ethernets:\n    eth0:\n      dhcp4: false\n  vlans:\n    vlan10:\n      id: 10\n      link: eth0\n",
        );
        let out_tmp = TempDir::new().unwrap();
        let files = Networkd.generate(&cfg, out_tmp.path()).unwrap();
        let eth0 = file_content(&files, "eth0.network").unwrap();
        assert!(eth0.contains("VLAN=vlan10"), "eth0.network missing VLAN=vlan10:\n{eth0}");
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

    #[test]
    fn test_default_route_uses_gateway() {
        let (_tmp, cfg) = setup_config(
            "network:\n  version: 2\n  ethernets:\n    eth0:\n      dhcp4: false\n      routes:\n        - to: default\n          via: 192.168.1.1\n",
        );
        let out_tmp = TempDir::new().unwrap();
        let files = Networkd.generate(&cfg, out_tmp.path()).unwrap();
        let content = &files[0].content;
        assert!(content.contains("Gateway=192.168.1.1"), "content:\n{content}");
        assert!(!content.contains("Gateway=_dhcp"), "content:\n{content}");
        assert!(!content.contains("Destination=default"), "content:\n{content}");
    }

    #[test]
    fn test_routing_policy_rule() {
        let (_tmp, cfg) = setup_config(
            "network:\n  version: 2\n  ethernets:\n    eth0:\n      routing-policy:\n        - from: 10.0.0.0/8\n          table: 100\n          priority: 10\n",
        );
        let out_tmp = TempDir::new().unwrap();
        let files = Networkd.generate(&cfg, out_tmp.path()).unwrap();
        let content = &files[0].content;
        assert!(content.contains("[RoutingPolicyRule]"), "content:\n{content}");
        assert!(content.contains("From=10.0.0.0/8"));
        assert!(content.contains("Table=100"));
        assert!(content.contains("Priority=10"));
    }

    #[test]
    fn test_dual_stack_dhcp() {
        let (_tmp, cfg) = setup_config(
            "network:\n  version: 2\n  ethernets:\n    eth0:\n      dhcp4: true\n      dhcp6: true\n",
        );
        let out_tmp = TempDir::new().unwrap();
        let files = Networkd.generate(&cfg, out_tmp.path()).unwrap();
        assert!(files[0].content.contains("DHCP=yes"));
    }
}
