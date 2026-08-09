//! Parser for `route` command output (Linux/Windows).

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct RouteParser;

static INFO: ParserInfo = ParserInfo {
    name: "route",
    argument: "--route",
    version: "1.9.0",
    description: "Converts `route` command output to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &["route"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static ROUTE_PARSER: RouteParser = RouteParser;
inventory::submit! { ParserEntry::new(&ROUTE_PARSER) }

fn flag_to_pretty(flag: char) -> Option<&'static str> {
    match flag {
        'U' => Some("UP"),
        'H' => Some("HOST"),
        'G' => Some("GATEWAY"),
        'R' => Some("REINSTATE"),
        'D' => Some("DYNAMIC"),
        'M' => Some("MODIFIED"),
        'A' => Some("ADDRCONF"),
        'C' => Some("CACHE"),
        '!' => Some("REJECT"),
        _ => None,
    }
}

fn str_to_int(s: &str) -> Value {
    s.trim()
        .parse::<i64>()
        .map(|n| Value::Number(n.into()))
        .unwrap_or(Value::Null)
}

fn normalize_headers(header: &str) -> String {
    let mut h = header.to_string();
    let has_next_hop = h.contains("Next Hop");
    if has_next_hop {
        h = h.replace("Next Hop", "Next_Hop");
        // In IPv6, "If" column at end
        h = h.replace(" If", " Iface");
    }
    // Normalize abbreviated column names
    h = h.replace(" Flag ", " Flags ");
    if h.ends_with(" Flag") {
        h = h[..h.len() - 5].to_string() + " Flags";
    }
    h = h.replace(" Met ", " Metric ");
    if h.ends_with(" Met") {
        h = h[..h.len() - 4].to_string() + " Metric";
    }
    h.to_lowercase()
}

impl Parser for RouteParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let mut lines: Vec<&str> = input.lines().collect();

        // Skip "Kernel IP routing table" or "Kernel IPv6 routing table" header line
        if let Some(first) = lines.first()
            && first.contains("routing table")
        {
            lines.remove(0);
        }

        if lines.is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        // Parse header
        let header_line = lines.remove(0);
        let normalized = normalize_headers(header_line);
        let headers: Vec<&str> = normalized.split_whitespace().collect();

        if headers.is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let int_cols = ["metric", "ref", "use", "mss", "window", "irtt"];

        let mut result = Vec::new();
        for line in &lines {
            // jc's route reads the table without filtering blank lines, so a
            // trailing newline produces a row of nothing, which its own
            // fixtures record as `{}`. Skipping it here left us one record
            // short of every `route -6` fixture.
            let values: Vec<&str> = line.split_whitespace().collect();
            if values.is_empty() {
                result.push(Map::new());
                continue;
            }

            let mut obj = Map::new();

            for (i, header) in headers.iter().enumerate() {
                let val = values.get(i).copied().unwrap_or("").trim();
                if int_cols.contains(header) {
                    obj.insert(header.to_string(), str_to_int(val));
                } else {
                    obj.insert(header.to_string(), Value::String(val.to_string()));
                }
            }

            // Add flags_pretty
            if let Some(flags_val) = obj.get("flags")
                && let Some(flags_str) = flags_val.as_str()
            {
                let pretty: Vec<Value> = flags_str
                    .chars()
                    .filter_map(|c| flag_to_pretty(c).map(|s| Value::String(s.to_string())))
                    .collect();
                obj.insert("flags_pretty".to_string(), Value::Array(pretty));
            }

            result.push(obj);
        }

        Ok(ParseOutput::Array(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jc_rs_core::traits::Parser;

    #[test]
    fn test_route_centos_golden() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/route.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/route.json"
        ))
        .unwrap();
        let result = RouteParser.parse(input, false).unwrap();
        let actual = serde_json::to_value(result).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_route_centos_6n_golden() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/route-6-n.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/route-6-n.json"
        ))
        .unwrap();
        let result = RouteParser.parse(input, false).unwrap();
        let actual = serde_json::to_value(result).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_route_empty() {
        let result = RouteParser.parse("", false).unwrap();
        assert!(matches!(result, ParseOutput::Array(v) if v.is_empty()));
    }

    #[test]
    fn test_route_registered() {
        assert!(jc_rs_core::registry::find_parser("route").is_some());
    }
}
