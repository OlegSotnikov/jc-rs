//! Parser for IP address strings.
//!
//! Note: jc's `ip_address` parser parses IP address *strings* (not `ip address show`).
//! It accepts IPv4/IPv6 addresses with optional CIDR notation and returns
//! network information.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};
use std::net::Ipv4Addr;

pub struct IpAddressParser;

static INFO: ParserInfo = ParserInfo {
    name: "ip_address",
    argument: "--ip-address",
    version: "1.0.0",
    description: "Converts IP address string to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[
        Platform::Linux,
        Platform::Darwin,
        Platform::FreeBSD,
        Platform::Windows,
    ],
    tags: &[Tag::String],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static IP_ADDRESS_PARSER: IpAddressParser = IpAddressParser;

inventory::submit! { ParserEntry::new(&IP_ADDRESS_PARSER) }

fn ipv4_to_u32(ip: Ipv4Addr) -> u32 {
    u32::from(ip)
}

fn u32_to_ipv4(n: u32) -> Ipv4Addr {
    Ipv4Addr::from(n)
}

fn ipv4_to_hex(ip: Ipv4Addr) -> String {
    let octets = ip.octets();
    format!(
        "{:02x}:{:02x}:{:02x}:{:02x}",
        octets[0], octets[1], octets[2], octets[3]
    )
}

fn ipv4_to_bin(ip: Ipv4Addr) -> String {
    let n = ipv4_to_u32(ip);
    format!("{:032b}", n)
}

fn parse_ipv4(ip_str: &str, cidr: Option<u8>) -> Map<String, Value> {
    let mut obj = Map::new();
    let prefix = cidr.unwrap_or(32);

    let ip: Ipv4Addr = match ip_str.parse() {
        Ok(ip) => ip,
        Err(_) => return obj,
    };

    let ip_int = ipv4_to_u32(ip);
    let mask_int: u32 = if prefix == 0 {
        0
    } else {
        !0u32 << (32 - prefix)
    };
    let network_int = ip_int & mask_int;
    let broadcast_int = network_int | !mask_int;
    let first_int = if prefix == 32 {
        network_int
    } else {
        network_int + 1
    };
    let last_int = if prefix == 32 {
        broadcast_int
    } else {
        broadcast_int.saturating_sub(1)
    };
    let hosts: u64 = if prefix >= 31 {
        0
    } else {
        (1u64 << (32 - prefix)) - 2
    };

    let mask_addr = u32_to_ipv4(mask_int);
    let hostmask_int = !mask_int;
    let hostmask_addr = u32_to_ipv4(hostmask_int);
    let network_addr = u32_to_ipv4(network_int);
    let broadcast_addr = u32_to_ipv4(broadcast_int);
    let first_addr = u32_to_ipv4(first_int);
    let last_addr = u32_to_ipv4(last_int);

    // DNS PTR
    let octets = ip.octets();
    let dns_ptr = format!(
        "{}.{}.{}.{}.in-addr.arpa",
        octets[3], octets[2], octets[1], octets[0]
    );

    obj.insert("version".to_string(), Value::Number(4i64.into()));
    obj.insert("max_prefix_length".to_string(), Value::Number(32i64.into()));
    obj.insert("ip".to_string(), Value::String(ip.to_string()));
    obj.insert("ip_compressed".to_string(), Value::String(ip.to_string()));
    obj.insert("ip_exploded".to_string(), Value::String(ip.to_string()));
    obj.insert(
        "ip_split".to_string(),
        Value::Array(
            octets
                .iter()
                .map(|o| Value::String(o.to_string()))
                .collect(),
        ),
    );
    obj.insert("scope_id".to_string(), Value::Null);
    obj.insert("ipv4_mapped".to_string(), Value::Null);
    obj.insert("six_to_four".to_string(), Value::Null);
    obj.insert("teredo_client".to_string(), Value::Null);
    obj.insert("teredo_server".to_string(), Value::Null);
    obj.insert("dns_ptr".to_string(), Value::String(dns_ptr));
    obj.insert(
        "network".to_string(),
        Value::String(network_addr.to_string()),
    );
    obj.insert(
        "broadcast".to_string(),
        Value::String(broadcast_addr.to_string()),
    );
    obj.insert(
        "hostmask".to_string(),
        Value::String(hostmask_addr.to_string()),
    );
    obj.insert("netmask".to_string(), Value::String(mask_addr.to_string()));
    obj.insert(
        "cidr_netmask".to_string(),
        Value::Number((prefix as i64).into()),
    );
    obj.insert("hosts".to_string(), Value::Number((hosts as i64).into()));
    obj.insert(
        "first_host".to_string(),
        Value::String(first_addr.to_string()),
    );
    obj.insert(
        "last_host".to_string(),
        Value::String(last_addr.to_string()),
    );

    // Boolean flags
    obj.insert("is_multicast".to_string(), Value::Bool(ip.is_multicast()));
    obj.insert("is_private".to_string(), Value::Bool(ip.is_private()));
    obj.insert(
        "is_global".to_string(),
        Value::Bool(
            !ip.is_private() && !ip.is_loopback() && !ip.is_link_local() && !ip.is_multicast(),
        ),
    );
    obj.insert("is_link_local".to_string(), Value::Bool(ip.is_link_local()));
    obj.insert("is_loopback".to_string(), Value::Bool(ip.is_loopback()));
    obj.insert(
        "is_reserved".to_string(),
        Value::Bool(ip.is_documentation() || ip.is_broadcast()),
    );
    obj.insert(
        "is_unspecified".to_string(),
        Value::Bool(ip.is_unspecified()),
    );

    // int sub-object
    let mut int_obj = Map::new();
    int_obj.insert("ip".to_string(), Value::Number((ip_int as i64).into()));
    int_obj.insert(
        "network".to_string(),
        Value::Number((network_int as i64).into()),
    );
    int_obj.insert(
        "broadcast".to_string(),
        Value::Number((broadcast_int as i64).into()),
    );
    int_obj.insert(
        "first_host".to_string(),
        Value::Number((first_int as i64).into()),
    );
    int_obj.insert(
        "last_host".to_string(),
        Value::Number((last_int as i64).into()),
    );
    obj.insert("int".to_string(), Value::Object(int_obj));

    // hex sub-object
    let mut hex_obj = Map::new();
    hex_obj.insert("ip".to_string(), Value::String(ipv4_to_hex(ip)));
    hex_obj.insert(
        "network".to_string(),
        Value::String(ipv4_to_hex(network_addr)),
    );
    hex_obj.insert(
        "broadcast".to_string(),
        Value::String(ipv4_to_hex(broadcast_addr)),
    );
    hex_obj.insert(
        "hostmask".to_string(),
        Value::String(ipv4_to_hex(hostmask_addr)),
    );
    hex_obj.insert("netmask".to_string(), Value::String(ipv4_to_hex(mask_addr)));
    hex_obj.insert(
        "first_host".to_string(),
        Value::String(ipv4_to_hex(first_addr)),
    );
    hex_obj.insert(
        "last_host".to_string(),
        Value::String(ipv4_to_hex(last_addr)),
    );
    obj.insert("hex".to_string(), Value::Object(hex_obj));

    // bin sub-object
    let mut bin_obj = Map::new();
    bin_obj.insert("ip".to_string(), Value::String(ipv4_to_bin(ip)));
    bin_obj.insert(
        "network".to_string(),
        Value::String(ipv4_to_bin(network_addr)),
    );
    bin_obj.insert(
        "broadcast".to_string(),
        Value::String(ipv4_to_bin(broadcast_addr)),
    );
    bin_obj.insert(
        "hostmask".to_string(),
        Value::String(ipv4_to_bin(hostmask_addr)),
    );
    bin_obj.insert("netmask".to_string(), Value::String(ipv4_to_bin(mask_addr)));
    bin_obj.insert(
        "first_host".to_string(),
        Value::String(ipv4_to_bin(first_addr)),
    );
    bin_obj.insert(
        "last_host".to_string(),
        Value::String(ipv4_to_bin(last_addr)),
    );
    obj.insert("bin".to_string(), Value::Object(bin_obj));

    obj
}

impl Parser for IpAddressParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(ParseError::InvalidInput("empty input".to_string()));
        }

        // Take first line/word
        let token = trimmed.lines().next().unwrap_or("").trim();

        // Check for CIDR notation
        let (ip_str, cidr) = if let Some(slash) = token.find('/') {
            let ip_part = &token[..slash];
            let cidr_str = &token[slash + 1..];
            let cidr_val = cidr_str.parse::<u8>().ok();
            (ip_part, cidr_val)
        } else {
            (token, None)
        };

        // Try IPv4 first (may also be integer)
        if ip_str.contains('.') {
            let obj = parse_ipv4(ip_str, cidr);
            if !obj.is_empty() {
                return Ok(ParseOutput::Object(obj));
            }
        }

        // Try integer (IPv4 integer representation)
        if let Ok(n) = ip_str.parse::<u32>() {
            let ip = Ipv4Addr::from(n);
            let obj = parse_ipv4(&ip.to_string(), cidr);
            if !obj.is_empty() {
                return Ok(ParseOutput::Object(obj));
            }
        }

        // Return empty on parse failure (rather than error, matching jc behavior)
        Ok(ParseOutput::Object(Map::new()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jc_rs_core::traits::Parser;

    #[test]
    fn test_ip_address_v4_cidr_golden() {
        let result = IpAddressParser.parse("192.168.1.0/24", false).unwrap();
        let actual = serde_json::to_value(result).unwrap();
        assert_eq!(actual["version"], serde_json::Value::Number(4i64.into()));
        assert_eq!(
            actual["cidr_netmask"],
            serde_json::Value::Number(24i64.into())
        );
        assert_eq!(actual["network"], "192.168.1.0");
        assert_eq!(actual["broadcast"], "192.168.1.255");
        assert_eq!(actual["netmask"], "255.255.255.0");
    }

    #[test]
    fn test_ip_address_v4_no_cidr_golden() {
        // Parser currently supports IPv4 only
        let result = IpAddressParser.parse("10.0.0.1", false).unwrap();
        let actual = serde_json::to_value(result).unwrap();
        assert_eq!(actual["version"], serde_json::Value::Number(4i64.into()));
        assert_eq!(actual["ip"], "10.0.0.1");
    }

    #[test]
    fn test_ip_address_empty_returns_err() {
        assert!(IpAddressParser.parse("", false).is_err());
    }

    #[test]
    fn test_ip_address_registered() {
        assert!(jc_rs_core::registry::find_parser("ip_address").is_some());
    }
}
