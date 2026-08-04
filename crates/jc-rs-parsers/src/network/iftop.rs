//! Parser for `iftop` command output.
//!
//! Supports iftop text-mode output (`iftop -t -B -s1`).

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct IftopParser;

static INFO: ParserInfo = ParserInfo {
    name: "iftop",
    argument: "--iftop",
    version: "1.1.0",
    description: "Converts `iftop` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &["iftop"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static IFTOP_PARSER: IftopParser = IftopParser;

inventory::submit! { ParserEntry::new(&IFTOP_PARSER) }

impl Parser for IftopParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }
        Ok(ParseOutput::Array(parse_iftop(input)))
    }
}

/// Convert iftop size/rate string to integer (SI units, 1000-based).
///
/// Examples:
///   "448b"  -> 448
///   "208b"  -> 208
///   "4.72Kb" -> 4720
///   "1.99Mb" -> 1990000
///   "112B"   -> 112
///   "1.18KB" -> 1180
///   "508KB"  -> 508000
///   "5.79MB" -> 5790000
///   "0B"     -> 0
fn convert_size(s: &str) -> i64 {
    let s = s.trim();
    if s.is_empty() {
        return 0;
    }

    // Find where numeric part ends
    let num_end = s
        .find(|c: char| !c.is_ascii_digit() && c != '.')
        .unwrap_or(s.len());
    let num_str = &s[..num_end];
    let unit = &s[num_end..];

    let value: f64 = num_str.parse().unwrap_or(0.0);

    // Normalize unit: lowercase first character for comparison
    let unit_lower = unit.to_lowercase();

    let multiplier: f64 = if unit_lower.is_empty() || unit_lower.starts_with('b') {
        // bytes/bits or bare number
        1.0
    } else if unit_lower.starts_with('k') {
        1_000.0
    } else if unit_lower.starts_with('m') {
        1_000_000.0
    } else if unit_lower.starts_with('g') {
        1_000_000_000.0
    } else if unit_lower.starts_with('t') {
        1_000_000_000_000.0
    } else {
        1.0
    };

    (value * multiplier).round() as i64
}

fn parse_iftop(input: &str) -> Vec<Map<String, Value>> {
    let mut raw_output: Vec<Map<String, Value>> = Vec::new();
    let mut interface_item: Map<String, Value> = Map::new();
    let mut clients: Vec<Value> = Vec::new();
    let mut is_previous_line_interface = false;
    let mut saw_already_host_line = false;
    let mut current_client: Map<String, Value> = Map::new();
    let mut current_connections: Vec<Value> = Vec::new();

    for line in input.lines() {
        if line.trim().is_empty() {
            continue;
        }

        if line.starts_with("interface:") {
            let device = line.splitn(2, ':').nth(1).unwrap_or("").trim().to_string();
            interface_item.insert("device".to_string(), Value::String(device));
        } else if line.starts_with("IP address is:") {
            let ip = line.splitn(2, ':').nth(1).unwrap_or("").trim().to_string();
            interface_item.insert("ip_address".to_string(), Value::String(ip));
        } else if line.starts_with("MAC address is:") {
            // "MAC address is: 08:00:27:c0:4a:4f"
            // Split on first ': ' to get everything after "MAC address is:"
            let mac = line
                .splitn(2, "MAC address is:")
                .nth(1)
                .unwrap_or("")
                .trim()
                .to_string();
            interface_item.insert("mac_address".to_string(), Value::String(mac));
        } else if line.starts_with("Listening on") {
            // ignore
        } else if line.starts_with("# Host name (port/service if enabled)") {
            if !saw_already_host_line {
                saw_already_host_line = true;
            } else {
                // Second occurrence: start new interface_item preserving device/ip/mac
                let device = interface_item.get("device").cloned().unwrap_or(Value::Null);
                let ip = interface_item
                    .get("ip_address")
                    .cloned()
                    .unwrap_or(Value::Null);
                let mac = interface_item
                    .get("mac_address")
                    .cloned()
                    .unwrap_or(Value::Null);
                interface_item = Map::new();
                interface_item.insert("device".to_string(), device);
                interface_item.insert("ip_address".to_string(), ip);
                interface_item.insert("mac_address".to_string(), mac);
            }
        } else if line.chars().all(|c| c == '-') {
            // separator line of dashes
        } else if line.contains("=>") && line.contains(':') && !is_previous_line_interface {
            // Send line with port: "   1 host:port => rate rate rate cum"
            if let Some(parsed) = parse_send_line_with_port(line) {
                current_client = Map::new();
                current_connections = Vec::new();
                current_client.insert("index".to_string(), Value::Number(parsed.index.into()));
                let mut conn = Map::new();
                conn.insert(
                    "host_name".to_string(),
                    Value::String(parsed.host_name.clone()),
                );
                conn.insert(
                    "host_port".to_string(),
                    Value::String(parsed.host_port.clone()),
                );
                conn.insert("last_2s".to_string(), Value::Number(parsed.last_2s.into()));
                conn.insert(
                    "last_10s".to_string(),
                    Value::Number(parsed.last_10s.into()),
                );
                conn.insert(
                    "last_40s".to_string(),
                    Value::Number(parsed.last_40s.into()),
                );
                conn.insert(
                    "cumulative".to_string(),
                    Value::Number(parsed.cumulative.into()),
                );
                conn.insert("direction".to_string(), Value::String("send".to_string()));
                current_connections.push(Value::Object(conn));
                is_previous_line_interface = true;
            }
        } else if line.contains("=>") && !line.contains(':') && is_previous_line_interface {
            // Send line without port (no-port mode) — skip (matches Python "should not happen" logic)
            // Actually check: "=>" and not is_previous and not ':' -> should not happen
            // "=>" and is_previous and not ':' -> also skipped
            // Since is_previous_line_interface=true here, this corresponds to
            // "=>" and is_previous and no ':' -> skipped in Python too
        } else if line.contains("=>") && !line.contains(':') && !is_previous_line_interface {
            // Send line without port — also skipped (matches Python "should not happen")
        } else if line.contains("=>") && line.contains(':') && is_previous_line_interface {
            // "=>" and is_previous and ':' -> "should not happen" in Python
        } else if line.contains("<=") && line.contains(':') && is_previous_line_interface {
            // Receive line with port
            if let Some(parsed) = parse_recv_line_with_port(line) {
                let mut conn = Map::new();
                conn.insert(
                    "host_name".to_string(),
                    Value::String(parsed.host_name.clone()),
                );
                conn.insert(
                    "host_port".to_string(),
                    Value::String(parsed.host_port.clone()),
                );
                conn.insert("last_2s".to_string(), Value::Number(parsed.last_2s.into()));
                conn.insert(
                    "last_10s".to_string(),
                    Value::Number(parsed.last_10s.into()),
                );
                conn.insert(
                    "last_40s".to_string(),
                    Value::Number(parsed.last_40s.into()),
                );
                conn.insert(
                    "cumulative".to_string(),
                    Value::Number(parsed.cumulative.into()),
                );
                conn.insert(
                    "direction".to_string(),
                    Value::String("receive".to_string()),
                );
                current_connections.push(Value::Object(conn));
                current_client.insert(
                    "connections".to_string(),
                    Value::Array(current_connections.clone()),
                );
                clients.push(Value::Object(current_client.clone()));
                is_previous_line_interface = false;
            }
        } else if line.contains("<=") && !line.contains(':') && is_previous_line_interface {
            // Receive line no port — skipped
        } else if line.starts_with("Total send rate:") {
            if let Some(rates) = parse_three_rates(line, "Total send rate:") {
                let mut obj = Map::new();
                obj.insert("last_2s".to_string(), Value::Number(rates.0.into()));
                obj.insert("last_10s".to_string(), Value::Number(rates.1.into()));
                obj.insert("last_40s".to_string(), Value::Number(rates.2.into()));
                interface_item.insert("total_send_rate".to_string(), Value::Object(obj));
            }
        } else if line.starts_with("Total receive rate:") {
            if let Some(rates) = parse_three_rates(line, "Total receive rate:") {
                let mut obj = Map::new();
                obj.insert("last_2s".to_string(), Value::Number(rates.0.into()));
                obj.insert("last_10s".to_string(), Value::Number(rates.1.into()));
                obj.insert("last_40s".to_string(), Value::Number(rates.2.into()));
                interface_item.insert("total_receive_rate".to_string(), Value::Object(obj));
            }
        } else if line.starts_with("Total send and receive rate:") {
            if let Some(rates) = parse_three_rates(line, "Total send and receive rate:") {
                let mut obj = Map::new();
                obj.insert("last_2s".to_string(), Value::Number(rates.0.into()));
                obj.insert("last_10s".to_string(), Value::Number(rates.1.into()));
                obj.insert("last_40s".to_string(), Value::Number(rates.2.into()));
                interface_item.insert(
                    "total_send_and_receive_rate".to_string(),
                    Value::Object(obj),
                );
            }
        } else if line.starts_with("Peak rate") {
            if let Some(rates) = parse_peak_rate(line) {
                let mut obj = Map::new();
                obj.insert("last_2s".to_string(), Value::Number(rates.0.into()));
                obj.insert("last_10s".to_string(), Value::Number(rates.1.into()));
                obj.insert("last_40s".to_string(), Value::Number(rates.2.into()));
                interface_item.insert("peak_rate".to_string(), Value::Object(obj));
            }
        } else if line.starts_with("Cumulative") {
            if let Some(rates) = parse_cumulative_rate(line) {
                let mut obj = Map::new();
                obj.insert("last_2s".to_string(), Value::Number(rates.0.into()));
                obj.insert("last_10s".to_string(), Value::Number(rates.1.into()));
                obj.insert("last_40s".to_string(), Value::Number(rates.2.into()));
                interface_item.insert("cumulative_rate".to_string(), Value::Object(obj));
            }
        } else if line.chars().all(|c| c == '=') {
            // End of section: emit current interface_item
            interface_item.insert("clients".to_string(), Value::Array(clients.clone()));
            clients = Vec::new();
            raw_output.push(interface_item.clone());
        }
    }

    raw_output
}

struct ConnLine {
    index: i64,
    host_name: String,
    host_port: String,
    last_2s: i64,
    last_10s: i64,
    last_40s: i64,
    cumulative: i64,
}

struct RecvLine {
    host_name: String,
    host_port: String,
    last_2s: i64,
    last_10s: i64,
    last_40s: i64,
    cumulative: i64,
}

/// Parse a send line with port.
/// Format: "   1 host:port                 =>     val   val   val   val"
fn parse_send_line_with_port(line: &str) -> Option<ConnLine> {
    // Split on "=>"
    let arrow_pos = line.find("=>")?;
    let before = &line[..arrow_pos];
    let after = &line[arrow_pos + 2..];

    // Parse rates from after
    let rates: Vec<&str> = after.split_whitespace().collect();
    if rates.len() < 4 {
        return None;
    }
    let last_2s = convert_size(rates[0]);
    let last_10s = convert_size(rates[1]);
    let last_40s = convert_size(rates[2]);
    let cumulative = convert_size(rates[3]);

    // Parse before: "   1 host:port   "
    let before_trimmed = before.trim();
    // Find index and host:port
    let mut parts = before_trimmed.splitn(2, char::is_whitespace);
    let index_str = parts.next()?;
    let index: i64 = index_str.trim().parse().ok()?;
    let host_port_str = parts.next()?.trim();

    // Split host:port on last colon to handle IPv6 or hostnames with colons
    // iftop uses "hostname:port" format
    let colon_pos = host_port_str.rfind(':')?;
    let host_name = host_port_str[..colon_pos].to_string();
    let host_port = host_port_str[colon_pos + 1..].to_string();

    Some(ConnLine {
        index,
        host_name,
        host_port,
        last_2s,
        last_10s,
        last_40s,
        cumulative,
    })
}

/// Parse a receive line with port.
/// Format: "     host:port                 <=     val   val   val   val"
fn parse_recv_line_with_port(line: &str) -> Option<RecvLine> {
    let arrow_pos = line.find("<=")?;
    let before = &line[..arrow_pos];
    let after = &line[arrow_pos + 2..];

    let rates: Vec<&str> = after.split_whitespace().collect();
    if rates.len() < 4 {
        return None;
    }
    let last_2s = convert_size(rates[0]);
    let last_10s = convert_size(rates[1]);
    let last_40s = convert_size(rates[2]);
    let cumulative = convert_size(rates[3]);

    let host_port_str = before.trim();
    let colon_pos = host_port_str.rfind(':')?;
    let host_name = host_port_str[..colon_pos].to_string();
    let host_port = host_port_str[colon_pos + 1..].to_string();

    Some(RecvLine {
        host_name,
        host_port,
        last_2s,
        last_10s,
        last_40s,
        cumulative,
    })
}

/// Parse three whitespace-separated rate values after a label prefix.
fn parse_three_rates(line: &str, prefix: &str) -> Option<(i64, i64, i64)> {
    let rest = line.strip_prefix(prefix)?.trim();
    let parts: Vec<&str> = rest.split_whitespace().collect();
    if parts.len() < 3 {
        return None;
    }
    Some((
        convert_size(parts[0]),
        convert_size(parts[1]),
        convert_size(parts[2]),
    ))
}

/// Parse "Peak rate (sent/received/total): val val val"
fn parse_peak_rate(line: &str) -> Option<(i64, i64, i64)> {
    let prefix = "Peak rate (sent/received/total):";
    let rest = line.strip_prefix(prefix)?.trim();
    let parts: Vec<&str> = rest.split_whitespace().collect();
    if parts.len() < 3 {
        return None;
    }
    Some((
        convert_size(parts[0]),
        convert_size(parts[1]),
        convert_size(parts[2]),
    ))
}

/// Parse "Cumulative (sent/received/total): val val val"
fn parse_cumulative_rate(line: &str) -> Option<(i64, i64, i64)> {
    let prefix = "Cumulative (sent/received/total):";
    let rest = line.strip_prefix(prefix)?.trim();
    let parts: Vec<&str> = rest.split_whitespace().collect();
    if parts.len() < 3 {
        return None;
    }
    Some((
        convert_size(parts[0]),
        convert_size(parts[1]),
        convert_size(parts[2]),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iftop_n1_golden() {
        let input = include_str!("../../../../tests/fixtures/ubuntu-20.10/iftop-b-n1.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/ubuntu-20.10/iftop-b-n1.json"
        ))
        .unwrap();
        let result = IftopParser.parse(input, false).unwrap();
        let actual = serde_json::to_value(result).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_iftop_n3_golden() {
        let input = include_str!("../../../../tests/fixtures/ubuntu-20.10/iftop-b-n3.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/ubuntu-20.10/iftop-b-n3.json"
        ))
        .unwrap();
        let result = IftopParser.parse(input, false).unwrap();
        let actual = serde_json::to_value(result).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_iftop_noport_golden() {
        let input = include_str!("../../../../tests/fixtures/ubuntu-20.10/iftop-b-n1-noport.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/ubuntu-20.10/iftop-b-n1-noport.json"
        ))
        .unwrap();
        let result = IftopParser.parse(input, false).unwrap();
        let actual = serde_json::to_value(result).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_iftop_empty() {
        let result = IftopParser.parse("", false).unwrap();
        assert!(matches!(result, ParseOutput::Array(v) if v.is_empty()));
    }

    #[test]
    fn test_iftop_registered() {
        assert!(jc_rs_core::registry::find_parser("iftop").is_some());
    }

    #[test]
    fn test_convert_size() {
        assert_eq!(convert_size("448b"), 448);
        assert_eq!(convert_size("208b"), 208);
        assert_eq!(convert_size("4.72Kb"), 4720);
        assert_eq!(convert_size("1.99Mb"), 1990000);
        assert_eq!(convert_size("112B"), 112);
        assert_eq!(convert_size("1.18KB"), 1180);
        assert_eq!(convert_size("508KB"), 508000);
        assert_eq!(convert_size("5.79MB"), 5790000);
        assert_eq!(convert_size("0B"), 0);
    }
}
