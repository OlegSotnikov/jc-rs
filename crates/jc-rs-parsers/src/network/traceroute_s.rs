//! Streaming parser for `traceroute` and `traceroute6` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use regex::Regex;
use serde_json::{Map, Value};

pub struct TracerouteStreamParser;

static INFO: ParserInfo = ParserInfo {
    name: "traceroute_s",
    argument: "--traceroute-s",
    version: "1.9.0",
    description: "Streaming parser for `traceroute` and `traceroute6` command output",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux, Platform::Darwin, Platform::FreeBSD],
    tags: &[Tag::Command],
    magic_commands: &[],
    streaming: true,
    hidden: false,
    deprecated: false,
};

static TRACEROUTE_STREAM_PARSER: TracerouteStreamParser = TracerouteStreamParser;
inventory::submit! { ParserEntry::new(&TRACEROUTE_STREAM_PARSER) }

// Re-use the get_probes logic from traceroute
fn get_probes(hop_string: &str) -> Vec<Map<String, Value>> {
    let re_asn = Regex::new(r"\[AS(\d+)\]").unwrap();
    let re_name_ip =
        Regex::new(r"(\S+)\s+\((\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}|[0-9a-fA-F:]+)\)").unwrap();
    let re_ip_only = Regex::new(r"(\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3})\s+[^(]").unwrap();
    let re_ipv6 = Regex::new(r"\b(?:[A-Fa-f0-9]{1,4}:){7}[A-Fa-f0-9]{1,4}\b").unwrap();
    let re_rtt = Regex::new(r"(?:(\d+(?:[.]\d+)?)\s+ms|(\s+[*]\s+))\s*(![\S]*)?\s*").unwrap();

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
        matches.push(MatchItem {
            start: m.start(),
            match_type: "IP_IPV6",
            value: m.as_str().to_string(),
            value2: None,
            value3: None,
        });
    }
    for cap in re_rtt.captures_iter(hop_string) {
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

    struct ProbeState {
        annotation: Option<String>,
        asn: Option<i64>,
        ip: Option<String>,
        name: Option<String>,
        rtt: Option<f64>,
    }

    let mut probes: Vec<Map<String, Value>> = Vec::new();
    let mut probe = ProbeState {
        annotation: None,
        asn: None,
        ip: None,
        name: None,
        rtt: None,
    };
    let mut last_was_rtt = false;
    let mut last_ip: Option<String> = None;
    let mut last_name: Option<String> = None;

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
                    None
                };

                if last_was_rtt {
                    if probe.ip.is_none() {
                        probe.ip = last_ip.clone();
                        probe.name = last_name.clone();
                    }
                }

                probe.rtt = rtt;
                probe.annotation = item.value3.as_ref().filter(|s| !s.is_empty()).cloned();

                last_ip = probe.ip.clone();
                last_name = probe.name.clone();

                let has_data = probe.ip.is_some()
                    || probe.asn.is_some()
                    || probe.annotation.is_some()
                    || probe.rtt.is_some()
                    || probe.name.is_some();

                if has_data {
                    let mut obj = Map::new();
                    obj.insert(
                        "annotation".to_string(),
                        probe
                            .annotation
                            .as_ref()
                            .map(|s| Value::String(s.clone()))
                            .unwrap_or(Value::Null),
                    );
                    obj.insert(
                        "asn".to_string(),
                        probe
                            .asn
                            .map(|n| Value::Number(n.into()))
                            .unwrap_or(Value::Null),
                    );
                    obj.insert(
                        "ip".to_string(),
                        probe
                            .ip
                            .as_ref()
                            .map(|s| Value::String(s.clone()))
                            .unwrap_or(Value::Null),
                    );
                    obj.insert(
                        "name".to_string(),
                        probe
                            .name
                            .as_ref()
                            .map(|s| Value::String(s.clone()))
                            .unwrap_or(Value::Null),
                    );
                    obj.insert(
                        "rtt".to_string(),
                        probe
                            .rtt
                            .and_then(|f| serde_json::Number::from_f64(f))
                            .map(Value::Number)
                            .unwrap_or(Value::Null),
                    );
                    probes.push(obj);
                }

                probe = ProbeState {
                    annotation: None,
                    asn: None,
                    ip: None,
                    name: None,
                    rtt: None,
                };
                last_was_rtt = true;
                continue;
            }
            _ => {}
        }
    }

    probes
}

impl Parser for TracerouteStreamParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let re_header =
            Regex::new(r"traceroute6? to (\S+)\s+\((\d+\.\d+\.\d+\.\d+|[0-9a-fA-F:]+)\)")
                .map_err(|e| ParseError::Regex(e.to_string()))?;
        let re_hops_bytes = Regex::new(r"(\d+) hops max, (\d+) byte packets")
            .map_err(|e| ParseError::Regex(e.to_string()))?;
        let re_hop =
            Regex::new(r"^\s*(\d+)?\s+(.+)$").map_err(|e| ParseError::Regex(e.to_string()))?;

        let lines: Vec<&str> = input
            .lines()
            .filter(|l| !l.contains("traceroute: Warning:") && !l.contains("traceroute6: Warning:"))
            .collect();

        if lines.is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let mut results: Vec<Map<String, Value>> = Vec::new();
        let mut current_hop: Option<(i64, String)> = None;

        for line in &lines {
            if line.trim().is_empty() {
                continue;
            }

            // Header line
            if line.starts_with("traceroute") {
                let mut header = Map::new();
                header.insert("type".to_string(), Value::String("header".to_string()));

                if let Some(caps) = re_header.captures(line) {
                    header.insert(
                        "destination_name".to_string(),
                        Value::String(caps.get(1).map_or("", |m| m.as_str()).to_string()),
                    );
                    header.insert(
                        "destination_ip".to_string(),
                        Value::String(caps.get(2).map_or("", |m| m.as_str()).to_string()),
                    );
                }
                if let Some(caps) = re_hops_bytes.captures(line) {
                    header.insert(
                        "max_hops".to_string(),
                        caps.get(1)
                            .and_then(|m| m.as_str().parse::<i64>().ok())
                            .map(|n| Value::Number(n.into()))
                            .unwrap_or(Value::Null),
                    );
                    header.insert(
                        "data_bytes".to_string(),
                        caps.get(2)
                            .and_then(|m| m.as_str().parse::<i64>().ok())
                            .map(|n| Value::Number(n.into()))
                            .unwrap_or(Value::Null),
                    );
                }
                results.push(header);
                continue;
            }

            if let Some(caps) = re_hop.captures(line) {
                if let Some(hop_num_str) = caps.get(1) {
                    // Flush previous hop
                    if let Some((idx, hop_str)) = current_hop.take() {
                        let probes = get_probes(&hop_str);
                        let mut hop_obj = Map::new();
                        hop_obj.insert("type".to_string(), Value::String("hop".to_string()));
                        hop_obj.insert("hop".to_string(), Value::Number(idx.into()));
                        hop_obj.insert(
                            "probes".to_string(),
                            Value::Array(probes.into_iter().map(Value::Object).collect()),
                        );
                        results.push(hop_obj);
                    }
                    let hop_idx = hop_num_str.as_str().parse::<i64>().unwrap_or(0);
                    let hop_str = caps.get(2).map_or("", |m| m.as_str()).to_string();
                    current_hop = Some((hop_idx, hop_str));
                } else {
                    // Continuation line
                    if let Some((_, ref mut hop_str)) = current_hop {
                        hop_str.push(' ');
                        hop_str.push_str(caps.get(2).map_or("", |m| m.as_str()));
                    }
                }
            }
        }

        // Flush last hop
        if let Some((idx, hop_str)) = current_hop {
            let probes = get_probes(&hop_str);
            let mut hop_obj = Map::new();
            hop_obj.insert("type".to_string(), Value::String("hop".to_string()));
            hop_obj.insert("hop".to_string(), Value::Number(idx.into()));
            hop_obj.insert(
                "probes".to_string(),
                Value::Array(probes.into_iter().map(Value::Object).collect()),
            );
            results.push(hop_obj);
        }

        Ok(ParseOutput::Array(results))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jc_rs_core::traits::Parser;

    #[test]
    fn test_traceroute_s_ipv4_golden() {
        let input = include_str!("../../../../tests/fixtures/generic/traceroute-n-ipv4.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/traceroute-n-ipv4-streaming.json"
        ))
        .unwrap();
        let result = TracerouteStreamParser.parse(input, false).unwrap();
        let actual = serde_json::to_value(result).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_traceroute_s_empty() {
        let result = TracerouteStreamParser.parse("", false).unwrap();
        assert!(matches!(result, ParseOutput::Array(v) if v.is_empty()));
    }

    #[test]
    fn test_traceroute_s_registered() {
        assert!(jc_rs_core::registry::find_parser("traceroute_s").is_some());
    }
}
