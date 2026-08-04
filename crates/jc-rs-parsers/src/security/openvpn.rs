//! Parser for OpenVPN status log files.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use regex::Regex;
use serde_json::{Map, Value};
use std::sync::LazyLock;

pub struct OpenvpnParser;

static INFO: ParserInfo = ParserInfo {
    name: "openvpn",
    argument: "--openvpn",
    version: "1.0.0",
    description: "Converts openvpn-status.log file to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux, Platform::Darwin, Platform::FreeBSD],
    tags: &[Tag::File],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static OPENVPN_PARSER: OpenvpnParser = OpenvpnParser;

inventory::submit! {
    ParserEntry::new(&OPENVPN_PARSER)
}

/// Split address into (address, prefix, port)
/// Handles: "10.10.10.10:49502", "10.200.0.0/16", "2001:db8::1000/124",
/// "22:1d:63:bf:62:38" (MAC), "10.10.10.10" (no port/prefix)
fn split_addr(addr_str: &str) -> (String, Option<String>, Option<String>) {
    // Check for MAC address pattern
    let mac_re = Regex::new(r"^(?:[0-9a-fA-F]{2}:){5}[0-9a-fA-F]{2}$").unwrap();
    if mac_re.is_match(addr_str) {
        return (addr_str.to_string(), None, None);
    }

    let mut address = addr_str.to_string();
    let mut prefix: Option<String> = None;

    // Try splitting on '/' for prefix
    if let Some(slash_pos) = address.rfind('/') {
        let pref = address[slash_pos + 1..].to_string();
        address = address[..slash_pos].to_string();
        prefix = Some(pref);
    }

    // `1.2.3.4:1194` carries a port; a bare `:` otherwise means IPv6, which
    // never does here.
    if address.contains(':')
        && let Some(caps) = IPV4_PORT_RE.captures(&address)
    {
        return (caps[1].to_string(), prefix, Some(caps[2].to_string()));
    }

    (address, prefix, None)
}

static IPV4_PORT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}):(\d+)$").expect("valid ipv4:port pattern")
});

/// `Thu Jun 18 04:23:03 2015`, read in the local zone the way jc reads it.
fn parse_openvpn_date(s: &str) -> Option<i64> {
    jc_rs_utils::parse_timestamp(s.trim(), &[jc_rs_utils::timestamp::formats::F1000]).naive_epoch
}

impl Parser for OpenvpnParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Object(Map::new()));
        }

        let mut raw_output: Map<String, Value> = Map::new();
        let mut clients: Vec<Value> = Vec::new();
        let mut routing_table: Vec<Value> = Vec::new();
        let mut global_stats: Map<String, Value> = Map::new();
        let mut section = "";
        let mut updated = String::new();

        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if line.starts_with("OpenVPN CLIENT LIST") {
                section = "clients";
                continue;
            }
            if line.starts_with("ROUTING TABLE") {
                section = "routing";
                continue;
            }
            if line.starts_with("GLOBAL STATS") {
                section = "stats";
                continue;
            }
            if line.starts_with("END") {
                break;
            }

            if section == "clients" && line.starts_with("Updated,") {
                updated = line.split_once(',').map(|x| x.1).unwrap_or("").to_string();
                continue;
            }

            if section == "clients" && line.starts_with("Common Name,Real Address,") {
                continue;
            }

            if section == "clients" {
                let parts: Vec<&str> = line.splitn(5, ',').collect();
                if parts.len() < 5 {
                    continue;
                }
                let c_name = parts[0];
                let real_addr_raw = parts[1];
                let r_bytes = parts[2];
                let s_bytes = parts[3];
                let connected = parts[4];

                let (addr, addr_prefix, addr_port) = split_addr(real_addr_raw);

                let mut client_obj: Map<String, Value> = Map::new();
                client_obj.insert("common_name".to_string(), Value::String(c_name.to_string()));
                client_obj.insert("real_address".to_string(), Value::String(addr));
                if let Ok(n) = r_bytes.parse::<i64>() {
                    client_obj.insert("bytes_received".to_string(), Value::Number(n.into()));
                }
                if let Ok(n) = s_bytes.parse::<i64>() {
                    client_obj.insert("bytes_sent".to_string(), Value::Number(n.into()));
                }
                client_obj.insert(
                    "connected_since".to_string(),
                    Value::String(connected.to_string()),
                );
                client_obj.insert("updated".to_string(), Value::String(updated.clone()));

                // prefix and port
                client_obj.insert(
                    "real_address_prefix".to_string(),
                    addr_prefix
                        .as_deref()
                        .and_then(|s| s.parse::<i64>().ok())
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                );
                client_obj.insert(
                    "real_address_port".to_string(),
                    addr_port
                        .as_deref()
                        .and_then(|s| s.parse::<i64>().ok())
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                );

                // Epoch fields
                if let Some(epoch) = parse_openvpn_date(connected) {
                    client_obj.insert(
                        "connected_since_epoch".to_string(),
                        Value::Number(epoch.into()),
                    );
                }
                if let Some(epoch) = parse_openvpn_date(&updated) {
                    client_obj.insert("updated_epoch".to_string(), Value::Number(epoch.into()));
                }

                clients.push(Value::Object(client_obj));
                continue;
            }

            if section == "routing" && line.starts_with("Virtual Address,Common Name,") {
                continue;
            }

            if section == "routing" {
                let parts: Vec<&str> = line.splitn(4, ',').collect();
                if parts.len() < 4 {
                    continue;
                }
                let mut virt_addr = parts[0].to_string();
                let c_name = parts[1];
                let real_addr_raw = parts[2];
                let last_ref = parts[3];

                // fixup: remove trailing "C" from virtual address
                if virt_addr.ends_with('C') {
                    virt_addr.pop();
                }

                let (virt_ip, virt_prefix, virt_port) = split_addr(&virt_addr);
                let (real_addr, real_prefix, real_port) = split_addr(real_addr_raw);

                let mut route_obj: Map<String, Value> = Map::new();
                route_obj.insert("virtual_address".to_string(), Value::String(virt_ip));
                route_obj.insert("common_name".to_string(), Value::String(c_name.to_string()));
                route_obj.insert("real_address".to_string(), Value::String(real_addr));
                route_obj.insert(
                    "last_reference".to_string(),
                    Value::String(last_ref.to_string()),
                );

                route_obj.insert(
                    "virtual_address_prefix".to_string(),
                    virt_prefix
                        .as_deref()
                        .and_then(|s| s.parse::<i64>().ok())
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                );
                route_obj.insert(
                    "virtual_address_port".to_string(),
                    virt_port
                        .as_deref()
                        .and_then(|s| s.parse::<i64>().ok())
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                );
                route_obj.insert(
                    "real_address_prefix".to_string(),
                    real_prefix
                        .as_deref()
                        .and_then(|s| s.parse::<i64>().ok())
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                );
                route_obj.insert(
                    "real_address_port".to_string(),
                    real_port
                        .as_deref()
                        .and_then(|s| s.parse::<i64>().ok())
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                );

                if let Some(epoch) = parse_openvpn_date(last_ref) {
                    route_obj.insert(
                        "last_reference_epoch".to_string(),
                        Value::Number(epoch.into()),
                    );
                }

                routing_table.push(Value::Object(route_obj));
                continue;
            }

            if section == "stats" && line.starts_with("Max bcast/mcast queue length") {
                let val = line
                    .split_once(',')
                    .map(|x| x.1)
                    .unwrap_or("0")
                    .trim()
                    .to_string();
                if let Ok(n) = val.parse::<i64>() {
                    global_stats.insert(
                        "max_bcast_mcast_queue_len".to_string(),
                        Value::Number(n.into()),
                    );
                }
                continue;
            }
        }

        raw_output.insert("clients".to_string(), Value::Array(clients));
        raw_output.insert("routing_table".to_string(), Value::Array(routing_table));
        raw_output.insert("global_stats".to_string(), Value::Object(global_stats));

        Ok(ParseOutput::Object(raw_output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openvpn_basic() {
        let input = r#"OpenVPN CLIENT LIST
Updated,Thu Jun 18 08:12:15 2015
Common Name,Real Address,Bytes Received,Bytes Sent,Connected Since
foo@example.com,10.10.10.10:49502,334948,1973012,Thu Jun 18 04:23:03 2015
ROUTING TABLE
Virtual Address,Common Name,Real Address,Last Ref
192.168.255.118,baz@example.com,10.10.10.10:63414,Thu Jun 18 08:12:09 2015
GLOBAL STATS
Max bcast/mcast queue length,0
END
"#;
        let parser = OpenvpnParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Object(obj) = result {
            assert!(obj.contains_key("clients"));
            assert!(obj.contains_key("routing_table"));
            assert!(obj.contains_key("global_stats"));

            if let Some(Value::Array(clients)) = obj.get("clients") {
                assert_eq!(clients.len(), 1);
                if let Value::Object(c) = &clients[0] {
                    assert_eq!(
                        c.get("common_name"),
                        Some(&Value::String("foo@example.com".to_string()))
                    );
                    assert_eq!(
                        c.get("real_address"),
                        Some(&Value::String("10.10.10.10".to_string()))
                    );
                    assert_eq!(
                        c.get("real_address_port"),
                        Some(&Value::Number(49502i64.into()))
                    );
                }
            }

            if let Some(Value::Object(stats)) = obj.get("global_stats") {
                assert_eq!(
                    stats.get("max_bcast_mcast_queue_len"),
                    Some(&Value::Number(0i64.into()))
                );
            }
        } else {
            panic!("Expected Object");
        }
    }

    #[test]
    fn test_openvpn_empty() {
        let parser = OpenvpnParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Object(obj) = result {
            assert!(obj.is_empty());
        } else {
            panic!("Expected Object");
        }
    }
}
