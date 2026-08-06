//! Streaming parser for `ping` and `ping6` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::{LineParser, Parser, Record, StreamingParser, parse_via_session};
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use regex::Regex;
use serde_json::{Map, Value};

pub struct PingStreamParser;

static INFO: ParserInfo = ParserInfo {
    name: "ping_s",
    argument: "--ping-s",
    version: "1.6.0",
    description: "Streaming parser for `ping` and `ping6` command output",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux, Platform::Darwin, Platform::FreeBSD],
    tags: &[Tag::Command, Tag::Streaming],
    magic_commands: &[],
    streaming: true,
    hidden: false,
    deprecated: false,
};

static PING_STREAM_PARSER: PingStreamParser = PingStreamParser;
inventory::submit! { ParserEntry::new(&PING_STREAM_PARSER) }

fn str_to_int(s: &str) -> Value {
    s.trim()
        .parse::<i64>()
        .map(|n| Value::Number(n.into()))
        .unwrap_or(Value::Null)
}

fn str_to_float(s: &str) -> Value {
    s.trim()
        .parse::<f64>()
        .ok()
        .and_then(serde_json::Number::from_f64)
        .map(Value::Number)
        .unwrap_or(Value::Null)
}

fn contains_ipv6(line: &str) -> bool {
    // Check if line contains an IPv6 address
    let normalized = line.replace(['(', ')', ',', '%'], " ");
    let parts: Vec<String> = normalized
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();
    for part in &parts {
        if part.contains(':') && part.len() > 3 {
            return true;
        }
    }
    false
}

#[derive(Default)]
struct PingState {
    linux: Option<bool>,
    ipv4: bool,
    has_hostname: bool,
    has_source_ip: bool,
    destination_ip: Option<String>,
    sent_bytes: Option<i64>,
    pattern: Option<String>,
    in_footer: bool,
    // Summary accumulation
    packets_transmitted: Option<i64>,
    packets_received: Option<i64>,
    packet_loss_percent: Option<f64>,
    duplicates: Option<i64>,
    errors: Option<i64>,
    corrupted: Option<i64>,
    time_ms: Option<i64>,
    round_trip_min: Option<f64>,
    round_trip_avg: Option<f64>,
    round_trip_max: Option<f64>,
    round_trip_stddev: Option<f64>,
}

/// jc's ICMP error tables, verbatim. Matching is by substring, first hit wins,
/// so the order these are declared in is the order jc declares them.
static ERROR_TYPES_V4: &[(&str, &str)] = &[
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

static ERROR_TYPES_V6: &[(&str, &str)] = &[
    ("Destination unreachable", "destination_unreachable"),
    ("Packet too big", "packet_too_big"),
    ("Time exceeded:", "time_exceeded"),
    ("Parameter problem:", "parameter_problem"),
];

/// A v6 error is refined by a second phrase, so `Destination unreachable: Port
/// unreachable` becomes `destination_unreachable_port_unreachable`.
static ERROR_CODES_V6: &[(&str, &str)] = &[
    ("No route", "no_route"),
    ("Administratively prohibited", "administratively_prohibited"),
    ("Address unreachable", "address_unreachable"),
    ("Port unreachable", "port_unreachable"),
    ("Hop limit", "hop_limit"),
    (
        "Fragment reassembly time exceeded",
        "fragment_reassembly_time_exceeded",
    ),
];

fn error_type(line: &str, ipv4: bool) -> Option<String> {
    if ipv4 {
        return ERROR_TYPES_V4
            .iter()
            .find(|(phrase, _)| line.contains(phrase))
            .map(|(_, code)| (*code).to_string());
    }

    let (_, kind) = ERROR_TYPES_V6
        .iter()
        .find(|(phrase, _)| line.contains(phrase))?;
    match ERROR_CODES_V6
        .iter()
        .find(|(phrase, _)| line.contains(phrase))
    {
        Some((_, code)) => Some(format!("{kind}_{code}")),
        None => Some((*kind).to_string()),
    }
}

fn parse_linux_line(line: &str, state: &mut PingState) -> Option<Map<String, Value>> {
    if line.starts_with("PING ") {
        state.ipv4 = line.contains("bytes of data");
        state.has_source_ip = line.contains("from");

        let mut l = line.to_string();
        if state.ipv4 && line[5..].starts_with(|c: char| !c.is_ascii_digit()) {
            state.has_hostname = true;
            // Insert placeholder hostname
            l = format!("{}nohost{}", &line[..5], &line[5..]);
        } else if state.ipv4 {
            state.has_hostname = false;
        } else {
            state.has_hostname = line.contains(" (");
        }

        let cleaned = l.replace(['(', ')'], " ");
        let parts: Vec<&str> = cleaned.split_whitespace().collect();

        // Field positions after `(` and `)` become spaces. `ping -I <src>` adds
        // `from <ip>` to the banner, which shifts the byte count but not the
        // destination; reading both as shifted put `from` in destination_ip.
        let (dst_ip_idx, bytes_idx) = if state.ipv4 {
            if state.has_source_ip { (2, 6) } else { (2, 3) }
        } else {
            if state.has_source_ip && state.has_hostname {
                (3, 7)
            } else if state.has_source_ip {
                (2, 6)
            } else if state.has_hostname {
                (3, 4)
            } else {
                (2, 3)
            }
        };

        state.destination_ip = parts
            .get(dst_ip_idx)
            .map(|s| s.trim_matches(|c| c == '(' || c == ')').to_string());
        state.sent_bytes = parts.get(bytes_idx).and_then(|s| s.parse::<i64>().ok());
        return None;
    }

    if line.starts_with("---") {
        state.in_footer = true;
        return None;
    }

    if state.in_footer {
        // Parse footer stats
        if let Some(m) = extract_re(r"(\d+) packets transmitted", line) {
            state.packets_transmitted = m.parse::<i64>().ok();
        }
        if let Some(m) = extract_re(r"(\d+) received,", line) {
            state.packets_received = m.parse::<i64>().ok();
        }
        if let Some(m) = extract_re(r"[+](\d+) duplicates", line) {
            state.duplicates = m.parse::<i64>().ok();
        }
        if let Some(m) = extract_re(r"[+](\d+) errors", line) {
            state.errors = m.parse::<i64>().ok();
        }
        if let Some(m) = extract_re(r"[+](\d+) corrupted", line) {
            state.corrupted = m.parse::<i64>().ok();
        }
        if let Some(m) = extract_re(r"([\d.]+)% packet loss", line) {
            state.packet_loss_percent = m.parse::<f64>().ok();
        }
        if let Some(m) = extract_re(r"time (\d+)ms", line) {
            state.time_ms = m.parse::<i64>().ok();
        }
        if let Ok(re) =
            Regex::new(r"rtt min/avg/max/mdev\s*=\s*([\d.]+)/([\d.]+)/([\d.]+)/([\d.]+)\s*ms")
            && let Some(caps) = re.captures(line)
        {
            state.round_trip_min = caps.get(1).and_then(|m| m.as_str().parse().ok());
            state.round_trip_avg = caps.get(2).and_then(|m| m.as_str().parse().ok());
            state.round_trip_max = caps.get(3).and_then(|m| m.as_str().parse().ok());
            state.round_trip_stddev = caps.get(4).and_then(|m| m.as_str().parse().ok());
        }

        // Return summary on each footer line (caller will use the last one)
        let mut obj = Map::new();
        obj.insert("type".to_string(), Value::String("summary".to_string()));
        obj.insert(
            "destination_ip".to_string(),
            state
                .destination_ip
                .as_ref()
                .map(|s| Value::String(s.clone()))
                .unwrap_or(Value::Null),
        );
        obj.insert(
            "sent_bytes".to_string(),
            state
                .sent_bytes
                .map(|n| Value::Number(n.into()))
                .unwrap_or(Value::Null),
        );
        obj.insert(
            "pattern".to_string(),
            state
                .pattern
                .as_ref()
                .map(|s| Value::String(s.clone()))
                .unwrap_or(Value::Null),
        );
        obj.insert(
            "packets_transmitted".to_string(),
            state
                .packets_transmitted
                .map(|n| Value::Number(n.into()))
                .unwrap_or(Value::Null),
        );
        obj.insert(
            "packets_received".to_string(),
            state
                .packets_received
                .map(|n| Value::Number(n.into()))
                .unwrap_or(Value::Null),
        );
        obj.insert(
            "packet_loss_percent".to_string(),
            state
                .packet_loss_percent
                .and_then(serde_json::Number::from_f64)
                .map(Value::Number)
                .unwrap_or(Value::Null),
        );
        obj.insert(
            "duplicates".to_string(),
            state
                .duplicates
                .map(|n| Value::Number(n.into()))
                .unwrap_or(Value::Number(0i64.into())),
        );
        obj.insert(
            "errors".to_string(),
            state
                .errors
                .map(|n| Value::Number(n.into()))
                .unwrap_or(Value::Null),
        );
        obj.insert(
            "corrupted".to_string(),
            state
                .corrupted
                .map(|n| Value::Number(n.into()))
                .unwrap_or(Value::Null),
        );
        // jc emits the total elapsed time as a float, between `corrupted` and
        // the round-trip block. It was parsed into state but never serialised.
        obj.insert(
            "time_ms".to_string(),
            state
                .time_ms
                .and_then(|n| serde_json::Number::from_f64(n as f64))
                .map(Value::Number)
                .unwrap_or(Value::Null),
        );
        obj.insert(
            "round_trip_ms_min".to_string(),
            state
                .round_trip_min
                .and_then(serde_json::Number::from_f64)
                .map(Value::Number)
                .unwrap_or(Value::Null),
        );
        obj.insert(
            "round_trip_ms_avg".to_string(),
            state
                .round_trip_avg
                .and_then(serde_json::Number::from_f64)
                .map(Value::Number)
                .unwrap_or(Value::Null),
        );
        obj.insert(
            "round_trip_ms_max".to_string(),
            state
                .round_trip_max
                .and_then(serde_json::Number::from_f64)
                .map(Value::Number)
                .unwrap_or(Value::Null),
        );
        obj.insert(
            "round_trip_ms_stddev".to_string(),
            state
                .round_trip_stddev
                .and_then(serde_json::Number::from_f64)
                .map(Value::Number)
                .unwrap_or(Value::Null),
        );
        return Some(obj);
    }

    // ICMP error responses ("From 10.0.0.1 icmp_seq=1 Destination Host
    // Unreachable"). These carry no reply fields at all, so they must be caught
    // before the reply branch or the whole line is dropped.
    if let Some(kind) = error_type(line, state.ipv4) {
        let has_ts = line.starts_with('[');
        let offset = if has_ts { 1 } else { 0 };
        let parts: Vec<&str> = line.split_whitespace().collect();
        let cleaned = line.replace('=', " ");
        let seq_parts: Vec<&str> = cleaned.split_whitespace().collect();

        let mut obj = Map::with_capacity(6);
        obj.insert("type".to_string(), Value::String(kind.to_string()));
        obj.insert(
            "destination_ip".to_string(),
            state
                .destination_ip
                .as_deref()
                .map_or(Value::Null, |s| Value::String(s.to_string())),
        );
        obj.insert(
            "sent_bytes".to_string(),
            state
                .sent_bytes
                .map_or(Value::Null, |n| Value::Number(n.into())),
        );
        obj.insert(
            "response_ip".to_string(),
            parts
                .get(offset + 1)
                .map_or(Value::Null, |s| Value::String(s.to_string())),
        );
        obj.insert(
            "icmp_seq".to_string(),
            seq_parts
                .get(offset + 3)
                .map_or(Value::Null, |s| str_to_int(s)),
        );
        obj.insert(
            "timestamp".to_string(),
            if has_ts {
                parts.first().map_or(Value::Null, |s| {
                    Value::String(s.trim_matches(['[', ']']).to_string())
                })
            } else {
                Value::Null
            },
        );
        return Some(obj);
    }

    // Check for timeout
    if line.contains("no answer yet for icmp_seq=") {
        let has_ts = line.starts_with('[');
        let offset = if has_ts { 1 } else { 0 };
        let cleaned = line.replace('=', " ");
        let parts: Vec<&str> = cleaned.split_whitespace().collect();
        let icmp_seq = parts.get(5 + offset).copied().unwrap_or("").to_string();

        let mut obj = Map::new();
        obj.insert("type".to_string(), Value::String("timeout".to_string()));
        obj.insert(
            "destination_ip".to_string(),
            state
                .destination_ip
                .as_ref()
                .map(|s| Value::String(s.clone()))
                .unwrap_or(Value::Null),
        );
        obj.insert(
            "sent_bytes".to_string(),
            state
                .sent_bytes
                .map(|n| Value::Number(n.into()))
                .unwrap_or(Value::Null),
        );
        obj.insert(
            "pattern".to_string(),
            state
                .pattern
                .as_ref()
                .map(|s| Value::String(s.clone()))
                .unwrap_or(Value::Null),
        );
        obj.insert(
            "timestamp".to_string(),
            if has_ts {
                parts
                    .first()
                    .map(|s| str_to_float(s.trim_matches(|c| c == '[' || c == ']')))
                    .unwrap_or(Value::Null)
            } else {
                Value::Null
            },
        );
        obj.insert("icmp_seq".to_string(), str_to_int(&icmp_seq));
        return Some(obj);
    }

    // Normal reply
    if line.contains(" bytes from ") {
        let has_ts = line.starts_with('[');
        let offset = if has_ts { 1 } else { 0 };
        let cleaned = line.replace(['(', ')', '='], " ");
        let parts: Vec<&str> = cleaned.split_whitespace().collect();

        let (bts, rip, iseq, t2l, tms) = if state.ipv4 && !state.has_hostname {
            (0, 3, 5, 7, 9)
        } else if state.ipv4 && state.has_hostname {
            (0, 4, 7, 9, 11)
        } else if !state.ipv4 && !state.has_hostname {
            (0, 3, 5, 7, 9)
        } else {
            (0, 4, 7, 9, 11)
        };
        let (bts, rip, iseq, t2l, tms) = (
            bts + offset,
            rip + offset,
            iseq + offset,
            t2l + offset,
            tms + offset,
        );

        let mut obj = Map::new();
        obj.insert("type".to_string(), Value::String("reply".to_string()));
        obj.insert(
            "destination_ip".to_string(),
            state
                .destination_ip
                .as_ref()
                .map(|s| Value::String(s.clone()))
                .unwrap_or(Value::Null),
        );
        obj.insert(
            "sent_bytes".to_string(),
            state
                .sent_bytes
                .map(|n| Value::Number(n.into()))
                .unwrap_or(Value::Null),
        );
        obj.insert(
            "pattern".to_string(),
            state
                .pattern
                .as_ref()
                .map(|s| Value::String(s.clone()))
                .unwrap_or(Value::Null),
        );
        obj.insert(
            "timestamp".to_string(),
            if has_ts {
                parts
                    .first()
                    .map(|s| str_to_float(s.trim_matches(|c| c == '[' || c == ']')))
                    .unwrap_or(Value::Null)
            } else {
                Value::Null
            },
        );
        obj.insert(
            "response_bytes".to_string(),
            str_to_int(parts.get(bts).copied().unwrap_or("")),
        );
        obj.insert(
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
        obj.insert(
            "icmp_seq".to_string(),
            str_to_int(parts.get(iseq).copied().unwrap_or("")),
        );
        obj.insert(
            "ttl".to_string(),
            str_to_int(parts.get(t2l).copied().unwrap_or("")),
        );
        obj.insert(
            "time_ms".to_string(),
            str_to_float(parts.get(tms).copied().unwrap_or("")),
        );
        obj.insert("duplicate".to_string(), Value::Bool(line.contains("DUP!")));
        return Some(obj);
    }

    None
}

fn parse_bsd_line(line: &str, state: &mut PingState) -> Option<Map<String, Value>> {
    if line.starts_with("PING ") && !line.starts_with("PING6(") {
        let parts: Vec<&str> = line.split_whitespace().collect();
        state.destination_ip = parts.get(2).map(|s| {
            s.trim_start_matches('(')
                .trim_end_matches(':')
                .trim_end_matches(')')
                .to_string()
        });
        state.sent_bytes = parts.get(3).and_then(|s| s.parse::<i64>().ok());
        return None;
    }

    if line.starts_with("PING6(") {
        let cleaned = line.replace(['(', ')'], " ");
        let parts: Vec<&str> = cleaned.split_whitespace().collect();
        state.destination_ip = parts.get(6).map(|s| s.to_string());
        state.sent_bytes = parts.get(1).and_then(|s| s.parse::<i64>().ok());
        return None;
    }

    if line.starts_with("---") {
        state.in_footer = true;
        return None;
    }

    if state.in_footer {
        if line.contains("packets transmitted") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if line.contains(" duplicates,") {
                state.packets_transmitted = parts.first().and_then(|s| s.parse().ok());
                state.packets_received = parts.get(3).and_then(|s| s.parse().ok());
                state.packet_loss_percent = parts
                    .get(8)
                    .map(|s| s.trim_end_matches('%'))
                    .and_then(|s| s.parse().ok());
                state.duplicates = parts
                    .get(6)
                    .map(|s| s.trim_start_matches('+'))
                    .and_then(|s| s.parse().ok());
            } else {
                state.packets_transmitted = parts.first().and_then(|s| s.parse().ok());
                state.packets_received = parts.get(3).and_then(|s| s.parse().ok());
                state.packet_loss_percent = parts
                    .get(6)
                    .map(|s| s.trim_end_matches('%'))
                    .and_then(|s| s.parse().ok());
                state.duplicates = Some(0);
            }
            return None;
        }

        // round-trip line
        if line.contains('/')
            && let Some(eq_pos) = line.find('=')
        {
            let after = line[eq_pos + 1..].trim().trim_end_matches(" ms");
            let rtt_parts: Vec<&str> = after.split('/').collect();
            state.round_trip_min = rtt_parts.first().and_then(|s| s.trim().parse().ok());
            state.round_trip_avg = rtt_parts.get(1).and_then(|s| s.trim().parse().ok());
            state.round_trip_max = rtt_parts.get(2).and_then(|s| s.trim().parse().ok());
            state.round_trip_stddev = rtt_parts.get(3).and_then(|s| s.trim().parse().ok());

            let mut obj = Map::new();
            obj.insert("type".to_string(), Value::String("summary".to_string()));
            obj.insert(
                "destination_ip".to_string(),
                state
                    .destination_ip
                    .as_ref()
                    .map(|s| Value::String(s.clone()))
                    .unwrap_or(Value::Null),
            );
            obj.insert(
                "sent_bytes".to_string(),
                state
                    .sent_bytes
                    .map(|n| Value::Number(n.into()))
                    .unwrap_or(Value::Null),
            );
            obj.insert(
                "pattern".to_string(),
                state
                    .pattern
                    .as_ref()
                    .map(|s| Value::String(s.clone()))
                    .unwrap_or(Value::Null),
            );
            obj.insert(
                "packets_transmitted".to_string(),
                state
                    .packets_transmitted
                    .map(|n| Value::Number(n.into()))
                    .unwrap_or(Value::Null),
            );
            obj.insert(
                "packets_received".to_string(),
                state
                    .packets_received
                    .map(|n| Value::Number(n.into()))
                    .unwrap_or(Value::Null),
            );
            obj.insert(
                "packet_loss_percent".to_string(),
                state
                    .packet_loss_percent
                    .and_then(serde_json::Number::from_f64)
                    .map(Value::Number)
                    .unwrap_or(Value::Null),
            );
            obj.insert(
                "duplicates".to_string(),
                state
                    .duplicates
                    .map(|n| Value::Number(n.into()))
                    .unwrap_or(Value::Number(0i64.into())),
            );
            obj.insert(
                "round_trip_ms_min".to_string(),
                state
                    .round_trip_min
                    .and_then(serde_json::Number::from_f64)
                    .map(Value::Number)
                    .unwrap_or(Value::Null),
            );
            obj.insert(
                "round_trip_ms_avg".to_string(),
                state
                    .round_trip_avg
                    .and_then(serde_json::Number::from_f64)
                    .map(Value::Number)
                    .unwrap_or(Value::Null),
            );
            obj.insert(
                "round_trip_ms_max".to_string(),
                state
                    .round_trip_max
                    .and_then(serde_json::Number::from_f64)
                    .map(Value::Number)
                    .unwrap_or(Value::Null),
            );
            obj.insert(
                "round_trip_ms_stddev".to_string(),
                state
                    .round_trip_stddev
                    .and_then(serde_json::Number::from_f64)
                    .map(Value::Number)
                    .unwrap_or(Value::Null),
            );
            return Some(obj);
        }
        return None;
    }

    // ICMP error response. BSD reports these as `92 bytes from host (ip):
    // Destination Host Unreachable`, which also matches the reply branch below,
    // so it has to be tested first, and it reports far fewer fields.
    if let Some(kind) = error_type(line, !contains_ipv6(line)) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        let mut obj = Map::with_capacity(4);
        obj.insert("type".to_string(), Value::String(kind));
        if let Some(bytes) = parts.first() {
            obj.insert("bytes".to_string(), Value::String((*bytes).to_string()));
        }
        obj.insert(
            "destination_ip".to_string(),
            state
                .destination_ip
                .as_deref()
                .map_or(Value::Null, |s| Value::String(s.to_string())),
        );
        if let Some(response) = parts.get(4) {
            obj.insert(
                "response_ip".to_string(),
                Value::String(
                    response
                        .trim_end_matches(':')
                        .trim_start_matches('(')
                        .trim_end_matches(')')
                        .to_string(),
                ),
            );
        }
        return Some(obj);
    }

    // Request timeout
    if line.starts_with("Request timeout for ") {
        let parts: Vec<&str> = line.split_whitespace().collect();
        let icmp_seq = parts.get(4).copied().unwrap_or("").to_string();
        let mut obj = Map::new();
        obj.insert("type".to_string(), Value::String("timeout".to_string()));
        obj.insert(
            "destination_ip".to_string(),
            state
                .destination_ip
                .as_ref()
                .map(|s| Value::String(s.clone()))
                .unwrap_or(Value::Null),
        );
        obj.insert(
            "sent_bytes".to_string(),
            state
                .sent_bytes
                .map(|n| Value::Number(n.into()))
                .unwrap_or(Value::Null),
        );
        obj.insert(
            "pattern".to_string(),
            state
                .pattern
                .as_ref()
                .map(|s| Value::String(s.clone()))
                .unwrap_or(Value::Null),
        );
        obj.insert("icmp_seq".to_string(), str_to_int(&icmp_seq));
        return Some(obj);
    }

    // Normal response
    if line.contains(" bytes from ") {
        let cleaned = line.replace([':', '='], " ");
        let parts: Vec<&str> = cleaned.split_whitespace().collect();
        let mut obj = Map::new();
        obj.insert("type".to_string(), Value::String("reply".to_string()));
        obj.insert(
            "destination_ip".to_string(),
            state
                .destination_ip
                .as_ref()
                .map(|s| Value::String(s.clone()))
                .unwrap_or(Value::Null),
        );
        obj.insert(
            "sent_bytes".to_string(),
            state
                .sent_bytes
                .map(|n| Value::Number(n.into()))
                .unwrap_or(Value::Null),
        );
        obj.insert(
            "pattern".to_string(),
            state
                .pattern
                .as_ref()
                .map(|s| Value::String(s.clone()))
                .unwrap_or(Value::Null),
        );
        obj.insert(
            "response_bytes".to_string(),
            str_to_int(parts.first().copied().unwrap_or("")),
        );
        obj.insert(
            "response_ip".to_string(),
            Value::String(parts.get(3).copied().unwrap_or("").to_string()),
        );
        obj.insert(
            "icmp_seq".to_string(),
            str_to_int(parts.get(5).copied().unwrap_or("")),
        );
        obj.insert(
            "ttl".to_string(),
            str_to_int(parts.get(7).copied().unwrap_or("")),
        );
        obj.insert(
            "time_ms".to_string(),
            str_to_float(parts.get(9).copied().unwrap_or("")),
        );
        return Some(obj);
    }

    None
}

fn extract_re(pattern: &str, line: &str) -> Option<String> {
    // Called from eight sites inside the per-line loop of a streaming parser,
    // so compiling here meant recompiling the same handful of literals for
    // every line of input.
    jc_rs_utils::cached_regex(pattern)?
        .captures(line)?
        .get(1)
        .map(|m| m.as_str().to_string())
}

impl Parser for PingStreamParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }
        parse_via_session(self, input, quiet)
    }

    fn as_streaming(&self) -> Option<&dyn StreamingParser> {
        Some(self)
    }
}

impl StreamingParser for PingStreamParser {
    fn session(&self) -> Box<dyn LineParser> {
        Box::new(PingSession::default())
    }
}

/// A ping reply is one line and goes out immediately, which is what makes
/// `ping host | jc-rs -u --ping-s` usable live. The trailing statistics block
/// is several lines that accumulate into one record, so it is held until the
/// end, which is also where jc emits it.
#[derive(Default)]
struct PingSession {
    state: PingState,
    summary: Option<Map<String, Value>>,
}

impl LineParser for PingSession {
    fn parse_line(&mut self, line: &str, _quiet: bool) -> Result<Option<Record>, ParseError> {
        let line = line.trim_end();
        if line.is_empty() || line.starts_with("WARNING: ") {
            return Ok(None);
        }

        if let Some(pattern) = line.strip_prefix("PATTERN: ") {
            self.state.pattern = Some(pattern.trim().to_string());
            return Ok(None);
        }

        // The banner tells us which ping we are reading; every later line is
        // parsed against that choice.
        if self.state.linux.is_none() {
            if line.ends_with("bytes of data.") {
                self.state.linux = Some(true);
            } else if line.contains("-->") {
                self.state.linux = Some(false);
            } else if line.ends_with("data bytes") {
                self.state.linux = Some(contains_ipv6(line));
            }
        }

        let output = match self.state.linux {
            Some(true) => parse_linux_line(line, &mut self.state),
            Some(false) => parse_bsd_line(line, &mut self.state),
            None if line.starts_with("PING ") => {
                self.state.linux = Some(true);
                parse_linux_line(line, &mut self.state)
            }
            None => None,
        };

        let Some(obj) = output else {
            return Ok(None);
        };

        if obj.get("type").and_then(Value::as_str) == Some("summary") {
            self.summary = Some(obj);
            return Ok(None);
        }
        Ok(Some(obj))
    }

    fn finalize(&mut self, _quiet: bool) -> Result<Option<Record>, ParseError> {
        Ok(self.summary.take())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jc_rs_core::traits::Parser;

    #[test]
    fn test_ping_s_centos_golden() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/ping-ip-O.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/ping-ip-O-streaming.json"
        ))
        .unwrap();
        let result = PingStreamParser.parse(input, false).unwrap();
        let actual = serde_json::to_value(result).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_ping_s_empty() {
        let result = PingStreamParser.parse("", false).unwrap();
        assert!(matches!(result, ParseOutput::Array(v) if v.is_empty()));
    }

    #[test]
    fn test_ping_s_registered() {
        assert!(jc_rs_core::registry::find_parser("ping_s").is_some());
    }
}
