//! Parser for `ping` and `ping6` command output.
//!
//! Supports Linux and BSD/macOS ping output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use regex::Regex;
use serde_json::{Map, Value};
use std::sync::LazyLock;

pub struct PingParser;

static INFO: ParserInfo = ParserInfo {
    name: "ping",
    argument: "--ping",
    version: "1.11.0",
    description: "Converts `ping` and `ping6` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux, Platform::Darwin, Platform::FreeBSD],
    tags: &[Tag::Command],
    magic_commands: &["ping", "ping6"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static PING_PARSER: PingParser = PingParser;

inventory::submit! { ParserEntry::new(&PING_PARSER) }

static RTT_4_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"rtt min/avg/max/mdev\s*=\s*([\d.]+)/([\d.]+)/([\d.]+)/([\d.]+)\s*ms")
        .expect("valid rtt pattern")
});

static RTT_3_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"round-trip min/avg/max\s*=\s*([\d.]+)/([\d.]+)/([\d.]+)\s*ms")
        .expect("valid round-trip pattern")
});

fn str_to_int(s: &str) -> Value {
    s.parse::<i64>()
        .map(|n| Value::Number(n.into()))
        .unwrap_or(Value::Null)
}

fn str_to_float(s: &str) -> Value {
    s.parse::<f64>()
        .map(|n| {
            serde_json::Number::from_f64(n)
                .map(Value::Number)
                .unwrap_or(Value::Null)
        })
        .unwrap_or(Value::Null)
}

fn is_linux(input: &str) -> bool {
    // Linux ping has "time Nms" in the stats lines
    let lines: Vec<&str> = input.lines().collect();
    // Check last 3 lines for linux-style " time "
    for line in lines.iter().rev().take(4) {
        if line.contains(" time ") && line.contains("ms") {
            return true;
        }
    }
    false
}

fn parse_linux(input: &str) -> Map<String, Value> {
    let mut obj = Map::new();
    let mut responses: Vec<Value> = Vec::new();
    let mut footer = false;
    let mut pattern: Option<String> = None;
    let mut ipv4 = true;
    let mut has_hostname = false;

    let mut lines: Vec<&str> = input.lines().collect();

    // Check for PATTERN line
    if lines
        .first()
        .map(|l| l.starts_with("PATTERN:"))
        .unwrap_or(false)
    {
        let pat_line = lines.remove(0);
        pattern = Some(
            pat_line
                .split_once(':')
                .map(|x| x.1)
                .unwrap_or("")
                .trim()
                .to_string(),
        );
    }

    // Skip to PING line
    while !lines.is_empty() && !lines[0].starts_with("PING ") {
        lines.remove(0);
    }

    for line in lines.iter().filter(|l| !l.is_empty()) {
        if line.starts_with("PING ") {
            ipv4 = line.contains("bytes of data");

            // Detect missing hostname: "PING  (IP)" has double space at position 5
            // Normalize by inserting "nohost" so positions are consistent with hostname case
            let normalized_line;
            let line = if line.starts_with("PING  ") {
                normalized_line = format!("PING nohost{}", &line[5..]);
                normalized_line.as_str()
            } else {
                line
            };

            has_hostname = if ipv4 {
                !line[5..].starts_with(|c: char| c.is_ascii_digit())
            } else {
                line.contains(" (")
            };

            let cleaned = line.replace(['(', ')'], " ");
            let parts: Vec<&str> = cleaned.split_whitespace().collect();

            // After replacing ( and ) with spaces:
            // With hostname + from: PING host IP from SRC IF: DATA TOTAL bytes
            //                       0    1    2   3    4   5    6    7
            // Without hostname + from: PING IP IP from SRC IF: DATA TOTAL bytes
            //                          0    1  2  3    4   5    6    7
            // With hostname, no from: PING host IP DATA TOTAL bytes
            //                         0    1    2  3    4
            // Without hostname, no from: PING IP IP DATA TOTAL bytes
            //                            0    1  2  3    4
            let (dst_ip_idx, data_bytes_idx) = if line.contains("from") {
                (2, 6)
            } else {
                (2, 3)
            };

            let dest_ip = parts
                .get(dst_ip_idx)
                .copied()
                .unwrap_or("")
                .trim_start_matches('(')
                .trim_end_matches(')')
                .to_string();
            let data_bytes = parts.get(data_bytes_idx).copied().unwrap_or("0");

            obj.insert("destination_ip".to_string(), Value::String(dest_ip));
            obj.insert("data_bytes".to_string(), str_to_int(data_bytes));
            obj.insert(
                "pattern".to_string(),
                pattern
                    .as_ref()
                    .map(|p| Value::String(p.clone()))
                    .unwrap_or(Value::Null),
            );
            continue;
        }

        if line.starts_with("---") {
            footer = true;
            // Skip destination if double-space after --- (missing hostname case, jc: if line[4] != ' ')
            if line.len() > 4 && line.as_bytes().get(4) != Some(&b' ') {
                let dest = line.split_whitespace().nth(1).unwrap_or("").to_string();
                obj.insert("destination".to_string(), Value::String(dest));
            }
            continue;
        }

        if footer {
            if obj.get("duplicates").is_none() {
                obj.insert("duplicates".to_string(), Value::Number(0i64.into()));
            }

            // packets transmitted / received
            if let Some(m) = extract_re(r"(\d+) packets transmitted", line) {
                obj.insert("packets_transmitted".to_string(), str_to_int(&m));
            }
            if let Some(m) = extract_re(r"(\d+) received,", line) {
                obj.insert("packets_received".to_string(), str_to_int(&m));
            }
            if let Some(m) = extract_re(r"\+(\d+) duplicates", line) {
                obj.insert("duplicates".to_string(), str_to_int(&m));
            }
            if let Some(m) = extract_re(r"\+(\d+) errors", line) {
                obj.insert("errors".to_string(), str_to_int(&m));
            }
            if let Some(m) = extract_re(r"([\d.]+)% packet loss", line) {
                obj.insert("packet_loss_percent".to_string(), str_to_float(&m));
            }
            // Extract total time: "time 19125ms" or "time 19125 ms"
            if let Some(m) = extract_re(r"time\s+([\d.]+)\s*ms", line) {
                obj.insert("time_ms".to_string(), str_to_float(&m));
            }
            // Try 4-part rtt (Linux): rtt min/avg/max/mdev = ...
            if let Some(caps) = RTT_4_RE.captures(line) {
                obj.insert(
                    "round_trip_ms_min".to_string(),
                    str_to_float(caps.get(1).map_or("", |m| m.as_str())),
                );
                obj.insert(
                    "round_trip_ms_avg".to_string(),
                    str_to_float(caps.get(2).map_or("", |m| m.as_str())),
                );
                obj.insert(
                    "round_trip_ms_max".to_string(),
                    str_to_float(caps.get(3).map_or("", |m| m.as_str())),
                );
                obj.insert(
                    "round_trip_ms_stddev".to_string(),
                    str_to_float(caps.get(4).map_or("", |m| m.as_str())),
                );
            }
            // Try 3-part round-trip (Alpine/BusyBox): round-trip min/avg/max = ...
            if !obj.contains_key("round_trip_ms_min")
                && let Some(caps) = RTT_3_RE.captures(line)
            {
                obj.insert(
                    "round_trip_ms_min".to_string(),
                    str_to_float(caps.get(1).map_or("", |m| m.as_str())),
                );
                obj.insert(
                    "round_trip_ms_avg".to_string(),
                    str_to_float(caps.get(2).map_or("", |m| m.as_str())),
                );
                obj.insert(
                    "round_trip_ms_max".to_string(),
                    str_to_float(caps.get(3).map_or("", |m| m.as_str())),
                );
                obj.insert("round_trip_ms_stddev".to_string(), Value::Null);
            }
            continue;
        }

        // Response lines
        if line.contains("no answer yet for icmp_seq=") {
            let has_ts = line.starts_with('[');
            let iseq_pos = if has_ts { 6 } else { 5 };
            let icmp_seq = line
                .replace('=', " ")
                .split_whitespace()
                .nth(iseq_pos)
                .unwrap_or("")
                .to_string();
            let mut resp = Map::new();
            resp.insert("type".to_string(), Value::String("timeout".to_string()));
            if has_ts {
                let ts = line
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .trim_start_matches('[')
                    .trim_end_matches(']');
                resp.insert("timestamp".to_string(), str_to_float(ts));
            } else {
                resp.insert("timestamp".to_string(), Value::Null);
            }
            resp.insert("icmp_seq".to_string(), str_to_int(&icmp_seq));
            responses.push(Value::Object(resp));
            continue;
        }

        if line.contains(" bytes from ") {
            // If no icmp_seq= found, this is an unparsable line (matches jc's except Exception behavior)
            if !line.contains("icmp_seq=") {
                let mut resp = Map::new();
                resp.insert(
                    "type".to_string(),
                    Value::String("unparsable_line".to_string()),
                );
                resp.insert("unparsed_line".to_string(), Value::String(line.to_string()));
                responses.push(Value::Object(resp));
                continue;
            }
            let cleaned = line.replace(['(', ')', '='], " ");
            let has_ts = cleaned.starts_with('[');

            let (bts, rip, iseq, t2l, tms) = if ipv4 && !has_hostname {
                (0, 3, 5, 7, 9)
            } else if ipv4 && has_hostname {
                (0, 4, 7, 9, 11)
            } else if !ipv4 && !has_hostname {
                (0, 3, 5, 7, 9)
            } else {
                (0, 4, 7, 9, 11)
            };

            let (bts, rip, iseq, t2l, tms) = if has_ts {
                (bts + 1, rip + 1, iseq + 1, t2l + 1, tms + 1)
            } else {
                (bts, rip, iseq, t2l, tms)
            };

            let parts: Vec<&str> = cleaned.split_whitespace().collect();
            let mut resp = Map::new();
            resp.insert("type".to_string(), Value::String("reply".to_string()));

            if has_ts {
                let ts = parts
                    .first()
                    .map(|s| s.trim_start_matches('[').trim_end_matches(']'))
                    .unwrap_or("");
                resp.insert("timestamp".to_string(), str_to_float(ts));
            } else {
                resp.insert("timestamp".to_string(), Value::Null);
            }

            resp.insert(
                "bytes".to_string(),
                str_to_int(parts.get(bts).copied().unwrap_or("")),
            );
            resp.insert(
                "response_ip".to_string(),
                Value::String(
                    parts
                        .get(rip)
                        .copied()
                        .unwrap_or("")
                        .trim_end_matches(':')
                        .to_string(),
                ),
            );
            resp.insert(
                "icmp_seq".to_string(),
                str_to_int(parts.get(iseq).copied().unwrap_or("")),
            );
            resp.insert(
                "ttl".to_string(),
                str_to_int(parts.get(t2l).copied().unwrap_or("")),
            );
            resp.insert(
                "time_ms".to_string(),
                str_to_float(parts.get(tms).copied().unwrap_or("")),
            );
            resp.insert("duplicate".to_string(), Value::Bool(line.contains("DUP!")));

            responses.push(Value::Object(resp));
        }
    }

    obj.insert("responses".to_string(), Value::Array(responses));
    obj
}

/// Map BSD ping error messages to type codes (matches jc's _error_type)
fn bsd_error_type(line: &str) -> Option<&'static str> {
    let type_map: &[(&str, &str)] = &[
        ("Destination Net Unreachable", "destination_net_unreachable"),
        (
            "Destination Host Unreachable",
            "destination_host_unreachable",
        ),
        (
            "Destination Protocol Unreachable",
            "destination_protocol_unreachable",
        ),
        (
            "Destination Port Unreachable",
            "destination_port_unreachable",
        ),
        ("Frag needed and DF set", "frag_needed_and_df_set"),
        ("Source Route Failed", "source_route_failed"),
        ("Destination Net Unknown", "destination_net_unknown"),
        ("Destination Host Unknown", "destination_host_unknown"),
        ("Source Host Isolated", "source_host_isolated"),
        ("Destination Net Prohibited", "destination_net_prohibited"),
        ("Destination Host Prohibited", "destination_host_prohibited"),
        (
            "Destination Net Unreachable for Type of Service",
            "destination_net_unreachable_for_type_of_service",
        ),
        (
            "Destination Host Unreachable for Type of Service",
            "destination_host_unreachable_for_type_of_service",
        ),
        ("Packet filtered", "packet_filtered"),
        ("Precedence Violation", "precedence_violation"),
        ("Precedence Cutoff", "precedence_cutoff"),
        ("Dest Unreachable, Bad Code", "dest_unreachable_bad_code"),
        ("Redirect Network", "redirect_network"),
        ("Redirect Host", "redirect_host"),
        (
            "Redirect Type of Service and Network",
            "redirect_type_of_service_and_network",
        ),
        ("Redirect, Bad Code", "redirect_bad_code"),
        ("Time to live exceeded", "time_to_live_exceeded"),
        (
            "Frag reassembly time exceeded",
            "frag_reassembly_time_exceeded",
        ),
        ("Time exceeded, Bad Code", "time_exceeded_bad_code"),
    ];
    for (pattern, code) in type_map {
        if line.contains(pattern) {
            return Some(code);
        }
    }
    None
}

fn parse_bsd(input: &str) -> Map<String, Value> {
    let mut obj = Map::new();
    let mut raw_responses: Vec<Map<String, Value>> = Vec::new();
    let mut footer = false;
    let mut pattern: Option<String> = None;
    let mut ping_error = false;
    let mut pending_resp: Option<Map<String, Value>> = None;

    let lines: Vec<&str> = input.lines().collect();

    for line in lines.iter().filter(|l| !l.trim().is_empty()) {
        if line.starts_with("PATTERN:") {
            pattern = Some(
                line.split_once(':')
                    .map(|x| x.1)
                    .unwrap_or("")
                    .trim()
                    .to_string(),
            );
            continue;
        }

        if line.starts_with("PING ") && !line.starts_with("PING6(") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            let dest_ip = parts
                .get(2)
                .copied()
                .unwrap_or("")
                .trim_start_matches('(')
                .trim_end_matches(':')
                .trim_end_matches(')')
                .to_string();
            let data_bytes = parts.get(3).copied().unwrap_or("0");
            obj.insert("destination_ip".to_string(), Value::String(dest_ip));
            obj.insert("data_bytes".to_string(), str_to_int(data_bytes));
            obj.insert(
                "pattern".to_string(),
                pattern
                    .as_ref()
                    .map(|p| Value::String(p.clone()))
                    .unwrap_or(Value::Null),
            );
            continue;
        }

        if line.starts_with("PING6(") {
            let cleaned = line.replace(['(', ')', '='], " ");
            let parts: Vec<&str> = cleaned.split_whitespace().collect();
            obj.insert(
                "source_ip".to_string(),
                Value::String(parts.get(4).copied().unwrap_or("").to_string()),
            );
            obj.insert(
                "destination_ip".to_string(),
                Value::String(parts.get(6).copied().unwrap_or("").to_string()),
            );
            obj.insert(
                "data_bytes".to_string(),
                str_to_int(parts.get(1).copied().unwrap_or("")),
            );
            obj.insert(
                "pattern".to_string(),
                pattern
                    .as_ref()
                    .map(|p| Value::String(p.clone()))
                    .unwrap_or(Value::Null),
            );
            continue;
        }

        if line.starts_with("---") {
            footer = true;
            let dest = line.split_whitespace().nth(1).unwrap_or("").to_string();
            obj.insert("destination".to_string(), Value::String(dest));
            continue;
        }

        if footer {
            if line.contains("packets transmitted") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if line.contains(" duplicates,") {
                    obj.insert(
                        "packets_transmitted".to_string(),
                        str_to_int(parts.first().copied().unwrap_or("")),
                    );
                    obj.insert(
                        "packets_received".to_string(),
                        str_to_int(parts.get(3).copied().unwrap_or("")),
                    );
                    let loss = parts.get(8).copied().unwrap_or("0%").trim_end_matches('%');
                    obj.insert("packet_loss_percent".to_string(), str_to_float(loss));
                    let dups = parts.get(6).copied().unwrap_or("0").trim_start_matches('+');
                    obj.insert("duplicates".to_string(), str_to_int(dups));
                } else {
                    obj.insert(
                        "packets_transmitted".to_string(),
                        str_to_int(parts.first().copied().unwrap_or("")),
                    );
                    obj.insert(
                        "packets_received".to_string(),
                        str_to_int(parts.get(3).copied().unwrap_or("")),
                    );
                    let loss = parts.get(6).copied().unwrap_or("0%").trim_end_matches('%');
                    obj.insert("packet_loss_percent".to_string(), str_to_float(loss));
                    obj.insert("duplicates".to_string(), Value::Number(0i64.into()));
                }
                continue;
            }

            // round trip line: "round-trip min/avg/max/stddev = 1.234/5.678/9.012/3.456 ms"
            if line.contains('/')
                && let Some(eq_pos) = line.find('=')
            {
                let after = line[eq_pos + 1..].trim();
                let ms = after.trim_end_matches(" ms").trim_end_matches(" ms\n");
                let parts: Vec<&str> = ms.split('/').collect();
                if parts.len() >= 3 {
                    obj.insert(
                        "round_trip_ms_min".to_string(),
                        str_to_float(parts[0].trim()),
                    );
                    obj.insert(
                        "round_trip_ms_avg".to_string(),
                        str_to_float(parts[1].trim()),
                    );
                    obj.insert(
                        "round_trip_ms_max".to_string(),
                        str_to_float(parts[2].trim()),
                    );
                    if parts.len() >= 4 {
                        obj.insert(
                            "round_trip_ms_stddev".to_string(),
                            str_to_float(parts[3].trim()),
                        );
                    } else {
                        obj.insert("round_trip_ms_stddev".to_string(), Value::Null);
                    }
                }
            }
            continue;
        }

        // When in ping_error state: skip Vr header, parse data line
        if ping_error {
            if line.starts_with("Vr ") {
                continue;
            }
            // This is the data line: parse hex fields
            if let Some(mut resp) = pending_resp.take() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                let parse_hex = |s: &str| i64::from_str_radix(s, 16).unwrap_or(0);
                if parts.len() >= 12 {
                    resp.insert("vr".to_string(), Value::Number(parse_hex(parts[0]).into()));
                    resp.insert("hl".to_string(), Value::Number(parse_hex(parts[1]).into()));
                    resp.insert("tos".to_string(), Value::Number(parse_hex(parts[2]).into()));
                    resp.insert("len".to_string(), Value::Number(parse_hex(parts[3]).into()));
                    resp.insert("id".to_string(), Value::Number(parse_hex(parts[4]).into()));
                    resp.insert("flg".to_string(), Value::Number(parse_hex(parts[5]).into()));
                    resp.insert("off".to_string(), Value::Number(parse_hex(parts[6]).into()));
                    resp.insert("ttl".to_string(), Value::Number(parse_hex(parts[7]).into()));
                    resp.insert("pro".to_string(), Value::Number(parse_hex(parts[8]).into()));
                    resp.insert("cks".to_string(), Value::Number(parse_hex(parts[9]).into()));
                    resp.insert("src".to_string(), Value::String(parts[10].to_string()));
                    resp.insert("dst".to_string(), Value::String(parts[11].to_string()));
                }
                raw_responses.push(resp);
            }
            ping_error = false;
            continue;
        }

        // Request timeout
        if line.starts_with("Request timeout for ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            let icmp_seq = parts.get(4).copied().unwrap_or("").to_string();
            let mut resp = Map::new();
            resp.insert("type".to_string(), Value::String("timeout".to_string()));
            resp.insert("icmp_seq".to_string(), str_to_int(&icmp_seq));
            raw_responses.push(resp);
            continue;
        }

        // Response lines with " bytes from "
        if line.contains(" bytes from ") {
            // Check for known error types first (matches jc's _error_type check)
            if let Some(err) = bsd_error_type(line) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                let mut resp = Map::new();
                resp.insert("type".to_string(), Value::String(err.to_string()));
                // bytes at index 0, response_ip at index 4 (hostname present)
                if let Some(b) = parts.first() {
                    resp.insert("bytes".to_string(), str_to_int(b));
                }
                if let Some(ip) = parts.get(4) {
                    // "(192.168.1.220):" → strip ':' first, then parens
                    let ip = ip
                        .trim_end_matches(':')
                        .trim_start_matches('(')
                        .trim_end_matches(')');
                    resp.insert("response_ip".to_string(), Value::String(ip.to_string()));
                }
                pending_resp = Some(resp);
                ping_error = true;
                continue;
            }

            // Normal reply or unparsable
            // Replace ':' and '=' with spaces (matching jc)
            let modified = line.replace([':', '='], " ");
            let parts: Vec<&str> = modified.split_whitespace().collect();

            // Try to parse as normal reply (needs at least 10 fields)
            if parts.len() >= 10 {
                let icmp_seq = parts.get(5).copied().unwrap_or("").to_string();
                let mut resp = Map::new();
                resp.insert("type".to_string(), Value::String("reply".to_string()));
                resp.insert(
                    "bytes".to_string(),
                    str_to_int(parts.first().copied().unwrap_or("")),
                );
                resp.insert(
                    "response_ip".to_string(),
                    Value::String(parts.get(3).copied().unwrap_or("").to_string()),
                );
                resp.insert("icmp_seq".to_string(), str_to_int(&icmp_seq));
                resp.insert(
                    "ttl".to_string(),
                    str_to_int(parts.get(7).copied().unwrap_or("")),
                );
                resp.insert(
                    "time_ms".to_string(),
                    str_to_float(parts.get(9).copied().unwrap_or("")),
                );
                raw_responses.push(resp);
            } else {
                // Not enough fields → unparsable_line (line already has ':' replaced)
                let mut resp = Map::new();
                resp.insert(
                    "type".to_string(),
                    Value::String("unparsable_line".to_string()),
                );
                resp.insert("unparsed_line".to_string(), Value::String(modified));
                raw_responses.push(resp);
            }
        }
    }

    // Post-process: add duplicate field to all responses with icmp_seq (matches jc dedup logic)
    let mut seq_list: Vec<String> = Vec::new();
    let responses: Vec<Value> = raw_responses
        .into_iter()
        .map(|mut resp| {
            if resp.contains_key("icmp_seq") {
                let seq = resp
                    .get("icmp_seq")
                    .and_then(|v| v.as_i64())
                    .map(|n| n.to_string())
                    .unwrap_or_default();
                let is_dup = seq_list.contains(&seq);
                resp.insert("duplicate".to_string(), Value::Bool(is_dup));
                seq_list.push(seq);
            }
            Value::Object(resp)
        })
        .collect();

    obj.insert("responses".to_string(), Value::Array(responses));
    obj
}

fn extract_re(pattern: &str, line: &str) -> Option<String> {
    Regex::new(pattern)
        .ok()?
        .captures(line)?
        .get(1)
        .map(|m| m.as_str().to_string())
}

impl Parser for PingParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Object(Map::new()));
        }

        let obj = if is_linux(input) {
            parse_linux(input)
        } else {
            parse_bsd(input)
        };

        Ok(ParseOutput::Object(obj))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jc_rs_core::traits::Parser;

    #[test]
    fn test_ping_centos_golden() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/ping-ip-O.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/ping-ip-O.json"
        ))
        .unwrap();
        let result = PingParser.parse(input, false).unwrap();
        let actual = serde_json::to_value(result).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_ping_empty() {
        let result = PingParser.parse("", false).unwrap();
        assert!(matches!(result, ParseOutput::Object(m) if m.is_empty()));
    }

    #[test]
    fn test_ping_registered() {
        assert!(jc_rs_core::registry::find_parser("ping").is_some());
    }
}
