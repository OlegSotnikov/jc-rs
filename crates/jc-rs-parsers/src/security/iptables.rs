//! Parser for `iptables` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use regex::Regex;
use serde_json::{Map, Value};

pub struct IptablesParser;

static INFO: ParserInfo = ParserInfo {
    name: "iptables",
    argument: "--iptables",
    version: "1.12.0",
    description: "Converts `iptables` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &["iptables"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static IPTABLES_PARSER: IptablesParser = IptablesParser;

inventory::submit! {
    ParserEntry::new(&IPTABLES_PARSER)
}

/// Convert size strings like "186K", "2M", "1G" to bytes
fn convert_size_to_int(s: &str) -> Value {
    let s = s.trim();
    if s.is_empty() {
        return Value::Number(0.into());
    }
    let (num_str, mult) = if s.ends_with('K') || s.ends_with('k') {
        (&s[..s.len() - 1], 1000i64)
    } else if s.ends_with('M') || s.ends_with('m') {
        (&s[..s.len() - 1], 1000 * 1000i64)
    } else if s.ends_with('G') || s.ends_with('g') {
        (&s[..s.len() - 1], 1000 * 1000 * 1000i64)
    } else if s.ends_with('T') || s.ends_with('t') {
        (&s[..s.len() - 1], 1000 * 1000 * 1000 * 1000i64)
    } else {
        (s, 1i64)
    };

    if let Ok(n) = num_str.parse::<f64>() {
        Value::Number((n as i64 * mult).into())
    } else if let Ok(n) = s.parse::<i64>() {
        Value::Number(n.into())
    } else {
        Value::String(s.to_string())
    }
}

/// Split string on whitespace like Python's `str.split(maxsplit=n-1)`.
/// This strips leading whitespace and collapses consecutive whitespace.
fn python_split_whitespace(s: &str, max_parts: usize) -> Vec<&str> {
    if max_parts == 0 {
        return Vec::new();
    }
    let mut result = Vec::new();
    let mut remaining = s.trim_start();

    while !remaining.is_empty() && result.len() < max_parts - 1 {
        // Find next whitespace
        let end = remaining
            .char_indices()
            .find(|(_, c)| c.is_whitespace())
            .map(|(i, _)| i)
            .unwrap_or(remaining.len());
        result.push(&remaining[..end]);
        remaining = remaining[end..].trim_start();
    }

    if !remaining.is_empty() {
        result.push(remaining);
    }

    result
}

impl Parser for IptablesParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(Vec::new()));
        }

        // Regex for: " (policy NAME packets, bytes)"
        let chain_pkt_byt_re = Regex::new(
            r"\s\(policy\s(?P<policy_name>.+)\s(?P<packets>.+)\spackets,\s(?P<bytes>.+)\sbytes\)",
        )
        .unwrap();

        let mut raw_output: Vec<Map<String, Value>> = Vec::new();
        let mut chain: Option<Map<String, Value>> = None;
        let mut headers: Vec<String> = Vec::new();

        for line in input.lines() {
            let line = if line.trim().is_empty() {
                continue;
            } else {
                line
            };

            if line.starts_with("Chain") {
                // Save previous chain
                if let Some(c) = chain.take() {
                    raw_output.push(c);
                }

                headers.clear();
                let mut new_chain: Map<String, Value> = Map::new();
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    new_chain.insert("chain".to_string(), Value::String(parts[1].to_string()));
                }

                // Try to parse policy, packets, bytes
                if let Some(caps) = chain_pkt_byt_re.captures(line) {
                    new_chain.insert(
                        "default_policy".to_string(),
                        Value::String(caps["policy_name"].to_string()),
                    );
                    new_chain.insert(
                        "default_packets".to_string(),
                        Value::String(caps["packets"].to_string()),
                    );
                    new_chain.insert(
                        "default_bytes".to_string(),
                        Value::String(caps["bytes"].to_string()),
                    );
                }

                new_chain.insert("rules".to_string(), Value::Array(Vec::new()));
                chain = Some(new_chain);
                continue;
            }

            // Header lines: starts with "target", "num", or has "pkts" in first few chars
            if headers.is_empty()
                && (line.starts_with("target")
                    || line.starts_with("num")
                    || (line.len() > 5 && line[..5].contains("pkts")))
            {
                let normalized = line.to_lowercase();
                let words: Vec<String> = normalized
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect();
                headers = words;
                headers.push("options".to_string());
                continue;
            }

            // Skip separator lines
            if line.starts_with("--") || line.trim().starts_with("--") {
                continue;
            }

            // Process rule lines
            if chain.is_some() && !headers.is_empty() {
                let mut rule_line = line.to_string();

                // A blank "target" column collapses under whitespace splitting
                // and shifts every later field left. jc stuffs an invisible
                // separator into the gap; the verbose (`-v`) layout needs the
                // same treatment, detected by the `opt` column landing where
                // `prot` should be.
                if !headers.is_empty() && headers[0] == "target" && line.starts_with(' ') {
                    rule_line = format!("\u{2063}{}", line);
                } else if headers.first().is_some_and(|h| *h == "pkts") {
                    let fields: Vec<&str> = line.split_whitespace().collect();
                    if fields
                        .get(3)
                        .is_some_and(|f| matches!(*f, "--" | "-f" | "!f"))
                    {
                        rule_line = format!(
                            "{} \u{2063} {}",
                            fields[..2].join(" "),
                            fields[2..].join(" ")
                        );
                    }
                }

                // Python-style split with maxsplit: split on whitespace, collapsing consecutive
                let parts: Vec<&str> = python_split_whitespace(&rule_line, headers.len());
                let mut rule: Map<String, Value> = Map::new();

                for (i, header) in headers.iter().enumerate() {
                    if let Some(val) = parts.get(i) {
                        let v = val.trim();
                        if header == "options" {
                            // jc preserves the raw options string including trailing whitespace
                            let raw_opt = parts.get(i).unwrap_or(&"");
                            if !raw_opt.is_empty() {
                                rule.insert(header.clone(), Value::String(raw_opt.to_string()));
                            }
                        } else {
                            rule.insert(header.clone(), Value::String(v.to_string()));
                        }
                    }
                }

                // Fix invisible char target
                if let Some(Value::String(t)) = rule.get("target") {
                    if t == "\u{2063}" {
                        rule.insert("target".to_string(), Value::String(String::new()));
                    }
                }

                // Process: convert types
                // num -> int
                if let Some(Value::String(s)) = rule.get("num").cloned() {
                    if let Ok(n) = s.trim().parse::<i64>() {
                        rule.insert("num".to_string(), Value::Number(n.into()));
                    }
                }
                // pkts -> int
                if let Some(Value::String(s)) = rule.get("pkts").cloned() {
                    if let Ok(n) = s.trim().parse::<i64>() {
                        rule.insert("pkts".to_string(), Value::Number(n.into()));
                    }
                }
                // bytes -> size int
                if let Some(Value::String(s)) = rule.get("bytes").cloned() {
                    let s = s.clone();
                    rule.insert("bytes".to_string(), convert_size_to_int(&s));
                }
                // opt "--" -> null
                if let Some(Value::String(s)) = rule.get("opt").cloned() {
                    if s == "--" {
                        rule.insert("opt".to_string(), Value::Null);
                    }
                }
                // target "" -> null
                if let Some(Value::String(s)) = rule.get("target").cloned() {
                    if s.is_empty() {
                        rule.insert("target".to_string(), Value::Null);
                    }
                }

                if let Some(c) = chain.as_mut() {
                    if let Some(Value::Array(rules)) = c.get_mut("rules") {
                        rules.push(Value::Object(rule));
                    }
                }
            }
        }

        // Save last chain
        if let Some(c) = chain.take() {
            raw_output.push(c);
        }

        // Process chain-level fields
        for chain_obj in raw_output.iter_mut() {
            if let Some(Value::String(s)) = chain_obj.get("default_packets").cloned() {
                if let Ok(n) = s.trim().parse::<i64>() {
                    chain_obj.insert("default_packets".to_string(), Value::Number(n.into()));
                }
            }
            if let Some(Value::String(s)) = chain_obj.get("default_bytes").cloned() {
                let s = s.clone();
                chain_obj.insert("default_bytes".to_string(), convert_size_to_int(&s));
            }
        }

        Ok(ParseOutput::Array(raw_output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iptables_no_jump() {
        let input = "Chain INPUT (policy ACCEPT)\ntarget     prot opt source               destination\n           udp  --  anywhere             anywhere\n";
        let parser = IptablesParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 1);
            assert_eq!(
                arr[0].get("chain"),
                Some(&Value::String("INPUT".to_string()))
            );
            let rules = arr[0].get("rules").unwrap();
            if let Value::Array(r) = rules {
                assert_eq!(r.len(), 1);
                if let Value::Object(rule) = &r[0] {
                    assert_eq!(rule.get("target"), Some(&Value::Null));
                    assert_eq!(rule.get("prot"), Some(&Value::String("udp".to_string())));
                    assert_eq!(rule.get("opt"), Some(&Value::Null));
                }
            }
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_iptables_empty() {
        let parser = IptablesParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 0);
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_iptables_with_policy() {
        let input = "Chain PREROUTING (policy DROP 0 packets, 0 bytes)\nnum   pkts bytes target     prot opt in     out     source               destination\n1     2183  186K PREROUTING_direct  all  --  any    any     anywhere             anywhere\n";
        let parser = IptablesParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 1);
            assert_eq!(
                arr[0].get("chain"),
                Some(&Value::String("PREROUTING".to_string()))
            );
            assert_eq!(
                arr[0].get("default_policy"),
                Some(&Value::String("DROP".to_string()))
            );
            assert_eq!(
                arr[0].get("default_packets"),
                Some(&Value::Number(0i64.into()))
            );
        } else {
            panic!("Expected Array");
        }
    }
}
