//! Parser for `ifconfig` command output.
//!
//! Supports Linux (old-style with "Link encap:"), OpenBSD/modern Linux
//! (flags= style), and FreeBSD/macOS (flags= with metric) output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use regex::Regex;
use serde_json::{Map, Value};

pub struct IfconfigParser;

static INFO: ParserInfo = ParserInfo {
    name: "ifconfig",
    argument: "--ifconfig",
    version: "2.4.0",
    description: "Converts `ifconfig` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux, Platform::Darwin, Platform::FreeBSD],
    tags: &[Tag::Command, Tag::Slurpable],
    magic_commands: &["ifconfig"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static IFCONFIG_PARSER: IfconfigParser = IfconfigParser;

inventory::submit! { ParserEntry::new(&IFCONFIG_PARSER) }

fn null_iface() -> Map<String, Value> {
    let mut m = Map::new();
    let null_fields = [
        "name",
        "flags",
        "state",
        "mtu",
        "type",
        "mac_addr",
        "ipv4_addr",
        "ipv4_mask",
        "ipv4_bcast",
        "ipv6_addr",
        "ipv6_mask",
        "ipv6_scope",
        "ipv6_type",
        "metric",
        "rx_packets",
        "rx_errors",
        "rx_dropped",
        "rx_overruns",
        "rx_frame",
        "tx_packets",
        "tx_errors",
        "tx_dropped",
        "tx_overruns",
        "tx_carrier",
        "tx_collisions",
        "rx_bytes",
        "tx_bytes",
    ];
    for f in &null_fields {
        m.insert(f.to_string(), Value::Null);
    }
    m
}

fn convert_hex_mask(mask: &str) -> String {
    // 0xffffff00 -> 255.255.255.0
    if mask.starts_with("0x") || mask.starts_with("0X") {
        let hex = &mask[2..];
        if hex.len() == 8 {
            let bytes: Vec<u8> = (0..4)
                .filter_map(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok())
                .collect();
            if bytes.len() == 4 {
                return format!("{}.{}.{}.{}", bytes[0], bytes[1], bytes[2], bytes[3]);
            }
        }
    }
    // CIDR notation -> dotted quad
    if let Ok(cidr) = mask.parse::<u8>()
        && cidr <= 32
    {
        let n: u32 = if cidr == 0 { 0 } else { !0u32 << (32 - cidr) };
        return format!(
            "{}.{}.{}.{}",
            (n >> 24) & 0xff,
            (n >> 16) & 0xff,
            (n >> 8) & 0xff,
            n & 0xff
        );
    }
    mask.to_string()
}

impl Parser for IfconfigParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        // Compile regexes
        // Old Linux format (ubuntu-16.04 style): "ens33     Link encap:Ethernet  HWaddr 00:..."
        // Fix: type stops before HWaddr using lookahead simulation by restricting to one word
        let re_linux_iface = Regex::new(
            r"^(?P<name>[a-zA-Z0-9:._-]+)\s+Link\s+encap:(?P<type>\S+\s?\S+)(?:\s+HWaddr\s+(?P<mac_addr>[0-9A-Fa-f:?]+))?",
        )
        .map_err(|e| ParseError::Regex(e.to_string()))?;

        let re_linux_ipv4 = Regex::new(
            r"inet\saddr:(?P<address>(?:[0-9]{1,3}\.){3}[0-9]{1,3})(?:\s+Bcast:(?P<broadcast>(?:[0-9]{1,3}\.){3}[0-9]{1,3}))?\s+Mask:(?P<mask>(?:[0-9]{1,3}\.){3}[0-9]{1,3})",
        )
        .map_err(|e| ParseError::Regex(e.to_string()))?;

        // Old Linux IPv6: "inet6 addr: fe80::c1ca:3dee:39f7:5937/64 Scope:Link"
        let re_linux_ipv6 = Regex::new(
            r"inet6\s+addr:\s+(?P<address>[^\s/]+)/(?P<mask>[0-9]+)\s+Scope:(?P<scope>\w+)",
        )
        .map_err(|e| ParseError::Regex(e.to_string()))?;

        let re_linux_state = Regex::new(
            r"\s+(?P<state>(?:\w+\s)+?)(?:\s+)?MTU:(?P<mtu>[0-9]+)\s+Metric:(?P<metric>[0-9]+)",
        )
        .map_err(|e| ParseError::Regex(e.to_string()))?;

        let re_linux_rx = Regex::new(
            r"RX\spackets:(?P<rx_packets>[0-9]+)\s+errors:(?P<rx_errors>[0-9]+)\s+dropped:(?P<rx_dropped>[0-9]+)\s+overruns:(?P<rx_overruns>[0-9]+)\s+frame:(?P<rx_frame>[0-9]+)",
        )
        .map_err(|e| ParseError::Regex(e.to_string()))?;

        let re_linux_tx = Regex::new(
            r"TX\spackets:(?P<tx_packets>[0-9]+)\s+errors:(?P<tx_errors>[0-9]+)\s+dropped:(?P<tx_dropped>[0-9]+)\s+overruns:(?P<tx_overruns>[0-9]+)\s+carrier:(?P<tx_carrier>[0-9]+)",
        )
        .map_err(|e| ParseError::Regex(e.to_string()))?;

        let re_linux_bytes =
            Regex::new(r"RX\sbytes:(?P<rx_bytes>\d+)\s+\([^)]*\)\s+TX\sbytes:(?P<tx_bytes>\d+)")
                .map_err(|e| ParseError::Regex(e.to_string()))?;

        let re_linux_tx_stats =
            Regex::new(r"collisions:(?P<tx_collisions>[0-9]+)\s+txqueuelen:[0-9]+")
                .map_err(|e| ParseError::Regex(e.to_string()))?;

        // OpenBSD/modern Linux flags= style (without metric)
        let re_openbsd_iface = Regex::new(
            r"(?x)^(?P<name>[a-zA-Z0-9:._-]+):\s+
            flags=(?P<flags>[0-9]+)
            (?:<(?P<state>[^>]*)>)?
            \s+mtu\s+(?P<mtu>[0-9]+)",
        )
        .map_err(|e| ParseError::Regex(e.to_string()))?;

        let re_openbsd_ipv4 = Regex::new(
            r"inet\s(?P<address>(?:[0-9]{1,3}\.){3}[0-9]{1,3})\s+netmask\s+(?P<mask>(?:[0-9]{1,3}\.){3}[0-9]{1,3}|0x[0-9a-fA-F]+)(?:\s+broadcast\s+(?P<broadcast>(?:[0-9]{1,3}\.){3}[0-9]{1,3}))?",
        )
        .map_err(|e| ParseError::Regex(e.to_string()))?;

        // Modern Linux/Ubuntu IPv6: "inet6 ADDR prefixlen MASK scopeid 0x20<link>"
        // Requires <type> at end to distinguish from FreeBSD format
        let re_openbsd_ipv6 = Regex::new(
            r"inet6\s+(?P<address>\S+)\s+prefixlen\s+(?P<mask>[0-9]+)\s+scopeid\s+(?P<scope>[0-9a-fA-F]+x[0-9a-fA-F]+)<(?P<type>link|host|global|compat)>",
        )
        .map_err(|e| ParseError::Regex(e.to_string()))?;

        let re_openbsd_details = Regex::new(
            r"\S+\s+(?:(?P<mac_addr>[0-9A-Fa-f:?]+)\s+)?txqueuelen\s+[0-9]+\s+\((?P<type>[^)]+)\)",
        )
        .map_err(|e| ParseError::Regex(e.to_string()))?;

        let re_openbsd_rx =
            Regex::new(r"RX\spackets\s(?P<rx_packets>[0-9]+)\s+bytes\s+(?P<rx_bytes>\d+)")
                .map_err(|e| ParseError::Regex(e.to_string()))?;

        let re_openbsd_rx_stats = Regex::new(
            r"RX\serrors\s(?P<rx_errors>[0-9]+)\s+dropped\s+(?P<rx_dropped>[0-9]+)\s+overruns\s+(?P<rx_overruns>[0-9]+)\s+frame\s+(?P<rx_frame>[0-9]+)",
        )
        .map_err(|e| ParseError::Regex(e.to_string()))?;

        let re_openbsd_tx =
            Regex::new(r"TX\spackets\s(?P<tx_packets>[0-9]+)\s+bytes\s+(?P<tx_bytes>\d+)")
                .map_err(|e| ParseError::Regex(e.to_string()))?;

        let re_openbsd_tx_stats = Regex::new(
            r"TX\serrors\s(?P<tx_errors>[0-9]+)\s+dropped\s+(?P<tx_dropped>[0-9]+)\s+overruns\s+(?P<tx_overruns>[0-9]+)\s+carrier\s+(?P<tx_carrier>[0-9]+)\s+collisions\s+(?P<tx_collisions>[0-9]+)",
        )
        .map_err(|e| ParseError::Regex(e.to_string()))?;

        // FreeBSD/macOS style (flags= with metric)
        let re_freebsd_iface = Regex::new(
            r"(?x)^(?P<name>[a-zA-Z0-9:._-]+):\s+
            flags=(?P<flags>[0-9]+)
            (?:<(?P<state>[^>]*)>)?
            \s+metric\s+(?P<metric>[0-9]+)
            \s+mtu\s+(?P<mtu>[0-9]+)",
        )
        .map_err(|e| ParseError::Regex(e.to_string()))?;

        let re_freebsd_ipv4 = Regex::new(
            r"inet\s(?P<address>(?:[0-9]{1,3}\.){3}[0-9]{1,3})\s+netmask\s+(?P<mask>0x[0-9a-fA-F]+)(?:\s+broadcast\s+(?P<broadcast>(?:[0-9]{1,3}\.){3}[0-9]{1,3}))?",
        )
        .map_err(|e| ParseError::Regex(e.to_string()))?;

        let re_freebsd_ipv4_v2 = Regex::new(
            r"inet\s(?P<address>(?:[0-9]{1,3}\.){3}[0-9]{1,3})/(?P<mask>\d+)(?:\s+broadcast\s+(?P<broadcast>(?:[0-9]{1,3}\.){3}[0-9]{1,3}))?",
        )
        .map_err(|e| ParseError::Regex(e.to_string()))?;

        // FreeBSD/macOS IPv6: "inet6 ADDR%scope_id prefixlen MASK scopeid 0x1"
        // The address may optionally have %scope_id suffix
        let re_freebsd_ipv6 = Regex::new(
            r"inet6\s(?P<address>[^\s%]+)(?:%(?P<scope_id>\S+))?\s+prefixlen\s+(?P<mask>\d+)(?:[^\n]*\sscopeid\s+(?P<scope>0x\w+))?",
        )
        .map_err(|e| ParseError::Regex(e.to_string()))?;

        let re_freebsd_ether = Regex::new(r"ether\s+(?P<mac_addr>[0-9A-Fa-f:]+)")
            .map_err(|e| ParseError::Regex(e.to_string()))?;

        let re_freebsd_status = Regex::new(r"status:\s+(?P<status>\w+)")
            .map_err(|e| ParseError::Regex(e.to_string()))?;

        let re_freebsd_nd6 = Regex::new(r"nd6\soptions=(?P<nd6_options>\d+)<(?P<nd6_flags>[^>]+)>")
            .map_err(|e| ParseError::Regex(e.to_string()))?;

        let re_freebsd_options =
            Regex::new(r"options=(?P<options>[0-9a-fA-F]+)<(?P<options_flags>[^>]+)>")
                .map_err(|e| ParseError::Regex(e.to_string()))?;

        let re_freebsd_media = Regex::new(r"media:\s+(?P<media>.+?)\s+<(?P<media_flags>[^>]+)>")
            .map_err(|e| ParseError::Regex(e.to_string()))?;

        // FreeBSD extra fields
        let re_freebsd_hwaddr = Regex::new(
            r"hwaddr\s+(?P<hw_address>[0-9A-Fa-f:]+)(?:\s+media:\s+(?P<media>.+?)\s+<(?P<media_flags>[^>]+)>)?",
        )
        .map_err(|e| ParseError::Regex(e.to_string()))?;

        let re_freebsd_plugged = Regex::new(r"plugged:\s+(?P<plugged>.+)")
            .map_err(|e| ParseError::Regex(e.to_string()))?;

        let re_freebsd_vendor = Regex::new(
            r"vendor:\s+(?P<vendor>.+?)\s+PN:\s+(?P<vendor_pn>\S+)\s+SN:\s+(?P<vendor_sn>\S+)\s+DATE:\s+(?P<vendor_date>\S+)",
        )
        .map_err(|e| ParseError::Regex(e.to_string()))?;

        let re_freebsd_temp_volts = Regex::new(
            r"(?i)module\s+temperature:\s+(?P<module_temperature>.+?)\s+voltage:\s+(?P<module_voltage>.+)",
        )
        .map_err(|e| ParseError::Regex(e.to_string()))?;

        let re_freebsd_lane = Regex::new(
            r"lane\s+(?P<lane>\d+):\s+RX\s+power:\s+(?P<rx_power_mw>\S+)\s+mW\s+\((?P<rx_power_dbm>\S+)\s+dBm\)\s+TX\s+bias:\s+(?P<tx_bias_ma>\S+)",
        )
        .map_err(|e| ParseError::Regex(e.to_string()))?;

        let re_freebsd_tx_rx_power = Regex::new(r"RX:\s+(?P<rx_power>.+)\s+TX:\s+(?P<tx_pwer>.+)")
            .map_err(|e| ParseError::Regex(e.to_string()))?;

        let mut raw_output: Vec<Map<String, Value>> = Vec::new();
        let mut interface_item = null_iface();
        let mut ipv4_info: Vec<Value> = Vec::new();
        let mut ipv6_info: Vec<Value> = Vec::new();
        let mut lane_info: Vec<Value> = Vec::new();

        let lines: Vec<&str> = input.lines().filter(|l| !l.trim().is_empty()).collect();

        for line in &lines {
            // Check for new interface (try all styles, FreeBSD first since it's most specific)
            let iface_match_linux = re_linux_iface.captures(line);
            let iface_match_freebsd = re_freebsd_iface.captures(line);
            let iface_match_openbsd = re_openbsd_iface.captures(line);

            let iface_match = iface_match_freebsd
                .as_ref()
                .or(iface_match_linux.as_ref())
                .or(iface_match_openbsd.as_ref());

            if let Some(caps) = iface_match {
                // Save previous interface
                if interface_item.get("name") != Some(&Value::Null) {
                    if !ipv4_info.is_empty() {
                        interface_item.insert("ipv4".to_string(), Value::Array(ipv4_info.clone()));
                    }
                    if !ipv6_info.is_empty() {
                        interface_item.insert("ipv6".to_string(), Value::Array(ipv6_info.clone()));
                    }
                    if !lane_info.is_empty() {
                        interface_item.insert("lanes".to_string(), Value::Array(lane_info.clone()));
                    }
                    raw_output.push(interface_item.clone());
                }

                interface_item = null_iface();
                ipv4_info = Vec::new();
                ipv6_info = Vec::new();
                lane_info = Vec::new();

                if let Some(n) = caps.name("name") {
                    interface_item
                        .insert("name".to_string(), Value::String(n.as_str().to_string()));
                }
                if let Some(f) = caps.name("flags") {
                    let flags_str = f.as_str();
                    // Parse as decimal (jc always uses decimal for flags)
                    if let Ok(n) = flags_str.parse::<i64>() {
                        interface_item.insert("flags".to_string(), Value::Number(n.into()));
                    }
                }
                if let Some(s) = caps.name("state") {
                    let state_str = s.as_str().trim();
                    if !state_str.is_empty() {
                        let state_vec: Vec<Value> = state_str
                            .split(',')
                            .map(|x| Value::String(x.trim().to_string()))
                            .collect();
                        interface_item.insert("state".to_string(), Value::Array(state_vec));
                    }
                    // empty <> → leave state as null
                }
                if let Some(m) = caps.name("mtu")
                    && let Ok(n) = m.as_str().parse::<i64>()
                {
                    interface_item.insert("mtu".to_string(), Value::Number(n.into()));
                }
                if let Some(mt) = caps.name("metric")
                    && let Ok(n) = mt.as_str().parse::<i64>()
                {
                    interface_item.insert("metric".to_string(), Value::Number(n.into()));
                }
                if let Some(t) = caps.name("type") {
                    interface_item.insert(
                        "type".to_string(),
                        Value::String(t.as_str().trim().to_string()),
                    );
                }
                if let Some(mac) = caps.name("mac_addr") {
                    interface_item.insert(
                        "mac_addr".to_string(),
                        Value::String(mac.as_str().to_string()),
                    );
                }
                continue;
            }

            // Parse IPv4
            let ipv4_caps = re_linux_ipv4
                .captures(line)
                .or_else(|| re_openbsd_ipv4.captures(line))
                .or_else(|| re_freebsd_ipv4.captures(line))
                .or_else(|| re_freebsd_ipv4_v2.captures(line));

            if let Some(caps) = ipv4_caps {
                let address = caps.name("address").map(|m| m.as_str()).unwrap_or("");
                let mask_raw = caps.name("mask").map(|m| m.as_str()).unwrap_or("");
                let mask = convert_hex_mask(mask_raw);
                let broadcast = caps
                    .name("broadcast")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .to_string();

                // Legacy top-level fields (last wins)
                interface_item.insert("ipv4_addr".to_string(), Value::String(address.to_string()));
                interface_item.insert("ipv4_mask".to_string(), Value::String(mask.clone()));
                if !broadcast.is_empty() {
                    interface_item
                        .insert("ipv4_bcast".to_string(), Value::String(broadcast.clone()));
                }

                let mut ipv4_obj = Map::new();
                ipv4_obj.insert("address".to_string(), Value::String(address.to_string()));
                ipv4_obj.insert("mask".to_string(), Value::String(mask));
                ipv4_obj.insert(
                    "broadcast".to_string(),
                    if broadcast.is_empty() {
                        Value::Null
                    } else {
                        Value::String(broadcast)
                    },
                );
                ipv4_info.push(Value::Object(ipv4_obj));
                continue;
            }

            // Parse IPv6 - try Linux old format first, then OpenBSD (requires <type>), then FreeBSD
            if let Some(caps) = re_linux_ipv6.captures(line) {
                // Old Linux: inet6 addr: ADDR/MASK Scope:TYPE → {address, mask, scope}
                let address = caps.name("address").map(|m| m.as_str()).unwrap_or("");
                let mask = caps.name("mask").map(|m| m.as_str()).unwrap_or("0");
                let scope = caps.name("scope").map(|m| m.as_str()).unwrap_or("");

                // Legacy
                interface_item.insert("ipv6_addr".to_string(), Value::String(address.to_string()));
                if let Ok(n) = mask.parse::<i64>() {
                    interface_item.insert("ipv6_mask".to_string(), Value::Number(n.into()));
                }
                if !scope.is_empty() {
                    interface_item
                        .insert("ipv6_scope".to_string(), Value::String(scope.to_string()));
                }

                let mut ipv6_obj = Map::new();
                ipv6_obj.insert("address".to_string(), Value::String(address.to_string()));
                if let Ok(n) = mask.parse::<i64>() {
                    ipv6_obj.insert("mask".to_string(), Value::Number(n.into()));
                }
                ipv6_obj.insert(
                    "scope".to_string(),
                    if scope.is_empty() {
                        Value::Null
                    } else {
                        Value::String(scope.to_string())
                    },
                );
                ipv6_info.push(Value::Object(ipv6_obj));
                continue;
            }

            if let Some(caps) = re_openbsd_ipv6.captures(line) {
                // Modern Linux/Ubuntu: inet6 ADDR prefixlen MASK scopeid 0x20<type>
                // → {address, mask, scope, type} (no scope_id)
                let address = caps.name("address").map(|m| m.as_str()).unwrap_or("");
                let mask = caps.name("mask").map(|m| m.as_str()).unwrap_or("0");
                let scope = caps.name("scope").map(|m| m.as_str()).unwrap_or("");
                let ipv6_type = caps.name("type").map(|m| m.as_str()).unwrap_or("");

                // Legacy
                interface_item.insert("ipv6_addr".to_string(), Value::String(address.to_string()));
                if let Ok(n) = mask.parse::<i64>() {
                    interface_item.insert("ipv6_mask".to_string(), Value::Number(n.into()));
                }
                if !scope.is_empty() {
                    interface_item
                        .insert("ipv6_scope".to_string(), Value::String(scope.to_string()));
                }
                if !ipv6_type.is_empty() {
                    interface_item.insert(
                        "ipv6_type".to_string(),
                        Value::String(ipv6_type.to_string()),
                    );
                }

                let mut ipv6_obj = Map::new();
                ipv6_obj.insert("address".to_string(), Value::String(address.to_string()));
                if let Ok(n) = mask.parse::<i64>() {
                    ipv6_obj.insert("mask".to_string(), Value::Number(n.into()));
                }
                ipv6_obj.insert(
                    "scope".to_string(),
                    if scope.is_empty() {
                        Value::Null
                    } else {
                        Value::String(scope.to_string())
                    },
                );
                ipv6_obj.insert(
                    "type".to_string(),
                    if ipv6_type.is_empty() {
                        Value::Null
                    } else {
                        Value::String(ipv6_type.to_string())
                    },
                );
                ipv6_info.push(Value::Object(ipv6_obj));
                continue;
            }

            if let Some(caps) = re_freebsd_ipv6.captures(line) {
                // FreeBSD/macOS: inet6 ADDR%scope_id prefixlen MASK scopeid 0x1
                // → {address, scope_id, mask, scope} (no type)
                let address = caps.name("address").map(|m| m.as_str()).unwrap_or("");
                let scope_id = caps.name("scope_id").map(|m| m.as_str()).unwrap_or("");
                let mask = caps.name("mask").map(|m| m.as_str()).unwrap_or("0");
                let scope = caps.name("scope").map(|m| m.as_str()).unwrap_or("");

                // Legacy top-level (last wins, always update)
                interface_item.insert("ipv6_addr".to_string(), Value::String(address.to_string()));
                if let Ok(n) = mask.parse::<i64>() {
                    interface_item.insert("ipv6_mask".to_string(), Value::Number(n.into()));
                }
                // Always update scope (last wins, even if empty → null)
                interface_item.insert(
                    "ipv6_scope".to_string(),
                    if scope.is_empty() {
                        Value::Null
                    } else {
                        Value::String(scope.to_string())
                    },
                );
                // Always update scope_id
                interface_item.insert(
                    "ipv6_scope_id".to_string(),
                    if scope_id.is_empty() {
                        Value::Null
                    } else {
                        Value::String(scope_id.to_string())
                    },
                );

                let mut ipv6_obj = Map::new();
                ipv6_obj.insert("address".to_string(), Value::String(address.to_string()));
                ipv6_obj.insert(
                    "scope_id".to_string(),
                    if scope_id.is_empty() {
                        Value::Null
                    } else {
                        Value::String(scope_id.to_string())
                    },
                );
                if let Ok(n) = mask.parse::<i64>() {
                    ipv6_obj.insert("mask".to_string(), Value::Number(n.into()));
                }
                ipv6_obj.insert(
                    "scope".to_string(),
                    if scope.is_empty() {
                        Value::Null
                    } else {
                        Value::String(scope.to_string())
                    },
                );
                ipv6_info.push(Value::Object(ipv6_obj));
                continue;
            }

            // FreeBSD lane info
            if let Some(caps) = re_freebsd_lane.captures(line) {
                let mut lane_obj = Map::new();
                if let Some(m) = caps.name("lane")
                    && let Ok(n) = m.as_str().parse::<i64>()
                {
                    lane_obj.insert("lane".to_string(), Value::Number(n.into()));
                }
                if let Some(m) = caps.name("rx_power_mw")
                    && let Ok(f) = m.as_str().parse::<f64>()
                {
                    lane_obj.insert(
                        "rx_power_mw".to_string(),
                        Value::Number(serde_json::Number::from_f64(f).unwrap()),
                    );
                }
                if let Some(m) = caps.name("rx_power_dbm")
                    && let Ok(f) = m.as_str().parse::<f64>()
                {
                    lane_obj.insert(
                        "rx_power_dbm".to_string(),
                        Value::Number(serde_json::Number::from_f64(f).unwrap()),
                    );
                }
                if let Some(m) = caps.name("tx_bias_ma")
                    && let Ok(f) = m.as_str().parse::<f64>()
                {
                    lane_obj.insert(
                        "tx_bias_ma".to_string(),
                        Value::Number(serde_json::Number::from_f64(f).unwrap()),
                    );
                }
                lane_info.push(Value::Object(lane_obj));
                continue;
            }

            // Other patterns
            if let Some(caps) = re_linux_state.captures(line) {
                if let Some(s) = caps.name("state") {
                    let state_str = s.as_str(); // keep trailing space, jc does not trim
                    if !state_str.trim().is_empty() {
                        let state_vec: Vec<Value> = state_str
                            .split(',')
                            .map(|x| Value::String(x.to_string()))
                            .collect();
                        interface_item.insert("state".to_string(), Value::Array(state_vec));
                    }
                }
                if let Some(m) = caps.name("mtu")
                    && let Ok(n) = m.as_str().parse::<i64>()
                {
                    interface_item.insert("mtu".to_string(), Value::Number(n.into()));
                }
                if let Some(mt) = caps.name("metric")
                    && let Ok(n) = mt.as_str().parse::<i64>()
                {
                    interface_item.insert("metric".to_string(), Value::Number(n.into()));
                }
                continue;
            }

            if let Some(caps) = re_linux_rx.captures(line) {
                for field in &[
                    "rx_packets",
                    "rx_errors",
                    "rx_dropped",
                    "rx_overruns",
                    "rx_frame",
                ] {
                    if let Some(m) = caps.name(field)
                        && let Ok(n) = m.as_str().parse::<i64>()
                    {
                        interface_item.insert(field.to_string(), Value::Number(n.into()));
                    }
                }
                continue;
            }

            if let Some(caps) = re_linux_tx.captures(line) {
                for field in &[
                    "tx_packets",
                    "tx_errors",
                    "tx_dropped",
                    "tx_overruns",
                    "tx_carrier",
                ] {
                    if let Some(m) = caps.name(field)
                        && let Ok(n) = m.as_str().parse::<i64>()
                    {
                        interface_item.insert(field.to_string(), Value::Number(n.into()));
                    }
                }
                continue;
            }

            if let Some(caps) = re_linux_bytes.captures(line) {
                if let Some(m) = caps.name("rx_bytes")
                    && let Ok(n) = m.as_str().parse::<i64>()
                {
                    interface_item.insert("rx_bytes".to_string(), Value::Number(n.into()));
                }
                if let Some(m) = caps.name("tx_bytes")
                    && let Ok(n) = m.as_str().parse::<i64>()
                {
                    interface_item.insert("tx_bytes".to_string(), Value::Number(n.into()));
                }
                continue;
            }

            if let Some(caps) = re_linux_tx_stats.captures(line) {
                if let Some(m) = caps.name("tx_collisions")
                    && let Ok(n) = m.as_str().parse::<i64>()
                {
                    interface_item.insert("tx_collisions".to_string(), Value::Number(n.into()));
                }
                continue;
            }

            if let Some(caps) = re_openbsd_details.captures(line) {
                if let Some(mac) = caps.name("mac_addr") {
                    interface_item.insert(
                        "mac_addr".to_string(),
                        Value::String(mac.as_str().to_string()),
                    );
                }
                if let Some(t) = caps.name("type") {
                    interface_item
                        .insert("type".to_string(), Value::String(t.as_str().to_string()));
                }
                continue;
            }

            if let Some(caps) = re_openbsd_rx.captures(line) {
                if let Some(m) = caps.name("rx_packets")
                    && let Ok(n) = m.as_str().parse::<i64>()
                {
                    interface_item.insert("rx_packets".to_string(), Value::Number(n.into()));
                }
                if let Some(m) = caps.name("rx_bytes")
                    && let Ok(n) = m.as_str().parse::<i64>()
                {
                    interface_item.insert("rx_bytes".to_string(), Value::Number(n.into()));
                }
                continue;
            }

            if let Some(caps) = re_openbsd_rx_stats.captures(line) {
                for field in &["rx_errors", "rx_dropped", "rx_overruns", "rx_frame"] {
                    if let Some(m) = caps.name(field)
                        && let Ok(n) = m.as_str().parse::<i64>()
                    {
                        interface_item.insert(field.to_string(), Value::Number(n.into()));
                    }
                }
                continue;
            }

            if let Some(caps) = re_openbsd_tx.captures(line) {
                if let Some(m) = caps.name("tx_packets")
                    && let Ok(n) = m.as_str().parse::<i64>()
                {
                    interface_item.insert("tx_packets".to_string(), Value::Number(n.into()));
                }
                if let Some(m) = caps.name("tx_bytes")
                    && let Ok(n) = m.as_str().parse::<i64>()
                {
                    interface_item.insert("tx_bytes".to_string(), Value::Number(n.into()));
                }
                continue;
            }

            if let Some(caps) = re_openbsd_tx_stats.captures(line) {
                for field in &[
                    "tx_errors",
                    "tx_dropped",
                    "tx_overruns",
                    "tx_carrier",
                    "tx_collisions",
                ] {
                    if let Some(m) = caps.name(field)
                        && let Ok(n) = m.as_str().parse::<i64>()
                    {
                        interface_item.insert(field.to_string(), Value::Number(n.into()));
                    }
                }
                continue;
            }

            if let Some(caps) = re_freebsd_ether.captures(line) {
                if let Some(mac) = caps.name("mac_addr") {
                    interface_item.insert(
                        "mac_addr".to_string(),
                        Value::String(mac.as_str().to_string()),
                    );
                }
                continue;
            }

            if let Some(caps) = re_freebsd_status.captures(line) {
                if let Some(s) = caps.name("status") {
                    interface_item
                        .insert("status".to_string(), Value::String(s.as_str().to_string()));
                }
                continue;
            }

            if let Some(caps) = re_freebsd_nd6.captures(line) {
                if let Some(opts) = caps.name("nd6_options")
                    && let Ok(n) = opts.as_str().parse::<i64>()
                {
                    interface_item.insert("nd6_options".to_string(), Value::Number(n.into()));
                }
                if let Some(flags) = caps.name("nd6_flags") {
                    let flag_vec: Vec<Value> = flags
                        .as_str()
                        .split(',')
                        .map(|f| Value::String(f.to_string()))
                        .collect();
                    interface_item.insert("nd6_flags".to_string(), Value::Array(flag_vec));
                }
                continue;
            }

            // FreeBSD hwaddr (may also include media on same line)
            if let Some(caps) = re_freebsd_hwaddr.captures(line) {
                if let Some(hw) = caps.name("hw_address") {
                    interface_item.insert(
                        "hw_address".to_string(),
                        Value::String(hw.as_str().to_string()),
                    );
                }
                // Always insert media/media_flags (null if not captured)
                if let Some(m) = caps.name("media") {
                    interface_item
                        .insert("media".to_string(), Value::String(m.as_str().to_string()));
                } else {
                    interface_item.insert("media".to_string(), Value::Null);
                }
                if let Some(flags) = caps.name("media_flags") {
                    let flag_vec: Vec<Value> = flags
                        .as_str()
                        .split(',')
                        .map(|f| Value::String(f.trim().to_string()))
                        .collect();
                    interface_item.insert("media_flags".to_string(), Value::Array(flag_vec));
                } else {
                    interface_item.insert("media_flags".to_string(), Value::Null);
                }
                continue;
            }

            if let Some(caps) = re_freebsd_plugged.captures(line) {
                if let Some(p) = caps.name("plugged") {
                    interface_item
                        .insert("plugged".to_string(), Value::String(p.as_str().to_string()));
                }
                continue;
            }

            if let Some(caps) = re_freebsd_vendor.captures(line) {
                if let Some(v) = caps.name("vendor") {
                    interface_item
                        .insert("vendor".to_string(), Value::String(v.as_str().to_string()));
                }
                if let Some(pn) = caps.name("vendor_pn") {
                    interface_item.insert(
                        "vendor_pn".to_string(),
                        Value::String(pn.as_str().to_string()),
                    );
                }
                if let Some(sn) = caps.name("vendor_sn") {
                    interface_item.insert(
                        "vendor_sn".to_string(),
                        Value::String(sn.as_str().to_string()),
                    );
                }
                if let Some(date) = caps.name("vendor_date") {
                    interface_item.insert(
                        "vendor_date".to_string(),
                        Value::String(date.as_str().to_string()),
                    );
                }
                continue;
            }

            if let Some(caps) = re_freebsd_temp_volts.captures(line) {
                if let Some(t) = caps.name("module_temperature") {
                    interface_item.insert(
                        "module_temperature".to_string(),
                        Value::String(t.as_str().to_string()),
                    );
                }
                if let Some(v) = caps.name("module_voltage") {
                    interface_item.insert(
                        "module_voltage".to_string(),
                        Value::String(v.as_str().to_string()),
                    );
                }
                continue;
            }

            if let Some(caps) = re_freebsd_options.captures(line) {
                if let Some(opts) = caps.name("options") {
                    interface_item.insert(
                        "options".to_string(),
                        Value::String(opts.as_str().to_string()),
                    );
                }
                if let Some(flags) = caps.name("options_flags") {
                    let flag_vec: Vec<Value> = flags
                        .as_str()
                        .split(',')
                        .map(|f| Value::String(f.to_string()))
                        .collect();
                    interface_item.insert("options_flags".to_string(), Value::Array(flag_vec));
                }
                continue;
            }

            if let Some(caps) = re_freebsd_media.captures(line) {
                if let Some(m) = caps.name("media") {
                    interface_item
                        .insert("media".to_string(), Value::String(m.as_str().to_string()));
                }
                if let Some(flags) = caps.name("media_flags") {
                    let flag_vec: Vec<Value> = flags
                        .as_str()
                        .split(',')
                        .map(|f| Value::String(f.trim().to_string()))
                        .collect();
                    interface_item.insert("media_flags".to_string(), Value::Array(flag_vec));
                }
                continue;
            }

            if let Some(caps) = re_freebsd_tx_rx_power.captures(line) {
                if let Some(rx) = caps.name("rx_power") {
                    interface_item.insert(
                        "rx_power".to_string(),
                        Value::String(rx.as_str().trim().to_string()),
                    );
                }
                if let Some(tx) = caps.name("tx_pwer") {
                    interface_item.insert(
                        "tx_pwer".to_string(),
                        Value::String(tx.as_str().trim().to_string()),
                    );
                }
                continue;
            }
        }

        // Flush last interface
        if interface_item.get("name") != Some(&Value::Null) {
            if !ipv4_info.is_empty() {
                interface_item.insert("ipv4".to_string(), Value::Array(ipv4_info));
            }
            if !ipv6_info.is_empty() {
                interface_item.insert("ipv6".to_string(), Value::Array(ipv6_info));
            }
            if !lane_info.is_empty() {
                interface_item.insert("lanes".to_string(), Value::Array(lane_info));
            }
            raw_output.push(interface_item);
        }

        Ok(ParseOutput::Array(raw_output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jc_rs_core::traits::Parser;

    #[test]
    fn test_ifconfig_centos_golden() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/ifconfig.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/ifconfig.json"
        ))
        .unwrap();
        let result = IfconfigParser.parse(input, false).unwrap();
        let actual = serde_json::to_value(result).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_ifconfig_empty() {
        let result = IfconfigParser.parse("", false).unwrap();
        assert!(matches!(result, ParseOutput::Array(v) if v.is_empty()));
    }

    #[test]
    fn test_ifconfig_registered() {
        assert!(jc_rs_core::registry::find_parser("ifconfig").is_some());
    }
}
