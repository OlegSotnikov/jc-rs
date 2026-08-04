//! Parser for `traceroute` and `traceroute6` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use regex::Regex;
use serde_json::{Map, Value};

pub struct TracerouteParser;

static INFO: ParserInfo = ParserInfo {
    name: "traceroute",
    argument: "--traceroute",
    version: "1.9.0",
    description: "Converts `traceroute` and `traceroute6` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux, Platform::Darwin, Platform::FreeBSD],
    tags: &[Tag::Command],
    magic_commands: &["traceroute", "traceroute6"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static TRACEROUTE_PARSER: TracerouteParser = TracerouteParser;

inventory::submit! { ParserEntry::new(&TRACEROUTE_PARSER) }

fn str_to_int_opt(s: &str) -> Option<i64> {
    s.parse::<i64>().ok()
}

fn str_to_float_opt(s: &str) -> Option<f64> {
    s.parse::<f64>().ok()
}

#[derive(Debug, Clone)]
struct Probe {
    annotation: Option<String>,
    asn: Option<i64>,
    ip: Option<String>,
    name: Option<String>,
    rtt: Option<f64>,
}

impl Probe {
    fn new() -> Self {
        Probe {
            annotation: None,
            asn: None,
            ip: None,
            name: None,
            rtt: None,
        }
    }

    fn to_value(&self) -> Value {
        let mut obj = Map::new();
        obj.insert(
            "annotation".to_string(),
            self.annotation
                .as_ref()
                .map(|s| Value::String(s.clone()))
                .unwrap_or(Value::Null),
        );
        obj.insert(
            "asn".to_string(),
            self.asn
                .map(|n| Value::Number(n.into()))
                .unwrap_or(Value::Null),
        );
        obj.insert(
            "ip".to_string(),
            self.ip
                .as_ref()
                .map(|s| Value::String(s.clone()))
                .unwrap_or(Value::Null),
        );
        obj.insert(
            "name".to_string(),
            self.name
                .as_ref()
                .map(|s| Value::String(s.clone()))
                .unwrap_or(Value::Null),
        );
        obj.insert(
            "rtt".to_string(),
            self.rtt
                .and_then(|f| serde_json::Number::from_f64(f))
                .map(Value::Number)
                .unwrap_or(Value::Null),
        );
        Value::Object(obj)
    }
}

fn get_probes(hop_string: &str) -> Vec<Probe> {
    let re_asn = Regex::new(r"\[AS(\d+)\]").unwrap();
    let re_name_ip =
        Regex::new(r"(\S+)\s+\((\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}|[0-9a-fA-F:]+)\)").unwrap();
    let re_ip_only = Regex::new(r"(\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3})\s+[^(]").unwrap();
    // Handle compressed IPv6 (e.g. 2605:9000:402:6a01::1) - match 2+ colon-separated groups
    let re_ipv6 =
        Regex::new(r"(?:^|[\s(])([0-9A-Fa-f]{1,4}(?::[0-9A-Fa-f]{0,4}){2,7})(?:[\s)]|$)").unwrap();
    let _re_ipv6_only = Regex::new(r"(([a-f0-9]*:)+[a-f0-9]+)").unwrap();
    let _re_rtt = Regex::new(r"(?:(\d+(?:\.?\d+)?)\s+ms|(\s+\*\s+))\s*(!\\S*)?").unwrap();

    // Collect all match positions
    struct MatchItem {
        start: usize,
        match_type: &'static str,
        value: String,
        value2: Option<String>,
        value3: Option<String>,
    }

    let mut matches: Vec<MatchItem> = Vec::new();

    for cap in re_asn.captures_iter(hop_string) {
        let m = cap.get(0).unwrap();
        matches.push(MatchItem {
            start: m.start(),
            match_type: "ASN",
            value: cap.get(1).map_or("", |m| m.as_str()).to_string(),
            value2: None,
            value3: None,
        });
    }

    for cap in re_name_ip.captures_iter(hop_string) {
        let m = cap.get(0).unwrap();
        matches.push(MatchItem {
            start: m.start(),
            match_type: "NAME_IP",
            value: cap.get(1).map_or("", |m| m.as_str()).to_string(),
            value2: Some(cap.get(2).map_or("", |m| m.as_str()).to_string()),
            value3: None,
        });
    }

    for cap in re_ip_only.captures_iter(hop_string) {
        let m = cap.get(0).unwrap();
        // Only add if not already covered by NAME_IP
        let ip_pos = m.start();
        let already_covered = matches.iter().any(|x| {
            x.match_type == "NAME_IP" && {
                let d = x.start as isize - ip_pos as isize;
                d.abs() < 50
            }
        });
        if !already_covered {
            matches.push(MatchItem {
                start: ip_pos,
                match_type: "IP_ONLY",
                value: cap.get(1).map_or("", |m| m.as_str()).to_string(),
                value2: None,
                value3: None,
            });
        }
    }

    for cap in re_ipv6.captures_iter(hop_string) {
        let m = cap.get(0).unwrap();
        // Use capture group 1 for the IPv6 address (strips surrounding whitespace/parens)
        let ipv6_str = cap.get(1).map_or(m.as_str(), |c| c.as_str());
        // Skip if already covered by NAME_IP match
        let already_covered = matches.iter().any(|x| {
            x.match_type == "NAME_IP" && {
                let d = x.start as isize - m.start() as isize;
                d.abs() < 50
            }
        });
        if !already_covered {
            matches.push(MatchItem {
                start: m.start(),
                match_type: "IP_IPV6",
                value: ipv6_str.to_string(),
                value2: None,
                value3: None,
            });
        }
    }

    // RTT matches
    for cap in Regex::new(r"(?:(\d+(?:[.]\d+)?)\s+ms|(\s+[*]\s+))\s*(![\S]*)?\s*")
        .unwrap()
        .captures_iter(hop_string)
    {
        let m = cap.get(0).unwrap();
        matches.push(MatchItem {
            start: m.start(),
            match_type: "RTT",
            value: cap.get(1).map_or("", |m| m.as_str()).to_string(),
            value2: cap.get(2).map(|m| m.as_str().to_string()),
            value3: cap.get(3).map(|m| m.as_str().to_string()),
        });
    }

    matches.sort_by_key(|m| m.start);

    let mut probes: Vec<Probe> = Vec::new();
    let mut probe = Probe::new();
    let mut last_was_rtt = false;
    let mut last_probe: Option<Probe> = None;

    for item in &matches {
        match item.match_type {
            "ASN" => {
                probe.asn = item.value.parse::<i64>().ok();
                last_was_rtt = false;
            }
            "NAME_IP" => {
                probe.name = Some(item.value.clone());
                probe.ip = item.value2.clone();
                last_was_rtt = false;
            }
            "IP_ONLY" => {
                probe.ip = Some(item.value.clone());
                last_was_rtt = false;
            }
            "IP_IPV6" => {
                probe.ip = Some(item.value.clone());
                last_was_rtt = false;
            }
            "RTT" => {
                let rtt = if !item.value.is_empty() {
                    item.value.parse::<f64>().ok()
                } else {
                    None // * means no response
                };

                // If last match was also RTT, carry ip/name/asn from last probe
                if last_was_rtt {
                    if let Some(ref lp) = last_probe {
                        if probe.ip.is_none() {
                            probe.ip = lp.ip.clone();
                            probe.name = lp.name.clone();
                        }
                        if probe.asn.is_none() {
                            probe.asn = lp.asn;
                        }
                    }
                }

                probe.rtt = rtt;
                probe.annotation = item.value3.as_ref().filter(|s| !s.is_empty()).cloned();

                let has_data = probe.ip.is_some()
                    || probe.asn.is_some()
                    || probe.annotation.is_some()
                    || probe.rtt.is_some()
                    || probe.name.is_some();
                if has_data {
                    last_probe = Some(probe.clone());
                    probes.push(probe.clone());
                }
                probe = Probe::new();
                last_was_rtt = true;
                continue;
            }
            _ => {}
        }
    }

    probes
}

impl Parser for TracerouteParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Object(Map::new()));
        }

        let re_header =
            Regex::new(r"traceroute6? to (\S+)\s+\((\d+\.\d+\.\d+\.\d+|[0-9a-fA-F:]+)\)")
                .map_err(|e| ParseError::Regex(e.to_string()))?;
        let re_hops_bytes = Regex::new(r"(\d+) hops max, (\d+) byte packets")
            .map_err(|e| ParseError::Regex(e.to_string()))?;
        let re_hop =
            Regex::new(r"^\s*(\d+)?\s+(.+)$").map_err(|e| ParseError::Regex(e.to_string()))?;

        // Filter warning lines
        let mut lines: Vec<&str> = input
            .lines()
            .filter(|l| !l.contains("traceroute: Warning:") && !l.contains("traceroute6: Warning:"))
            .collect();

        if lines.is_empty() {
            return Ok(ParseOutput::Object(Map::new()));
        }

        // Check if header row exists
        let has_header =
            lines[0].starts_with("traceroute to ") || lines[0].starts_with("traceroute6 to ");
        if !has_header {
            if !quiet {
                eprintln!("jc-rs warning: No header row found in traceroute output.");
            }
            let dummy = "traceroute to <<_>>  (<<_>>), ? hops max, ? byte packets";
            lines.insert(0, dummy);
        }

        let mut obj = Map::new();

        // Parse header - always emit all 4 fields (null if no-header format)
        if let Some(caps) = re_header.captures(lines[0]) {
            obj.insert(
                "destination_name".to_string(),
                Value::String(caps.get(1).map_or("", |m| m.as_str()).to_string()),
            );
            obj.insert(
                "destination_ip".to_string(),
                Value::String(caps.get(2).map_or("", |m| m.as_str()).to_string()),
            );
        } else {
            obj.insert("destination_name".to_string(), Value::Null);
            obj.insert("destination_ip".to_string(), Value::Null);
        }

        if let Some(caps) = re_hops_bytes.captures(lines[0]) {
            if let Some(n) = str_to_int_opt(caps.get(1).map_or("", |m| m.as_str())) {
                obj.insert("max_hops".to_string(), Value::Number(n.into()));
            } else {
                obj.insert("max_hops".to_string(), Value::Null);
            }
            if let Some(n) = str_to_int_opt(caps.get(2).map_or("", |m| m.as_str())) {
                obj.insert("data_bytes".to_string(), Value::Number(n.into()));
            } else {
                obj.insert("data_bytes".to_string(), Value::Null);
            }
        } else {
            obj.insert("max_hops".to_string(), Value::Null);
            obj.insert("data_bytes".to_string(), Value::Null);
        }

        let mut hops: Vec<Value> = Vec::new();
        let mut current_hop: Option<(i64, Vec<Probe>)> = None;
        let mut hop_string = String::new();

        for line in &lines[1..] {
            if line.trim().is_empty() {
                continue;
            }

            if let Some(caps) = re_hop.captures(line) {
                // If new hop number, flush previous
                if let Some(hop_num_str) = caps.get(1) {
                    if let Some((idx, _probes)) = current_hop.take() {
                        let probe_values: Vec<Value> = get_probes(&hop_string)
                            .into_iter()
                            .map(|p| p.to_value())
                            .collect();
                        let mut hop_obj = Map::new();
                        hop_obj.insert("hop".to_string(), Value::Number(idx.into()));
                        hop_obj.insert("probes".to_string(), Value::Array(probe_values));
                        hops.push(Value::Object(hop_obj));
                    }
                    let hop_idx = hop_num_str.as_str().parse::<i64>().unwrap_or(0);
                    hop_string = caps.get(2).map_or("", |m| m.as_str()).to_string();
                    current_hop = Some((hop_idx, Vec::new()));
                } else {
                    // Continuation line (no hop number)
                    if current_hop.is_some() {
                        hop_string.push(' ');
                        hop_string.push_str(caps.get(2).map_or("", |m| m.as_str()));
                    }
                }
            }
        }

        // Flush last hop
        if let Some((idx, _)) = current_hop.take() {
            let probe_values: Vec<Value> = get_probes(&hop_string)
                .into_iter()
                .map(|p| p.to_value())
                .collect();
            let mut hop_obj = Map::new();
            hop_obj.insert("hop".to_string(), Value::Number(idx.into()));
            hop_obj.insert("probes".to_string(), Value::Array(probe_values));
            hops.push(Value::Object(hop_obj));
        }

        obj.insert("hops".to_string(), Value::Array(hops));

        Ok(ParseOutput::Object(obj))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jc_rs_core::traits::Parser;

    #[test]
    fn test_traceroute_centos_golden() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/traceroute.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/traceroute.json"
        ))
        .unwrap();
        let result = TracerouteParser.parse(input, false).unwrap();
        let actual = serde_json::to_value(result).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_traceroute_ipv4_golden() {
        let input = include_str!("../../../../tests/fixtures/generic/traceroute-n-ipv4.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/traceroute-n-ipv4.json"
        ))
        .unwrap();
        let result = TracerouteParser.parse(input, false).unwrap();
        let actual = serde_json::to_value(result).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_traceroute_empty() {
        let result = TracerouteParser.parse("", false).unwrap();
        assert!(matches!(result, ParseOutput::Object(m) if m.is_empty()));
    }

    #[test]
    fn test_traceroute_registered() {
        assert!(jc_rs_core::registry::find_parser("traceroute").is_some());
    }
}
