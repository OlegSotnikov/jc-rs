//! Parser for Windows `route print` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct RoutePrintParser;

static INFO: ParserInfo = ParserInfo {
    name: "route_print",
    argument: "--route-print",
    version: "1.0.0",
    description: "Converts Windows `route print` command output to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Windows],
    tags: &[Tag::Command],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static ROUTE_PRINT_PARSER: RoutePrintParser = RoutePrintParser;
inventory::submit! { ParserEntry::new(&ROUTE_PRINT_PARSER) }

fn str_to_int_opt(s: &str) -> Option<i64> {
    s.trim().parse::<i64>().ok()
}

/// `route print` pads the interface table with dots to fixed columns, and jc
/// reads it by column rather than by token: index in [0,5), MAC in [5,30),
/// description from 30 on. Hunting for "six hex pairs" instead mistook the
/// eight-byte placeholder that virtual adapters report for part of their name.
fn parse_interface_list(lines: &[&str]) -> Vec<Map<String, Value>> {
    let mut interfaces = Vec::new();
    let mut in_list = false;

    for line in lines {
        if line.starts_with("Interface List") {
            in_list = true;
            continue;
        }
        if !in_list {
            continue;
        }
        if line.chars().all(|c| c == '=') && !line.is_empty() {
            break;
        }
        if line.trim().is_empty() {
            continue;
        }

        let chars: Vec<char> = line.trim_end().chars().collect();
        let slice = |from: usize, to: usize| -> String {
            chars
                .get(from..to.min(chars.len()))
                .map(|c| c.iter().collect::<String>())
                .unwrap_or_default()
                .replace('.', "")
                .trim()
                .to_string()
        };

        let interface_index = slice(0, 5).parse::<i64>().unwrap_or(0);
        let mac_field = slice(5, 30);
        let description = slice(30, chars.len());

        // An empty field, or the placeholder virtual adapters report, is null
        // rather than an address.
        let mac_address = if mac_field.is_empty() || mac_field == "00 00 00 00 00 00 00 e0" {
            None
        } else {
            Some(mac_field.replace(' ', ":"))
        };

        let mut iface = Map::with_capacity(3);
        iface.insert(
            "interface_index".to_string(),
            Value::Number(interface_index.into()),
        );
        iface.insert(
            "mac_address".to_string(),
            mac_address.map_or(Value::Null, Value::String),
        );
        iface.insert("description".to_string(), Value::String(description));
        interfaces.push(iface);
    }

    interfaces
}

fn parse_ipv4_route_table(lines: &[&str]) -> (Vec<Map<String, Value>>, Vec<Map<String, Value>>) {
    let mut active: Vec<Map<String, Value>> = Vec::new();
    let mut persistent: Vec<Map<String, Value>> = Vec::new();

    let mut in_ipv4 = false;
    let mut section = ""; // "active" or "persistent"
    let mut skip_header = false;

    for line in lines {
        if line.starts_with("IPv4 Route Table") {
            in_ipv4 = true;
            continue;
        }
        if line.starts_with("IPv6 Route Table") {
            break;
        }
        if !in_ipv4 {
            continue;
        }

        if line.chars().all(|c| c == '=') {
            skip_header = false;
            continue;
        }

        if line.starts_with("Active Routes:") {
            section = "active";
            skip_header = true;
            continue;
        }
        if line.starts_with("Persistent Routes:") {
            section = "persistent";
            skip_header = true;
            continue;
        }

        if skip_header {
            skip_header = false;
            continue; // skip the header line (Network Destination...)
        }

        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed == "None" {
            continue;
        }
        if trimmed.contains("Default Gateway:") {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();

        if section == "active" && parts.len() >= 5 {
            let metric_str = parts[4];
            let (metric, metric_default) = if metric_str == "Default" {
                (Value::Null, Value::Bool(true))
            } else {
                (
                    str_to_int_opt(metric_str)
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                    Value::Bool(false),
                )
            };
            let mut route = Map::new();
            route.insert(
                "network_destination".to_string(),
                Value::String(parts[0].to_string()),
            );
            route.insert("netmask".to_string(), Value::String(parts[1].to_string()));
            route.insert("gateway".to_string(), Value::String(parts[2].to_string()));
            route.insert("interface".to_string(), Value::String(parts[3].to_string()));
            route.insert("metric".to_string(), metric);
            route.insert("metric_set_to_default".to_string(), metric_default);
            active.push(route);
        } else if section == "persistent" && parts.len() >= 4 {
            let metric_str = parts[3];
            let (metric, metric_default) = if metric_str == "Default" {
                (Value::Null, Value::Bool(true))
            } else {
                (
                    str_to_int_opt(metric_str)
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                    Value::Bool(false),
                )
            };
            let mut route = Map::new();
            route.insert(
                "network_address".to_string(),
                Value::String(parts[0].to_string()),
            );
            route.insert("netmask".to_string(), Value::String(parts[1].to_string()));
            route.insert(
                "gateway_address".to_string(),
                Value::String(parts[2].to_string()),
            );
            route.insert("metric".to_string(), metric);
            route.insert("metric_set_to_default".to_string(), metric_default);
            persistent.push(route);
        }
    }

    (active, persistent)
}

fn parse_ipv6_route_table(lines: &[&str]) -> (Vec<Map<String, Value>>, Vec<Map<String, Value>>) {
    let mut active: Vec<Map<String, Value>> = Vec::new();
    let mut persistent: Vec<Map<String, Value>> = Vec::new();

    let mut in_ipv6 = false;
    let mut section = "";
    let mut skip_header = false;
    let mut pending: Option<Map<String, Value>> = None;

    for line in lines {
        if line.starts_with("IPv6 Route Table") {
            in_ipv6 = true;
            continue;
        }
        if !in_ipv6 {
            continue;
        }

        if line.chars().all(|c| c == '=') {
            skip_header = false;
            // Flush pending
            if let Some(p) = pending.take() {
                if section == "active" {
                    active.push(p);
                } else if section == "persistent" {
                    persistent.push(p);
                }
            }
            continue;
        }

        if line.starts_with("Active Routes:") {
            section = "active";
            skip_header = true;
            continue;
        }
        if line.starts_with("Persistent Routes:") {
            // Flush pending
            if let Some(p) = pending.take()
                && section == "active"
            {
                active.push(p);
            }
            section = "persistent";
            skip_header = true;
            continue;
        }

        if skip_header {
            skip_header = false;
            continue;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed == "None" {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.is_empty() {
            continue;
        }

        // Check if this is a continuation line (gateway on separate line)
        // Continuation lines start with whitespace and have just one token
        if line.starts_with("                                    ")
            || (line.starts_with(' ') && parts.len() == 1)
        {
            // This is a gateway continuation
            if let Some(ref mut p) = pending {
                p.insert("gateway".to_string(), Value::String(parts[0].to_string()));
                let entry = pending.take().unwrap();
                if section == "active" {
                    active.push(entry);
                } else if section == "persistent" {
                    persistent.push(entry);
                }
            }
            continue;
        }

        // Flush any pending entry
        if let Some(p) = pending.take() {
            if section == "active" {
                active.push(p);
            } else if section == "persistent" {
                persistent.push(p);
            }
        }

        if parts.len() >= 3 {
            let iface_val = str_to_int_opt(parts[0])
                .map(|n| Value::Number(n.into()))
                .unwrap_or(Value::Null);
            let metric_str = parts[1];
            let (metric, metric_default) = if metric_str == "Default" {
                (Value::Null, Value::Bool(true))
            } else {
                (
                    str_to_int_opt(metric_str)
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                    Value::Bool(false),
                )
            };

            let mut route = Map::new();
            route.insert("interface".to_string(), iface_val);
            route.insert("metric".to_string(), metric);
            route.insert("metric_set_to_default".to_string(), metric_default);
            route.insert(
                "network_destination".to_string(),
                Value::String(parts[2].to_string()),
            );

            if parts.len() >= 4 {
                route.insert("gateway".to_string(), Value::String(parts[3].to_string()));
                if section == "active" {
                    active.push(route);
                } else if section == "persistent" {
                    persistent.push(route);
                }
            } else {
                // Gateway on next line
                pending = Some(route);
            }
        }
    }

    // Flush remaining
    if let Some(p) = pending {
        if section == "active" {
            active.push(p);
        } else if section == "persistent" {
            persistent.push(p);
        }
    }

    (active, persistent)
}

impl Parser for RoutePrintParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Object(Map::new()));
        }

        let lines: Vec<&str> = input.lines().collect();

        let interfaces = parse_interface_list(&lines);
        let (ipv4_active, ipv4_persistent) = parse_ipv4_route_table(&lines);
        let (ipv6_active, ipv6_persistent) = parse_ipv6_route_table(&lines);

        let mut obj = Map::new();

        obj.insert(
            "interface_list".to_string(),
            Value::Array(interfaces.into_iter().map(Value::Object).collect()),
        );

        let mut ipv4_table = Map::new();
        ipv4_table.insert(
            "active_routes".to_string(),
            Value::Array(ipv4_active.into_iter().map(Value::Object).collect()),
        );
        ipv4_table.insert(
            "persistent_routes".to_string(),
            Value::Array(ipv4_persistent.into_iter().map(Value::Object).collect()),
        );
        obj.insert("ipv4_route_table".to_string(), Value::Object(ipv4_table));

        let mut ipv6_table = Map::new();
        ipv6_table.insert(
            "active_routes".to_string(),
            Value::Array(ipv6_active.into_iter().map(Value::Object).collect()),
        );
        ipv6_table.insert(
            "persistent_routes".to_string(),
            Value::Array(ipv6_persistent.into_iter().map(Value::Object).collect()),
        );
        obj.insert("ipv6_route_table".to_string(), Value::Object(ipv6_table));

        Ok(ParseOutput::Object(obj))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jc_rs_core::traits::Parser;

    #[test]
    fn test_route_print_win10_golden() {
        let input = include_str!("../../../../tests/fixtures/windows/windows-10/route_print.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/windows/windows-10/route_print.json"
        ))
        .unwrap();
        let result = RoutePrintParser.parse(input, false).unwrap();
        let actual = serde_json::to_value(result).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_route_print_win2016_golden() {
        let input = include_str!("../../../../tests/fixtures/windows/windows-2016/route_print.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/windows/windows-2016/route_print.json"
        ))
        .unwrap();
        let result = RoutePrintParser.parse(input, false).unwrap();
        let actual = serde_json::to_value(result).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_route_print_empty() {
        let result = RoutePrintParser.parse("", false).unwrap();
        assert!(matches!(result, ParseOutput::Object(m) if m.is_empty()));
    }

    #[test]
    fn test_route_print_registered() {
        assert!(jc_rs_core::registry::find_parser("route_print").is_some());
    }
}
