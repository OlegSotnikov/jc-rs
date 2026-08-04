//! Parser for `ufw app info [application]` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct UfwAppinfoParser;

static INFO: ParserInfo = ParserInfo {
    name: "ufw_appinfo",
    argument: "--ufw-appinfo",
    version: "1.3.0",
    description: "Converts `ufw app info [application]` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &["ufw app"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static UFW_APPINFO_PARSER: UfwAppinfoParser = UfwAppinfoParser;

inventory::submit! {
    ParserEntry::new(&UFW_APPINFO_PARSER)
}

fn parse_port_list(data: &str) -> Vec<String> {
    let mut result = Vec::new();
    for part in data.split(',') {
        let part = part.trim();
        if !part.is_empty() && !part.contains(':') && part != "any" {
            result.push(part.to_string());
        }
    }
    result
}

fn parse_port_ranges(data: &str) -> Vec<(String, String)> {
    let mut result = Vec::new();
    let data = data.trim();

    if data == "any" {
        result.push(("0".to_string(), "65535".to_string()));
        return result;
    }

    for part in data.split(',') {
        let part = part.trim();
        if part.contains(':') {
            let sides: Vec<&str> = part.splitn(2, ':').collect();
            if sides.len() == 2 {
                result.push((sides[0].trim().to_string(), sides[1].trim().to_string()));
            }
        }
    }
    result
}

fn normalize_ports(ports: &[i64], ranges: &[(i64, i64)]) -> (Vec<i64>, Vec<(i64, i64)>) {
    // Build a set of all ports in ranges
    let mut range_set: std::collections::HashSet<i64> = std::collections::HashSet::new();
    for (start, end) in ranges {
        for p in *start..=*end {
            range_set.insert(p);
        }
    }

    // Normalized port list: unique ports not in range_set
    let mut seen_ports = std::collections::HashSet::new();
    let mut norm_ports: Vec<i64> = Vec::new();
    for &p in ports {
        if !range_set.contains(&p) && seen_ports.insert(p) {
            norm_ports.push(p);
        }
    }
    norm_ports.sort();

    // Normalize ranges: merge overlapping ranges
    let mut all_range_ports: Vec<i64> = range_set.into_iter().collect();
    all_range_ports.sort();

    let mut norm_ranges: Vec<(i64, i64)> = Vec::new();
    let mut state = "findstart";
    let mut current_start = 0i64;
    let mut prev = -2i64;

    for &p in &all_range_ports {
        if state == "findstart" {
            current_start = p;
            state = "findend";
        } else if p != prev + 1 {
            // gap
            norm_ranges.push((current_start, prev));
            current_start = p;
        }
        prev = p;
    }
    if state == "findend" {
        norm_ranges.push((current_start, prev));
    }

    (norm_ports, norm_ranges)
}

impl Parser for UfwAppinfoParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(Vec::new()));
        }

        let mut raw_output: Vec<Map<String, Value>> = Vec::new();
        let mut item: Option<Map<String, Value>> = None;
        let mut in_ports = false;

        for line in input.lines() {
            // blank line resets port parsing in some formats, but we keep going
            if line.starts_with("--") {
                if let Some(obj) = item.take() {
                    raw_output.push(obj);
                    in_ports = false;
                }
                item = Some(Map::new());
                continue;
            }

            if line.starts_with("Profile:") {
                if item.is_none() {
                    item = Some(Map::new());
                }
                if let Some(ref mut obj) = item
                    && let Some(val) = line.split_once(": ").map(|x| x.1)
                {
                    obj.insert("profile".to_string(), Value::String(val.trim().to_string()));
                }
                in_ports = false;
                continue;
            }

            if line.starts_with("Title:") {
                if let Some(ref mut obj) = item
                    && let Some(val) = line.split_once(": ").map(|x| x.1)
                {
                    obj.insert("title".to_string(), Value::String(val.trim().to_string()));
                }
                in_ports = false;
                continue;
            }

            if line.starts_with("Description:") {
                if let Some(ref mut obj) = item
                    && let Some(val) = line.split_once(": ").map(|x| x.1)
                {
                    obj.insert(
                        "description".to_string(),
                        Value::String(val.trim().to_string()),
                    );
                }
                in_ports = false;
                continue;
            }

            if line.starts_with("Port") {
                in_ports = true;
                continue;
            }

            if in_ports {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                if let Some(ref mut obj) = item {
                    // Check for protocol-specific: "1,2,3/tcp" or "50:51/udp"
                    let parts: Vec<&str> = trimmed.rsplitn(2, '/').collect();
                    if parts.len() == 2 {
                        let proto = parts[0].trim();
                        let port_data = parts[1].trim();

                        match proto {
                            "tcp" => {
                                let plist = parse_port_list(port_data);
                                if !plist.is_empty() {
                                    obj.insert(
                                        "tcp_list".to_string(),
                                        Value::Array(
                                            plist.into_iter().map(Value::String).collect(),
                                        ),
                                    );
                                }

                                let pranges = parse_port_ranges(port_data);
                                if !pranges.is_empty() {
                                    let ranges: Vec<Value> = pranges
                                        .into_iter()
                                        .map(|(s, e)| {
                                            let mut range_obj = Map::new();
                                            range_obj.insert("start".to_string(), Value::String(s));
                                            range_obj.insert("end".to_string(), Value::String(e));
                                            Value::Object(range_obj)
                                        })
                                        .collect();
                                    obj.insert("tcp_ranges".to_string(), Value::Array(ranges));
                                }
                            }
                            "udp" => {
                                let plist = parse_port_list(port_data);
                                if !plist.is_empty() {
                                    obj.insert(
                                        "udp_list".to_string(),
                                        Value::Array(
                                            plist.into_iter().map(Value::String).collect(),
                                        ),
                                    );
                                }

                                let pranges = parse_port_ranges(port_data);
                                if !pranges.is_empty() {
                                    let ranges: Vec<Value> = pranges
                                        .into_iter()
                                        .map(|(s, e)| {
                                            let mut range_obj = Map::new();
                                            range_obj.insert("start".to_string(), Value::String(s));
                                            range_obj.insert("end".to_string(), Value::String(e));
                                            Value::Object(range_obj)
                                        })
                                        .collect();
                                    obj.insert("udp_ranges".to_string(), Value::Array(ranges));
                                }
                            }
                            _ => {}
                        }
                    } else {
                        // No protocol suffix: applies to both tcp and udp
                        let port_data = trimmed;

                        let plist = parse_port_list(port_data);
                        let pranges = parse_port_ranges(port_data);

                        // Add to tcp
                        if !plist.is_empty() {
                            let existing: Vec<String> = obj
                                .get("tcp_list")
                                .and_then(|v| v.as_array())
                                .map(|a| {
                                    a.iter()
                                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                        .collect()
                                })
                                .unwrap_or_default();
                            let mut merged = existing;
                            merged.extend_from_slice(&plist);
                            obj.insert(
                                "tcp_list".to_string(),
                                Value::Array(merged.into_iter().map(Value::String).collect()),
                            );
                        }
                        if !pranges.is_empty() {
                            let existing: Vec<Value> = obj
                                .get("tcp_ranges")
                                .and_then(|v| v.as_array())
                                .cloned()
                                .unwrap_or_default();
                            let mut merged = existing;
                            for (s, e) in &pranges {
                                let mut range_obj = Map::new();
                                range_obj.insert("start".to_string(), Value::String(s.clone()));
                                range_obj.insert("end".to_string(), Value::String(e.clone()));
                                merged.push(Value::Object(range_obj));
                            }
                            obj.insert("tcp_ranges".to_string(), Value::Array(merged));
                        }

                        // Add to udp
                        if !plist.is_empty() {
                            let existing: Vec<String> = obj
                                .get("udp_list")
                                .and_then(|v| v.as_array())
                                .map(|a| {
                                    a.iter()
                                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                        .collect()
                                })
                                .unwrap_or_default();
                            let mut merged = existing;
                            merged.extend_from_slice(&plist);
                            obj.insert(
                                "udp_list".to_string(),
                                Value::Array(merged.into_iter().map(Value::String).collect()),
                            );
                        }
                        if !pranges.is_empty() {
                            let existing: Vec<Value> = obj
                                .get("udp_ranges")
                                .and_then(|v| v.as_array())
                                .cloned()
                                .unwrap_or_default();
                            let mut merged = existing;
                            for (s, e) in &pranges {
                                let mut range_obj = Map::new();
                                range_obj.insert("start".to_string(), Value::String(s.clone()));
                                range_obj.insert("end".to_string(), Value::String(e.clone()));
                                merged.push(Value::Object(range_obj));
                            }
                            obj.insert("udp_ranges".to_string(), Value::Array(merged));
                        }
                    }
                }
            }
        }

        if let Some(obj) = item.take() {
            raw_output.push(obj);
        }

        // Process: convert string ports to integers and compute normalized lists
        let processed: Vec<Map<String, Value>> = raw_output
            .into_iter()
            .map(|mut profile| {
                // Convert tcp_list to int
                if let Some(Value::Array(list)) = profile.get("tcp_list").cloned() {
                    let ints: Vec<i64> = list
                        .iter()
                        .filter_map(|v| v.as_str().and_then(|s| s.parse::<i64>().ok()))
                        .collect();
                    profile.insert(
                        "tcp_list".to_string(),
                        Value::Array(ints.iter().map(|&n| Value::Number(n.into())).collect()),
                    );
                }

                // Convert udp_list to int
                if let Some(Value::Array(list)) = profile.get("udp_list").cloned() {
                    let ints: Vec<i64> = list
                        .iter()
                        .filter_map(|v| v.as_str().and_then(|s| s.parse::<i64>().ok()))
                        .collect();
                    profile.insert(
                        "udp_list".to_string(),
                        Value::Array(ints.iter().map(|&n| Value::Number(n.into())).collect()),
                    );
                }

                // Convert tcp_ranges start/end to int
                for proto in &["tcp", "udp"] {
                    let key = format!("{}_ranges", proto);
                    if let Some(Value::Array(ranges)) = profile.get(&key).cloned() {
                        let converted: Vec<Value> = ranges
                            .into_iter()
                            .map(|r| {
                                if let Value::Object(mut range_obj) = r {
                                    if let Some(Value::String(s)) = range_obj.get("start").cloned()
                                        && let Ok(n) = s.parse::<i64>()
                                    {
                                        range_obj
                                            .insert("start".to_string(), Value::Number(n.into()));
                                    }
                                    if let Some(Value::String(s)) = range_obj.get("end").cloned()
                                        && let Ok(n) = s.parse::<i64>()
                                    {
                                        range_obj
                                            .insert("end".to_string(), Value::Number(n.into()));
                                    }
                                    Value::Object(range_obj)
                                } else {
                                    r
                                }
                            })
                            .collect();
                        profile.insert(key.clone(), Value::Array(converted));
                    }
                }

                // Compute normalized lists and ranges
                for proto in &["tcp", "udp"] {
                    let list_key = format!("{}_list", proto);
                    let range_key = format!("{}_ranges", proto);

                    let port_list: Vec<i64> = profile
                        .get(&list_key)
                        .and_then(|v| v.as_array())
                        .map(|a| a.iter().filter_map(|v| v.as_i64()).collect())
                        .unwrap_or_default();

                    let range_list: Vec<(i64, i64)> = profile
                        .get(&range_key)
                        .and_then(|v| v.as_array())
                        .map(|a| {
                            a.iter()
                                .filter_map(|v| {
                                    if let Value::Object(r) = v {
                                        let s = r.get("start").and_then(|v| v.as_i64())?;
                                        let e = r.get("end").and_then(|v| v.as_i64())?;
                                        Some((s, e))
                                    } else {
                                        None
                                    }
                                })
                                .collect()
                        })
                        .unwrap_or_default();

                    if !port_list.is_empty() || !range_list.is_empty() {
                        let (norm_ports, norm_ranges) = normalize_ports(&port_list, &range_list);

                        if !norm_ports.is_empty() {
                            profile.insert(
                                format!("normalized_{}_list", proto),
                                Value::Array(
                                    norm_ports
                                        .iter()
                                        .map(|&n| Value::Number(n.into()))
                                        .collect(),
                                ),
                            );
                        }

                        if !norm_ranges.is_empty() {
                            profile.insert(
                                format!("normalized_{}_ranges", proto),
                                Value::Array(
                                    norm_ranges
                                        .iter()
                                        .map(|(s, e)| {
                                            let mut r = Map::new();
                                            r.insert(
                                                "start".to_string(),
                                                Value::Number((*s).into()),
                                            );
                                            r.insert("end".to_string(), Value::Number((*e).into()));
                                            Value::Object(r)
                                        })
                                        .collect(),
                                ),
                            );
                        }
                    }
                }

                profile
            })
            .collect();

        Ok(ParseOutput::Array(processed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ufw_appinfo_msn() {
        let input = "Profile: MSN\nTitle: MSN Chat\nDescription: MSN chat protocol (with file transfer and voice)\n\nPorts:\n  1863\n  6891:6900/tcp\n  6901\n";
        let parser = UfwAppinfoParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 1);
            assert_eq!(
                arr[0].get("profile"),
                Some(&Value::String("MSN".to_string()))
            );
            assert!(arr[0].contains_key("normalized_tcp_list"));
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_ufw_appinfo_empty() {
        let parser = UfwAppinfoParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 0);
        } else {
            panic!("Expected Array");
        }
    }
}
