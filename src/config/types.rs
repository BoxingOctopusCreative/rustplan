use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct NetplanFile {
    pub network: NetworkConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct NetworkConfig {
    pub version: Option<u32>,
    pub renderer: Option<Renderer>,
    #[serde(default)]
    pub ethernets: IndexMap<String, EthernetDef>,
    #[serde(default)]
    pub wifis: IndexMap<String, WifiDef>,
    #[serde(default)]
    pub modems: IndexMap<String, CommonDef>,
    #[serde(default)]
    pub bridges: IndexMap<String, BridgeDef>,
    #[serde(default)]
    pub bonds: IndexMap<String, BondDef>,
    #[serde(default)]
    pub vlans: IndexMap<String, VlanDef>,
    #[serde(default)]
    pub tunnels: IndexMap<String, TunnelDef>,
    #[serde(default)]
    pub vrfs: IndexMap<String, VrfDef>,
    #[serde(default, rename = "dummy-devices")]
    pub dummy_devices: IndexMap<String, CommonDef>,
    #[serde(default, rename = "virtual-ethernets")]
    pub virtual_ethernets: IndexMap<String, VethDef>,
    #[serde(default, rename = "nm-devices")]
    pub nm_devices: IndexMap<String, CommonDef>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Renderer {
    Networkd,
    NetworkManager,
}

impl Default for Renderer {
    fn default() -> Self {
        Renderer::Networkd
    }
}

/// Fields common to all device types.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct CommonDef {
    pub renderer: Option<Renderer>,
    pub dhcp4: Option<bool>,
    pub dhcp6: Option<bool>,
    #[serde(default)]
    pub addresses: Vec<String>,
    pub nameservers: Option<Nameservers>,
    #[serde(default)]
    pub routes: Vec<Route>,
    #[serde(default, rename = "routing-policy")]
    pub routing_policy: Vec<RoutingPolicy>,
    pub mtu: Option<u32>,
    pub macaddress: Option<String>,
    #[serde(rename = "set-name")]
    pub set_name: Option<String>,
    #[serde(rename = "match")]
    pub match_: Option<Match>,
    pub optional: Option<bool>,
    pub critical: Option<bool>,
    #[serde(rename = "ignore-carrier")]
    pub ignore_carrier: Option<bool>,
    #[serde(rename = "link-local")]
    pub link_local: Option<Vec<String>>,
    #[serde(rename = "accept-ra")]
    pub accept_ra: Option<u8>,
    #[serde(rename = "dhcp4-overrides")]
    pub dhcp4_overrides: Option<DhcpOverrides>,
    #[serde(rename = "dhcp6-overrides")]
    pub dhcp6_overrides: Option<DhcpOverrides>,
    pub auth: Option<Auth>,
    pub wakeonlan: Option<bool>,
    #[serde(rename = "emit-lldp")]
    pub emit_lldp: Option<bool>,
    #[serde(rename = "receive-checksum-offload")]
    pub receive_checksum_offload: Option<bool>,
    #[serde(rename = "transmit-checksum-offload")]
    pub transmit_checksum_offload: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct Nameservers {
    #[serde(default)]
    pub addresses: Vec<String>,
    #[serde(default)]
    pub search: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct Route {
    pub to: Option<String>,
    pub via: Option<String>,
    pub metric: Option<u32>,
    pub table: Option<u32>,
    #[serde(rename = "on-link")]
    pub on_link: Option<bool>,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub scope: Option<String>,
    #[serde(rename = "from")]
    pub from: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct RoutingPolicy {
    pub from: Option<String>,
    pub to: Option<String>,
    pub table: Option<u32>,
    pub priority: Option<u32>,
    pub mark: Option<u32>,
    #[serde(rename = "type-of-service")]
    pub type_of_service: Option<u8>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct Match {
    pub name: Option<String>,
    pub macaddress: Option<String>,
    pub driver: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct DhcpOverrides {
    #[serde(rename = "use-dns")]
    pub use_dns: Option<bool>,
    #[serde(rename = "use-ntp")]
    pub use_ntp: Option<bool>,
    #[serde(rename = "use-hostname")]
    pub use_hostname: Option<bool>,
    #[serde(rename = "use-mtu")]
    pub use_mtu: Option<bool>,
    #[serde(rename = "use-routes")]
    pub use_routes: Option<bool>,
    pub hostname: Option<String>,
    #[serde(rename = "route-metric")]
    pub route_metric: Option<u32>,
    #[serde(rename = "send-hostname")]
    pub send_hostname: Option<bool>,
    #[serde(rename = "use-domains")]
    pub use_domains: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct Auth {
    #[serde(rename = "key-management")]
    pub key_management: Option<String>,
    pub password: Option<String>,
    pub identity: Option<String>,
    #[serde(rename = "anonymous-identity")]
    pub anonymous_identity: Option<String>,
    #[serde(rename = "ca-certificate")]
    pub ca_certificate: Option<String>,
    #[serde(rename = "client-certificate")]
    pub client_certificate: Option<String>,
    #[serde(rename = "client-key")]
    pub client_key: Option<String>,
    #[serde(rename = "client-key-password")]
    pub client_key_password: Option<String>,
    pub eap: Option<String>,
    pub phase2_auth: Option<String>,
}

// --- Device-specific structs ---

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct EthernetDef {
    #[serde(flatten)]
    pub common: CommonDef,
    pub link: Option<String>,
    #[serde(rename = "virtual-function-count")]
    pub virtual_function_count: Option<u32>,
    #[serde(rename = "embedded-switch-mode")]
    pub embedded_switch_mode: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct WifiDef {
    #[serde(flatten)]
    pub common: CommonDef,
    #[serde(default, rename = "access-points")]
    pub access_points: IndexMap<String, AccessPoint>,
    pub band: Option<String>,
    pub channel: Option<u32>,
    #[serde(rename = "bssid")]
    pub bssid: Option<String>,
    pub hidden: Option<bool>,
    pub mode: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct AccessPoint {
    pub password: Option<String>,
    pub auth: Option<Auth>,
    pub mode: Option<String>,
    pub band: Option<String>,
    pub channel: Option<u32>,
    pub bssid: Option<String>,
    pub hidden: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct BridgeDef {
    #[serde(flatten)]
    pub common: CommonDef,
    #[serde(default)]
    pub interfaces: Vec<String>,
    pub parameters: Option<BridgeParams>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct BridgeParams {
    #[serde(rename = "ageing-time")]
    pub ageing_time: Option<u32>,
    pub priority: Option<u32>,
    #[serde(rename = "forward-delay")]
    pub forward_delay: Option<u32>,
    #[serde(rename = "hello-time")]
    pub hello_time: Option<u32>,
    #[serde(rename = "max-age")]
    pub max_age: Option<u32>,
    pub stp: Option<bool>,
    #[serde(rename = "path-cost")]
    pub path_cost: Option<IndexMap<String, u32>>,
    #[serde(rename = "port-priority")]
    pub port_priority: Option<IndexMap<String, u32>>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct BondDef {
    #[serde(flatten)]
    pub common: CommonDef,
    #[serde(default)]
    pub interfaces: Vec<String>,
    pub parameters: Option<BondParams>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct BondParams {
    pub mode: Option<String>,
    #[serde(rename = "mii-monitor-interval")]
    pub mii_monitor_interval: Option<u32>,
    #[serde(rename = "up-delay")]
    pub up_delay: Option<u32>,
    #[serde(rename = "down-delay")]
    pub down_delay: Option<u32>,
    #[serde(rename = "learn-packet-interval")]
    pub learn_packet_interval: Option<u32>,
    #[serde(rename = "ad-select")]
    pub ad_select: Option<String>,
    #[serde(rename = "all-slaves-active")]
    pub all_slaves_active: Option<bool>,
    #[serde(rename = "arp-interval")]
    pub arp_interval: Option<u32>,
    #[serde(rename = "arp-ip-targets")]
    pub arp_ip_targets: Option<Vec<String>>,
    #[serde(rename = "arp-validate")]
    pub arp_validate: Option<String>,
    #[serde(rename = "arp-all-targets")]
    pub arp_all_targets: Option<String>,
    #[serde(rename = "fail-over-mac-policy")]
    pub fail_over_mac_policy: Option<String>,
    #[serde(rename = "gratuitous-arp")]
    pub gratuitous_arp: Option<u32>,
    #[serde(rename = "igmp-membership-interval")]
    pub igmp_membership_interval: Option<u32>,
    #[serde(rename = "lacp-rate")]
    pub lacp_rate: Option<String>,
    #[serde(rename = "packets-per-slave")]
    pub packets_per_slave: Option<u32>,
    pub primary: Option<String>,
    #[serde(rename = "primary-reselect-policy")]
    pub primary_reselect_policy: Option<String>,
    #[serde(rename = "resend-igmp")]
    pub resend_igmp: Option<u32>,
    #[serde(rename = "transmit-hash-policy")]
    pub transmit_hash_policy: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct VlanDef {
    #[serde(flatten)]
    pub common: CommonDef,
    pub id: Option<u16>,
    pub link: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct TunnelDef {
    #[serde(flatten)]
    pub common: CommonDef,
    pub mode: Option<String>,
    pub local: Option<String>,
    pub remote: Option<String>,
    pub key: Option<TunnelKey>,
    pub mark: Option<u32>,
    pub port: Option<u16>,
    pub ttl: Option<u8>,
    #[serde(default)]
    pub peers: Vec<WireguardPeer>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum TunnelKey {
    Single(String),
    Split {
        input: Option<u32>,
        output: Option<u32>,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct WireguardPeer {
    pub keys: Option<WireguardKeys>,
    #[serde(rename = "allowed-ips")]
    pub allowed_ips: Option<Vec<String>>,
    pub endpoint: Option<String>,
    #[serde(rename = "keepalive")]
    pub keepalive: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct WireguardKeys {
    pub public: Option<String>,
    pub shared: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct VrfDef {
    #[serde(flatten)]
    pub common: CommonDef,
    pub table: Option<u32>,
    #[serde(default)]
    pub interfaces: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct VethDef {
    #[serde(flatten)]
    pub common: CommonDef,
    pub peer: Option<String>,
}
