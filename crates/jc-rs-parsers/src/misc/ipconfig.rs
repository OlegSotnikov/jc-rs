//! Windows `ipconfig` command parser.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::parse_timestamp;
use serde_json::{Map, Value};

struct IpconfigParser;

static INFO: ParserInfo = ParserInfo {
    name: "ipconfig",
    argument: "--ipconfig",
    version: "1.0.0",
    description: "Windows `ipconfig` command parser",
    author: "jc-rs contributors",
    author_email: "jc-rs@example.com",
    compatible: &[Platform::Windows],
    tags: &[Tag::Command],
    magic_commands: &["ipconfig"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

fn parse_yes_no_enabled(s: &str) -> Option<bool> {
    let lower = s.trim().to_lowercase();
    match lower.as_str() {
        "yes" | "enabled" => Some(true),
        "no" | "disabled" => Some(false),
        _ => None,
    }
}

/// Parse an IPv6 address string like "fd7a:...:47b2(Preferred)" or "fe80::...%8(Preferred)".
/// Returns (address_without_status, status).
fn parse_ipv6_addr(s: &str) -> (String, Option<String>) {
    let s = s.trim();
    if let Some(paren_pos) = s.rfind('(') {
        if s.ends_with(')') {
            let addr = s[..paren_pos].trim().to_string();
            let status = s[paren_pos + 1..s.len() - 1].to_string();
            return (addr, Some(status));
        }
    }
    (s.to_string(), None)
}

/// Parse an IPv4 address like "100.115.71.66(Preferred)" or "169.254.x.x(Autoconfiguration)".
fn parse_ipv4_addr(s: &str) -> (String, Option<String>, bool) {
    let s = s.trim();
    if let Some(paren_pos) = s.rfind('(') {
        if s.ends_with(')') {
            let addr = s[..paren_pos].trim().to_string();
            let status_raw = &s[paren_pos + 1..s.len() - 1];
            let autoconfigured = status_raw.to_lowercase().contains("autoconfiguration");
            let status = if autoconfigured {
                None // autoconfigured addresses may have no display status
            } else {
                Some(status_raw.to_string())
            };
            return (addr, status, autoconfigured);
        }
    }
    (s.to_string(), None, false)
}

/// Extract link-local IPv6 prefix length from address like "fe80::...%16(Preferred)".
fn parse_link_local_ipv6(s: &str) -> (String, Option<i64>, Option<String>) {
    let s = s.trim();
    let (addr_with_prefix, status) = if let Some(paren_pos) = s.rfind('(') {
        if s.ends_with(')') {
            let a = s[..paren_pos].trim();
            let st = s[paren_pos + 1..s.len() - 1].to_string();
            (a, Some(st))
        } else {
            (s, None)
        }
    } else {
        (s, None)
    };

    // Extract %N (interface index) as prefix_length
    let (addr, prefix_length) = if let Some(pct_pos) = addr_with_prefix.rfind('%') {
        let addr = &addr_with_prefix[..pct_pos];
        let prefix: Option<i64> = addr_with_prefix[pct_pos + 1..].parse::<i64>().ok();
        (addr.to_string(), prefix)
    } else {
        (addr_with_prefix.to_string(), None)
    };

    (addr, prefix_length, status)
}

/// Parse a lease date string and return (raw, epoch, iso).
fn parse_lease_date(s: &str) -> (String, Option<i64>, Option<String>) {
    let raw = s.trim().to_string();
    let parsed = parse_timestamp(&raw, Some("%A, %B %d, %Y %I:%M:%S %p"));
    (raw, parsed.naive_epoch, parsed.iso)
}

/// Normalize a label by stripping Windows ipconfig filler ". . . . ." patterns.
fn normalize_label(label: &str) -> String {
    let s = label.trim();
    // Strip trailing dot-space filler (e.g. ". . . . ." or ". . ." or plain dots).
    // Labels: "Host Name . . . . .", "IP Routing Enabled. . . . .", "Primary Dns Suffix  . . ."
    let bytes = s.as_bytes();
    let mut end = bytes.len();
    while end > 0 && (bytes[end - 1] == b'.' || bytes[end - 1] == b' ') {
        end -= 1;
    }
    s[..end].trim().to_string()
}

/// Try to extract (label, value) from a line like "   Label . . . : Value".
fn parse_kv_line(line: &str) -> Option<(String, String)> {
    // Find " : " separator
    if let Some(sep_pos) = line.find(" : ") {
        let label_raw = &line[..sep_pos];
        let value = line[sep_pos + 3..].to_string();
        let label = normalize_label(label_raw);
        if !label.is_empty() {
            return Some((label, value));
        }
    }
    None
}

/// Detect adapter header: "Type adapter Name:" at start of line (no leading spaces).
fn parse_adapter_header(line: &str) -> Option<(String, String, String)> {
    // adapter header lines start without leading spaces and end with ":"
    if line.starts_with(' ') || line.starts_with('\t') {
        return None;
    }
    let trimmed = line.trim();
    if !trimmed.ends_with(':') {
        return None;
    }
    // Skip non-adapter header lines
    if trimmed.starts_with("Windows IP")
        || trimmed.starts_with("The command")
        || trimmed.starts_with("---")
    {
        return None;
    }

    let line_no_colon = &trimmed[..trimmed.len() - 1];

    // Detect type prefix
    let (adapter_type, name) = if let Some(rest) = line_no_colon.strip_prefix("Ethernet adapter ") {
        ("Ethernet".to_string(), rest.to_string())
    } else if let Some(rest) = line_no_colon.strip_prefix("Unknown adapter ") {
        ("Unknown".to_string(), rest.to_string())
    } else if let Some(rest) = line_no_colon.strip_prefix("PPP adapter ") {
        ("PPP".to_string(), rest.to_string())
    } else if let Some(rest) = line_no_colon.strip_prefix("Tunnel adapter ") {
        ("Tunnel".to_string(), rest.to_string())
    } else if let Some(rest) = line_no_colon.strip_prefix("Wireless LAN adapter ") {
        ("Wireless LAN".to_string(), rest.to_string())
    } else {
        return None;
    };

    Some((line_no_colon.to_string(), adapter_type, name))
}

fn new_adapter(name_long: &str, adapter_type: &str, name: &str) -> Map<String, Value> {
    let mut a = Map::new();
    a.insert(
        "name_long".to_string(),
        Value::String(name_long.to_string()),
    );
    a.insert("name".to_string(), Value::String(name.to_string()));
    a.insert("type".to_string(), Value::String(adapter_type.to_string()));
    a.insert("connection_specific_dns_suffix".to_string(), Value::Null);
    a.insert(
        "connection_specific_dns_suffix_search_list".to_string(),
        Value::Array(vec![]),
    );
    a.insert("description".to_string(), Value::Null);
    a.insert("physical_address".to_string(), Value::Null);
    a.insert("dhcp_enabled".to_string(), Value::Null);
    a.insert("autoconfiguration_enabled".to_string(), Value::Null);
    a.insert("ipv6_addresses".to_string(), Value::Array(vec![]));
    a.insert("temporary_ipv6_addresses".to_string(), Value::Array(vec![]));
    a.insert(
        "link_local_ipv6_addresses".to_string(),
        Value::Array(vec![]),
    );
    a.insert("ipv4_addresses".to_string(), Value::Array(vec![]));
    a.insert("default_gateways".to_string(), Value::Array(vec![]));
    a.insert("dhcp_server".to_string(), Value::Null);
    a.insert("dhcpv6_iaid".to_string(), Value::Null);
    a.insert("dhcpv6_client_duid".to_string(), Value::Null);
    a.insert("dns_servers".to_string(), Value::Array(vec![]));
    a.insert("primary_wins_server".to_string(), Value::Null);
    a.insert("lease_expires".to_string(), Value::Null);
    a.insert("lease_obtained".to_string(), Value::Null);
    a.insert("netbios_over_tcpip".to_string(), Value::Null);
    a.insert("media_state".to_string(), Value::Null);
    a.insert("extras".to_string(), Value::Array(vec![]));
    a
}

pub fn parse_ipconfig(input: &str) -> Map<String, Value> {
    let mut obj = Map::new();
    let mut host_name: Option<String> = None;
    let mut primary_dns_suffix: Option<String> = None;
    let mut node_type: Option<String> = None;
    let mut ip_routing_enabled: Option<bool> = None;
    let mut wins_proxy_enabled: Option<bool> = None;
    let mut global_dns_suffix_list: Vec<Value> = Vec::new();
    let mut adapters: Vec<Map<String, Value>> = Vec::new();

    // Current adapter being parsed
    let mut current_adapter: Option<Map<String, Value>> = None;
    // Last label seen (for multi-value/continuation)
    let mut last_label: Option<String> = None;
    // For subnet mask: associate with last ipv4 address
    // For DNS servers and gateways: accumulate
    let mut in_global = true;

    let push_current_adapter =
        |adapters: &mut Vec<Map<String, Value>>, adapter: &mut Option<Map<String, Value>>| {
            if let Some(a) = adapter.take() {
                adapters.push(a);
            }
        };

    for line in input.lines() {
        // Detect adapter header (no leading spaces, ends with ":")
        if !line.starts_with(' ') && !line.starts_with('\t') {
            let trimmed = line.trim();

            if trimmed.is_empty() || trimmed == "Windows IP Configuration" {
                continue;
            }

            if let Some((name_long, adapter_type, name)) = parse_adapter_header(line) {
                push_current_adapter(&mut adapters, &mut current_adapter);
                current_adapter = Some(new_adapter(&name_long, &adapter_type, &name));
                in_global = false;
                last_label = None;
                continue;
            }

            // Non-adapter header lines at start level
            continue;
        }

        // Key-value line
        if let Some((label, value)) = parse_kv_line(line) {
            let val_trimmed = value.trim().to_string();
            last_label = Some(label.clone());

            if in_global {
                match label.as_str() {
                    "Host Name" => {
                        host_name = Some(val_trimmed);
                    }
                    "Primary Dns Suffix" => {
                        primary_dns_suffix = if val_trimmed.is_empty() {
                            None
                        } else {
                            Some(val_trimmed)
                        };
                    }
                    "Node Type" => {
                        node_type = Some(val_trimmed);
                    }
                    "IP Routing Enabled" => {
                        ip_routing_enabled = parse_yes_no_enabled(&val_trimmed);
                    }
                    "WINS Proxy Enabled" => {
                        wins_proxy_enabled = parse_yes_no_enabled(&val_trimmed);
                    }
                    "DNS Suffix Search List" => {
                        if !val_trimmed.is_empty() {
                            global_dns_suffix_list.push(Value::String(val_trimmed));
                        }
                    }
                    _ => {}
                }
            } else if let Some(ref mut adapter) = current_adapter {
                match label.as_str() {
                    "Connection-specific DNS Suffix" => {
                        adapter.insert(
                            "connection_specific_dns_suffix".to_string(),
                            if val_trimmed.is_empty() {
                                Value::Null
                            } else {
                                Value::String(val_trimmed)
                            },
                        );
                    }
                    "Connection-specific DNS Suffix Search List" => {
                        if !val_trimmed.is_empty() {
                            let mut list: Vec<Value> = match adapter
                                .remove("connection_specific_dns_suffix_search_list")
                            {
                                Some(Value::Array(a)) => a,
                                _ => vec![],
                            };
                            list.push(Value::String(val_trimmed));
                            adapter.insert(
                                "connection_specific_dns_suffix_search_list".to_string(),
                                Value::Array(list),
                            );
                        }
                    }
                    "Description" => {
                        adapter.insert("description".to_string(), Value::String(val_trimmed));
                    }
                    "Physical Address" => {
                        adapter.insert(
                            "physical_address".to_string(),
                            if val_trimmed.is_empty() {
                                Value::Null
                            } else {
                                Value::String(val_trimmed)
                            },
                        );
                    }
                    "DHCP Enabled" => {
                        adapter.insert(
                            "dhcp_enabled".to_string(),
                            parse_yes_no_enabled(&val_trimmed)
                                .map(Value::Bool)
                                .unwrap_or(Value::Null),
                        );
                    }
                    "Autoconfiguration Enabled" => {
                        adapter.insert(
                            "autoconfiguration_enabled".to_string(),
                            parse_yes_no_enabled(&val_trimmed)
                                .map(Value::Bool)
                                .unwrap_or(Value::Null),
                        );
                    }
                    "IPv6 Address" => {
                        let (addr, status) = parse_ipv6_addr(&val_trimmed);
                        let mut ipv6: Vec<Value> = match adapter.remove("ipv6_addresses") {
                            Some(Value::Array(a)) => a,
                            _ => vec![],
                        };
                        ipv6.push(serde_json::json!({
                            "address": addr,
                            "status": status.unwrap_or_default()
                        }));
                        adapter.insert("ipv6_addresses".to_string(), Value::Array(ipv6));
                    }
                    "Temporary IPv6 Address" => {
                        let (addr, status) = parse_ipv6_addr(&val_trimmed);
                        let mut tmp: Vec<Value> = match adapter.remove("temporary_ipv6_addresses") {
                            Some(Value::Array(a)) => a,
                            _ => vec![],
                        };
                        tmp.push(serde_json::json!({
                            "address": addr,
                            "status": status.unwrap_or_default()
                        }));
                        adapter.insert("temporary_ipv6_addresses".to_string(), Value::Array(tmp));
                    }
                    "Link-local IPv6 Address" => {
                        let (addr, prefix_length, status) = parse_link_local_ipv6(&val_trimmed);
                        let mut ll: Vec<Value> = match adapter.remove("link_local_ipv6_addresses") {
                            Some(Value::Array(a)) => a,
                            _ => vec![],
                        };
                        ll.push(serde_json::json!({
                            "address": addr,
                            "prefix_length": prefix_length,
                            "status": status.unwrap_or_default()
                        }));
                        adapter.insert("link_local_ipv6_addresses".to_string(), Value::Array(ll));
                    }
                    "IPv4 Address" | "IP Address" => {
                        let (addr, status, autoconfigured) = parse_ipv4_addr(&val_trimmed);
                        let mut ipv4: Vec<Value> = match adapter.remove("ipv4_addresses") {
                            Some(Value::Array(a)) => a,
                            _ => vec![],
                        };
                        ipv4.push(serde_json::json!({
                            "address": addr,
                            "subnet_mask": null,
                            "status": status,
                            "autoconfigured": autoconfigured
                        }));
                        adapter.insert("ipv4_addresses".to_string(), Value::Array(ipv4));
                    }
                    "Subnet Mask" => {
                        // Associate with last IPv4 address
                        if let Some(ipv4_arr) = adapter.get_mut("ipv4_addresses") {
                            if let Value::Array(arr) = ipv4_arr {
                                if let Some(last) = arr.last_mut() {
                                    if let Value::Object(obj) = last {
                                        obj.insert(
                                            "subnet_mask".to_string(),
                                            Value::String(val_trimmed),
                                        );
                                    }
                                }
                            }
                        }
                    }
                    "Default Gateway" => {
                        if !val_trimmed.is_empty() {
                            let mut gw: Vec<Value> = match adapter.remove("default_gateways") {
                                Some(Value::Array(a)) => a,
                                _ => vec![],
                            };
                            gw.push(Value::String(val_trimmed));
                            adapter.insert("default_gateways".to_string(), Value::Array(gw));
                        }
                    }
                    "DHCP Server" => {
                        adapter.insert("dhcp_server".to_string(), Value::String(val_trimmed));
                    }
                    "DHCPv6 IAID" => {
                        adapter.insert(
                            "dhcpv6_iaid".to_string(),
                            if val_trimmed.is_empty() {
                                Value::Null
                            } else {
                                Value::String(val_trimmed)
                            },
                        );
                    }
                    "DHCPv6 Client DUID" => {
                        adapter.insert(
                            "dhcpv6_client_duid".to_string(),
                            if val_trimmed.is_empty() {
                                Value::Null
                            } else {
                                Value::String(val_trimmed)
                            },
                        );
                    }
                    "DNS Servers" => {
                        if !val_trimmed.is_empty() {
                            let mut dns: Vec<Value> = match adapter.remove("dns_servers") {
                                Some(Value::Array(a)) => a,
                                _ => vec![],
                            };
                            dns.push(Value::String(val_trimmed));
                            adapter.insert("dns_servers".to_string(), Value::Array(dns));
                        }
                    }
                    "Primary WINS Server" => {
                        adapter.insert(
                            "primary_wins_server".to_string(),
                            Value::String(val_trimmed),
                        );
                    }
                    "Lease Obtained" => {
                        let (raw, epoch, iso) = parse_lease_date(&val_trimmed);
                        adapter.insert("lease_obtained".to_string(), Value::String(raw));
                        if let Some(e) = epoch {
                            adapter.insert(
                                "lease_obtained_epoch".to_string(),
                                Value::Number(e.into()),
                            );
                        }
                        if let Some(i) = iso {
                            adapter.insert("lease_obtained_iso".to_string(), Value::String(i));
                        }
                    }
                    "Lease Expires" => {
                        let (raw, epoch, iso) = parse_lease_date(&val_trimmed);
                        adapter.insert("lease_expires".to_string(), Value::String(raw));
                        if let Some(e) = epoch {
                            adapter
                                .insert("lease_expires_epoch".to_string(), Value::Number(e.into()));
                        }
                        if let Some(i) = iso {
                            adapter.insert("lease_expires_iso".to_string(), Value::String(i));
                        }
                    }
                    "NetBIOS over Tcpip" => {
                        adapter.insert(
                            "netbios_over_tcpip".to_string(),
                            parse_yes_no_enabled(&val_trimmed)
                                .map(Value::Bool)
                                .unwrap_or(Value::Null),
                        );
                    }
                    "Media State" => {
                        adapter.insert("media_state".to_string(), Value::String(val_trimmed));
                    }
                    _ => {}
                }
            }
        } else {
            // Potential continuation line: leading spaces, no " : " separator
            let trimmed = line.trim();
            if trimmed.is_empty() {
                last_label = None;
                continue;
            }

            // Only process continuation for known multi-value fields
            if let Some(ref label) = last_label.clone() {
                if in_global && label == "DNS Suffix Search List" {
                    global_dns_suffix_list.push(Value::String(trimmed.to_string()));
                } else if let Some(ref mut adapter) = current_adapter {
                    match label.as_str() {
                        "DNS Servers" => {
                            let mut dns: Vec<Value> = match adapter.remove("dns_servers") {
                                Some(Value::Array(a)) => a,
                                _ => vec![],
                            };
                            dns.push(Value::String(trimmed.to_string()));
                            adapter.insert("dns_servers".to_string(), Value::Array(dns));
                        }
                        "Default Gateway" => {
                            let mut gw: Vec<Value> = match adapter.remove("default_gateways") {
                                Some(Value::Array(a)) => a,
                                _ => vec![],
                            };
                            gw.push(Value::String(trimmed.to_string()));
                            adapter.insert("default_gateways".to_string(), Value::Array(gw));
                        }
                        "Connection-specific DNS Suffix Search List" => {
                            let mut list: Vec<Value> = match adapter
                                .remove("connection_specific_dns_suffix_search_list")
                            {
                                Some(Value::Array(a)) => a,
                                _ => vec![],
                            };
                            list.push(Value::String(trimmed.to_string()));
                            adapter.insert(
                                "connection_specific_dns_suffix_search_list".to_string(),
                                Value::Array(list),
                            );
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // Finalize last adapter
    if let Some(a) = current_adapter {
        adapters.push(a);
    }

    obj.insert(
        "host_name".to_string(),
        host_name.map(Value::String).unwrap_or(Value::Null),
    );
    obj.insert(
        "primary_dns_suffix".to_string(),
        primary_dns_suffix.map(Value::String).unwrap_or(Value::Null),
    );
    obj.insert(
        "node_type".to_string(),
        node_type.map(Value::String).unwrap_or(Value::Null),
    );
    obj.insert(
        "ip_routing_enabled".to_string(),
        ip_routing_enabled.map(Value::Bool).unwrap_or(Value::Null),
    );
    obj.insert(
        "wins_proxy_enabled".to_string(),
        wins_proxy_enabled.map(Value::Bool).unwrap_or(Value::Null),
    );
    obj.insert(
        "dns_suffix_search_list".to_string(),
        Value::Array(global_dns_suffix_list),
    );
    obj.insert(
        "adapters".to_string(),
        Value::Array(adapters.into_iter().map(Value::Object).collect()),
    );
    obj.insert("extras".to_string(), Value::Array(vec![]));

    obj
}

impl Parser for IpconfigParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        Ok(ParseOutput::Object(parse_ipconfig(input)))
    }
}

static INSTANCE: IpconfigParser = IpconfigParser;

inventory::submit! {
    ParserEntry::new(&INSTANCE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jc_rs_core::registry::find_parser;
    use jc_rs_core::types::ParseOutput;

    fn get_fixture(rel_path: &str) -> String {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        let paths = [
            format!("{manifest}/../../tests/fixtures/{rel_path}"),
            format!("{manifest}/../../../tests/fixtures/{rel_path}"),
        ];
        for p in &paths {
            if let Ok(c) = std::fs::read_to_string(p) {
                return c;
            }
        }
        panic!("fixture not found: {rel_path}");
    }

    #[test]
    fn test_ipconfig_registered() {
        assert!(find_parser("ipconfig").is_some());
    }

    #[test]
    fn test_ipconfig_win10() {
        let input = get_fixture("windows/windows-10/ipconfig.out");
        let parser = find_parser("ipconfig").unwrap();
        let result = parser.parse(&input, true).unwrap();
        let obj = match result {
            ParseOutput::Object(o) => o,
            _ => panic!("expected object"),
        };
        assert_eq!(obj["host_name"], serde_json::json!("DESKTOP-WIN10-PRO"));
        assert_eq!(obj["node_type"], serde_json::json!("Hybrid"));
        assert_eq!(obj["ip_routing_enabled"], serde_json::json!(false));
        assert_eq!(obj["wins_proxy_enabled"], serde_json::json!(false));
        let dns_list = obj["dns_suffix_search_list"].as_array().unwrap();
        assert_eq!(dns_list.len(), 2);
        let adapters = obj["adapters"].as_array().unwrap();
        assert_eq!(adapters.len(), 3);
        // First adapter (Tailscale)
        assert_eq!(adapters[0]["name"], serde_json::json!("Tailscale"));
        assert_eq!(adapters[0]["type"], serde_json::json!("Unknown"));
        let ipv6 = adapters[0]["ipv6_addresses"].as_array().unwrap();
        assert_eq!(ipv6.len(), 1);
        assert_eq!(
            ipv6[0]["address"],
            serde_json::json!("fd7a:115c:a1e0:ab12:4843:cd96:6293:47b2")
        );
        // Second adapter (Ethernet 2)
        assert_eq!(adapters[1]["name"], serde_json::json!("Ethernet 2"));
        assert_eq!(adapters[1]["dhcp_enabled"], serde_json::json!(true));
        let ipv4 = adapters[1]["ipv4_addresses"].as_array().unwrap();
        assert_eq!(ipv4.len(), 1);
        assert_eq!(ipv4[0]["address"], serde_json::json!("10.50.13.132"));
        // Lease dates
        assert!(!adapters[1]["lease_obtained"].is_null());
    }

    #[test]
    fn test_ipconfig_xp() {
        let input = get_fixture("windows/windows-xp/ipconfig.out");
        let parser = find_parser("ipconfig").unwrap();
        let result = parser.parse(&input, true).unwrap();
        let obj = match result {
            ParseOutput::Object(o) => o,
            _ => panic!("expected object"),
        };
        assert_eq!(obj["host_name"], serde_json::json!("DESKTOP-PC4"));
        let adapters = obj["adapters"].as_array().unwrap();
        assert_eq!(adapters.len(), 1);
        // XP uses "IP Address" instead of "IPv4 Address"
        let ipv4 = adapters[0]["ipv4_addresses"].as_array().unwrap();
        assert_eq!(ipv4.len(), 1);
        assert_eq!(ipv4[0]["address"], serde_json::json!("192.168.22.135"));
    }
}
