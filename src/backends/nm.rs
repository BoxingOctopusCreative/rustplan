use std::path::{Path, PathBuf};
use std::fmt::Write as _;

use anyhow::Result;

use super::{Backend, OutputFile};
use crate::config::types::*;

pub struct NetworkManager;

impl Backend for NetworkManager {
    fn generate(&self, cfg: &NetworkConfig, root: &Path) -> Result<Vec<OutputFile>> {
        let out_dir = root.join("run/NetworkManager/system-connections");
        let mut files = Vec::new();

        for (id, dev) in &cfg.ethernets {
            files.push(ethernet_file(&out_dir, id, dev));
        }
        for (id, dev) in &cfg.wifis {
            files.extend(wifi_files(&out_dir, id, dev));
        }
        for (id, dev) in &cfg.bridges {
            files.push(bridge_file(&out_dir, id, dev));
        }
        for (id, dev) in &cfg.bonds {
            files.push(bond_file(&out_dir, id, dev));
        }
        for (id, dev) in &cfg.vlans {
            files.push(vlan_file(&out_dir, id, dev));
        }

        Ok(files)
    }
}

fn conn_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("netplan-{id}.nmconnection"))
}

fn write_ipv4_section(out: &mut String, common: &CommonDef) {
    out.push_str("[ipv4]\n");
    if common.dhcp4 == Some(true) {
        out.push_str("method=auto\n");
    } else if !common.addresses.is_empty() {
        out.push_str("method=manual\n");
        for (i, addr) in common.addresses.iter().enumerate() {
            writeln!(out, "address{i}={addr}").unwrap();
        }
    } else {
        out.push_str("method=disabled\n");
    }
    if let Some(ns) = &common.nameservers {
        if !ns.addresses.is_empty() {
            let v4_addrs: Vec<&str> = ns.addresses.iter()
                .filter(|a| !a.contains(':'))
                .map(|a| a.as_str())
                .collect();
            if !v4_addrs.is_empty() {
                writeln!(out, "dns={}", v4_addrs.join(";")).unwrap();
            }
        }
        if !ns.search.is_empty() {
            writeln!(out, "dns-search={}", ns.search.join(";")).unwrap();
        }
    }
    for route in &common.routes {
        if let (Some(to), Some(via)) = (&route.to, &route.via) {
            if !to.contains(':') {
                writeln!(out, "route1={to},{via}").unwrap();
            }
        }
    }
    out.push('\n');
}

fn write_ipv6_section(out: &mut String, common: &CommonDef) {
    out.push_str("[ipv6]\n");
    if common.dhcp6 == Some(true) {
        out.push_str("method=auto\n");
    } else {
        out.push_str("method=ignore\n");
    }
    out.push('\n');
}

fn ethernet_file(dir: &Path, id: &str, dev: &EthernetDef) -> OutputFile {
    let mut out = String::new();
    out.push_str("[connection]\n");
    writeln!(out, "id=netplan-{id}").unwrap();
    out.push_str("type=ethernet\n");
    writeln!(out, "interface-name={id}").unwrap();
    out.push('\n');
    out.push_str("[ethernet]\n\n");
    write_ipv4_section(&mut out, &dev.common);
    write_ipv6_section(&mut out, &dev.common);
    OutputFile { path: conn_path(dir, id), content: out }
}

fn wifi_files(dir: &Path, id: &str, dev: &WifiDef) -> Vec<OutputFile> {
    let mut files = Vec::new();
    for (ssid, ap) in &dev.access_points {
        let safe_id = format!("{id}-{}", ssid.replace(' ', "_"));
        let mut out = String::new();
        out.push_str("[connection]\n");
        writeln!(out, "id=netplan-{safe_id}").unwrap();
        out.push_str("type=wifi\n");
        writeln!(out, "interface-name={id}").unwrap();
        out.push('\n');
        out.push_str("[wifi]\n");
        writeln!(out, "ssid={ssid}").unwrap();
        out.push_str("mode=infrastructure\n");
        out.push('\n');
        if let Some(pw) = &ap.password {
            out.push_str("[wifi-security]\n");
            out.push_str("key-mgmt=wpa-psk\n");
            writeln!(out, "psk={pw}").unwrap();
            out.push('\n');
        }
        write_ipv4_section(&mut out, &dev.common);
        write_ipv6_section(&mut out, &dev.common);
        let path = dir.join(format!("netplan-{safe_id}.nmconnection"));
        files.push(OutputFile { path, content: out });
    }
    files
}

fn bridge_file(dir: &Path, id: &str, dev: &BridgeDef) -> OutputFile {
    let mut out = String::new();
    out.push_str("[connection]\n");
    writeln!(out, "id=netplan-{id}").unwrap();
    out.push_str("type=bridge\n");
    writeln!(out, "interface-name={id}").unwrap();
    out.push('\n');
    out.push_str("[bridge]\n\n");
    write_ipv4_section(&mut out, &dev.common);
    write_ipv6_section(&mut out, &dev.common);
    OutputFile { path: conn_path(dir, id), content: out }
}

fn bond_file(dir: &Path, id: &str, dev: &BondDef) -> OutputFile {
    let mut out = String::new();
    out.push_str("[connection]\n");
    writeln!(out, "id=netplan-{id}").unwrap();
    out.push_str("type=bond\n");
    writeln!(out, "interface-name={id}").unwrap();
    out.push('\n');
    out.push_str("[bond]\n");
    if let Some(params) = &dev.parameters {
        if let Some(mode) = &params.mode {
            writeln!(out, "mode={mode}").unwrap();
        }
        if let Some(v) = params.mii_monitor_interval {
            writeln!(out, "miimon={v}").unwrap();
        }
    }
    out.push('\n');
    write_ipv4_section(&mut out, &dev.common);
    write_ipv6_section(&mut out, &dev.common);
    OutputFile { path: conn_path(dir, id), content: out }
}

fn vlan_file(dir: &Path, id: &str, dev: &VlanDef) -> OutputFile {
    let mut out = String::new();
    out.push_str("[connection]\n");
    writeln!(out, "id=netplan-{id}").unwrap();
    out.push_str("type=vlan\n");
    writeln!(out, "interface-name={id}").unwrap();
    out.push('\n');
    out.push_str("[vlan]\n");
    if let Some(vid) = dev.id {
        writeln!(out, "id={vid}").unwrap();
    }
    if let Some(parent) = &dev.link {
        writeln!(out, "parent={parent}").unwrap();
    }
    out.push('\n');
    write_ipv4_section(&mut out, &dev.common);
    write_ipv6_section(&mut out, &dev.common);
    OutputFile { path: conn_path(dir, id), content: out }
}
