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
    author_email: "",
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

fn parse_interface_list(lines: &[&str]) -> Vec<Map<String, Value>> {
    let mut interfaces = Vec::new();
    let mut in_list = false;

    for line in lines {
        if line.starts_with("Interface List") {
            in_list = true;
            continue;
        }
        if in_list {
            // End of interface list
            if line.trim_start_matches('=').is_empty() && !line.is_empty() {
                // not a separator line — parse it
            } else if line.chars().all(|c| c == '=') {
                break;
            }

            let line_str = line.trim_end();
            if line_str.is_empty() {
                continue;
            }

            // Format: "  10...00 0c 29 86 1e 1f ......Intel(R)..."
            // First 5 chars: index (with dots/spaces)
            // Next 25 chars: MAC (hex pairs separated by spaces)
            // Rest: description

            let chars: Vec<char> = line_str.chars().collect();
            if chars.len() < 5 {
                continue;
            }

            // Parse interface index from start
            let idx_part: String = chars[..5.min(chars.len())]
                .iter()
                .filter(|&&c| c.is_ascii_digit() || c == ' ')
                .collect::<String>()
                .trim()
                .to_string();
            // Actually use the raw first segment before dots
            let idx_str: String = line_str
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == ' ')
                .collect::<String>()
                .trim()
                .to_string();

            let interface_index = idx_str.parse::<i64>().unwrap_or(0);

            // Find the MAC address area (after dots, 6 hex bytes separated by spaces)
            // The format uses dots as padding before and after the MAC
            let stripped = line_str.replace(".", " ");
            let parts: Vec<&str> = stripped.split_whitespace().collect();

            // First part is the index
            // MAC might be next 6 hex pairs
            let mac_address = if parts.len() > 1 {
                // Try to find 6 consecutive hex pairs
                let mut mac_parts: Vec<String> = Vec::new();
                let mut mac_found = false;
                let mut remaining_start = 1; // skip index
                for i in 1..parts.len() {
                    if is_hex_pair(parts[i]) {
                        mac_parts.push(parts[i].to_lowercase().to_string());
                        if mac_parts.len() == 6 {
                            mac_found = true;
                            remaining_start = i + 1;
                            break;
                        }
                    } else if !mac_parts.is_empty() {
                        // Interrupted hex sequence
                        mac_parts.clear();
                        remaining_start = i;
                    } else {
                        remaining_start = i;
                    }
                }

                let mac_str = if mac_found {
                    Some(mac_parts.join(":"))
                } else {
                    None
                };
                (mac_str, remaining_start)
            } else {
                (None, 1)
            };

            // Description is everything after the MAC area
            // Use the original line to find description
            let description = extract_description(line_str);

            let mut iface = Map::new();
            iface.insert(
                "interface_index".to_string(),
                Value::Number(interface_index.into()),
            );
            iface.insert(
                "mac_address".to_string(),
                mac_address.0.map(Value::String).unwrap_or(Value::Null),
            );
            iface.insert("description".to_string(), Value::String(description));
            interfaces.push(iface);
        }
    }

    interfaces
}

fn is_hex_pair(s: &str) -> bool {
    s.len() == 2 && s.chars().all(|c| c.is_ascii_hexdigit())
}

fn extract_description(line: &str) -> String {
    // Description appears after the MAC/dot section
    // Find last sequence of dots and take everything after
    let parts: Vec<&str> = line.splitn(2, "......").collect();
    if parts.len() > 1 {
        return parts[1].trim().to_string();
    }
    // Try with fewer dots
    let parts: Vec<&str> = line.splitn(2, "...").collect();
    if parts.len() > 1 {
        return parts[1].trim_start_matches('.').trim().to_string();
    }
    line.trim().to_string()
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
            if let Some(p) = pending.take() {
                if section == "active" {
                    active.push(p);
                }
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

/// Parse interface list from route print output.
/// The format is:
///   INDEX...MAC_BYTES......DESCRIPTION
/// where INDEX is right-padded to 5 chars, MAC is 6 space-separated hex bytes (25 chars), desc is the rest.
fn parse_interface_list_v2(lines: &[&str]) -> Vec<Map<String, Value>> {
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
        if line.trim_matches('=').is_empty() {
            break; // End of section
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Parse: first 5 chars are index (with dots/spaces), next 25 are MAC, rest is description
        // Example: " 10...00 0c 29 86 1e 1f ......Intel(R) PRO/1000 MT Network Connection"
        let line_bytes: Vec<char> = line.chars().collect();
        let len = line_bytes.len();

        // Index: leading digits before first dot
        let idx_end = line_bytes
            .iter()
            .position(|&c| c == '.')
            .unwrap_or(5.min(len));
        let idx_str: String = line_bytes[..idx_end]
            .iter()
            .collect::<String>()
            .trim()
            .to_string();
        let interface_index = idx_str.parse::<i64>().unwrap_or(0);

        // Find description after "......"
        let desc = {
            // Find last run of 3+ dots
            let line_s = line;
            if let Some(pos) = find_dot_separator(line_s) {
                line_s[pos..].trim_start_matches('.').trim().to_string()
            } else {
                // fallback: everything after first space-run after index
                line_s[idx_end..].trim_matches('.').trim().to_string()
            }
        };

        // MAC: look for 6 consecutive hex pairs in the line
        let mac = extract_mac(line);

        let mut iface = Map::new();
        iface.insert(
            "interface_index".to_string(),
            Value::Number(interface_index.into()),
        );
        iface.insert(
            "mac_address".to_string(),
            mac.map(Value::String).unwrap_or(Value::Null),
        );
        iface.insert("description".to_string(), Value::String(desc));
        interfaces.push(iface);
    }

    interfaces
}

fn find_dot_separator(line: &str) -> Option<usize> {
    // Find position of "......" (6+ dots) in line
    let bytes = line.as_bytes();
    let mut dot_count = 0;
    let mut start = 0;
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'.' {
            if dot_count == 0 {
                start = i;
            }
            dot_count += 1;
            if dot_count >= 6 {
                return Some(i + 1); // position after the dots
            }
        } else {
            dot_count = 0;
        }
    }
    None
}

fn extract_mac(line: &str) -> Option<String> {
    // Look for pattern of 6 space-separated 2-char hex values
    let parts: Vec<&str> = line.split_whitespace().collect();
    // Skip the first (index)
    let mut consecutive_hex = Vec::new();
    for part in parts.iter().skip(1) {
        if is_hex_pair(part) {
            consecutive_hex.push(part.to_lowercase());
            if consecutive_hex.len() == 6 {
                // Check it's not all zeros (placeholder)
                let all_zero = consecutive_hex.iter().all(|h| h == "00");
                let special = consecutive_hex.iter().any(|h| h == "e0"); // special adapter
                let mac = consecutive_hex.join(":");
                if mac == "00:00:00:00:00:00" || (all_zero && special) {
                    return None;
                }
                return Some(mac);
            }
        } else {
            consecutive_hex.clear();
        }
    }
    None
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

        let interfaces = parse_interface_list_v2(&lines);
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
