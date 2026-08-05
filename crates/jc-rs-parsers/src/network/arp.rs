//! Parser for `arp` command output.
//!
//! Supports Linux-style (with header "Address HWtype ...") and
//! BSD/macOS-style (`arp -a`) output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ArpParser;

static INFO: ParserInfo = ParserInfo {
    name: "arp",
    argument: "--arp",
    version: "1.8.0",
    description: "Converts `arp` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux, Platform::Darwin, Platform::FreeBSD],
    tags: &[Tag::Command, Tag::Slurpable],
    magic_commands: &["arp"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static ARP_PARSER: ArpParser = ArpParser;

inventory::submit! { ParserEntry::new(&ARP_PARSER) }

impl Parser for ArpParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let mut cleandata: Vec<&str> = input.lines().filter(|l| !l.trim().is_empty()).collect();

        // Remove trailing "Entries: N" line if present (arp -v)
        if let Some(last) = cleandata.last()
            && last.starts_with("Entries:")
        {
            cleandata.pop();
        }

        if cleandata.is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let first = cleandata[0];

        // Linux style: starts with "Address" header
        if first.starts_with("Address") {
            return parse_linux_style(&cleandata);
        }

        // BSD/macOS -a style: lines end with ']'  e.g. "[ethernet]"
        if first.trim_end().ends_with(']') {
            return parse_bsd_a_style(&cleandata);
        }

        // BSD style without -a: lines like "hostname (ip) at hwaddr [type] on iface"
        // but we also handle generic BSD table style
        parse_bsd_style(&cleandata)
    }
}

/// Parse Linux `arp` output (tabular with header row).
fn parse_linux_style(lines: &[&str]) -> Result<ParseOutput, ParseError> {
    // Fix the header: "Flags Mask" -> "flags_mask", lowercase everything
    let header_fixed = lines[0].replace("Flags Mask", "flags_mask").to_lowercase();
    let headers: Vec<&str> = header_fixed.split_whitespace().collect();

    let mut result = Vec::new();
    for line in &lines[1..] {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.is_empty() {
            continue;
        }
        let mut obj = Map::new();
        for (i, header) in headers.iter().enumerate() {
            let val = cols.get(i).copied().unwrap_or("").to_string();
            obj.insert(header.to_string(), Value::String(val));
        }
        result.push(obj);
    }

    Ok(ParseOutput::Array(result))
}

/// Parse BSD/macOS `arp -a` style.
/// Format: `name (ip) at hwaddr on iface ifscope [hwtype]`
///      or `? (ip) at hwaddr on iface ifscope permanent [hwtype]`
fn parse_bsd_a_style(lines: &[&str]) -> Result<ParseOutput, ParseError> {
    let mut result = Vec::new();

    for line in lines {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 6 {
            continue;
        }

        let name_raw = parts[0];
        let name: Value = if name_raw == "?" {
            Value::Null
        } else {
            Value::String(name_raw.to_string())
        };

        let address = parts[1]
            .trim_start_matches('(')
            .trim_end_matches(')')
            .to_string();
        // parts[2] == "at"
        let hwaddress = parts[3].to_string();
        // parts[4] == "on"
        let iface = parts[5].to_string();

        let permanent = parts.contains(&"permanent");

        let hwtype = parts
            .last()
            .map(|t| t.trim_start_matches('[').trim_end_matches(']').to_string())
            .unwrap_or_default();

        let mut obj = Map::new();
        obj.insert("name".to_string(), name);
        obj.insert("address".to_string(), Value::String(address));
        obj.insert("hwtype".to_string(), Value::String(hwtype));
        obj.insert("hwaddress".to_string(), Value::String(hwaddress));
        obj.insert("iface".to_string(), Value::String(iface));
        obj.insert("permanent".to_string(), Value::Bool(permanent));

        // Check for "expires in N seconds" field
        if let Some(exp_idx) = parts.iter().position(|&p| p == "expires") {
            // "expires in 942 seconds": the number is 2 positions after "expires"
            if let Some(exp_val) = parts.get(exp_idx + 2)
                && let Ok(n) = exp_val.parse::<i64>()
            {
                obj.insert("expires".to_string(), Value::Number(n.into()));
            }
        }

        result.push(obj);
    }

    Ok(ParseOutput::Array(result))
}

/// Parse old BSD-style `arp` output without -a flag.
fn parse_bsd_style(lines: &[&str]) -> Result<ParseOutput, ParseError> {
    let mut result = Vec::new();

    for line in lines {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        // Skip bucket / "There are N entries" lines (AIX)
        if parts[0].contains("bucket:") {
            continue;
        }
        if parts.len() >= 2 && parts[0] == "There" && parts[1] == "are" {
            continue;
        }

        let mut obj = Map::new();
        let name_raw = parts[0];
        let name: Value = if name_raw == "?" {
            Value::Null
        } else {
            Value::String(name_raw.to_string())
        };

        let address = if parts.len() > 1 {
            parts[1]
                .trim_start_matches('(')
                .trim_end_matches(')')
                .to_string()
        } else {
            String::new()
        };

        obj.insert("name".to_string(), name);
        obj.insert("address".to_string(), Value::String(address));

        // Check for (incomplete)
        if parts.contains(&"(incomplete)") || parts.contains(&"<incomplete>") {
            obj.insert("hwtype".to_string(), Value::Null);
            obj.insert("hwaddress".to_string(), Value::Null);
            if parts.len() >= 6 {
                obj.insert("iface".to_string(), Value::String(parts[5].to_string()));
            }
        } else if parts.len() >= 5 {
            let hwaddress = parts[3].to_string();
            let hwtype = parts[4]
                .trim_start_matches('[')
                .trim_end_matches(']')
                .to_string();
            obj.insert("hwtype".to_string(), Value::String(hwtype));
            obj.insert("hwaddress".to_string(), Value::String(hwaddress));

            if parts.contains(&"permanent") {
                obj.insert("permanent".to_string(), Value::Bool(true));
            } else if parts.len() >= 7 && !parts[6].starts_with("in") {
                obj.insert("iface".to_string(), Value::String(parts[6].to_string()));
            }
        }

        result.push(obj);
    }

    Ok(ParseOutput::Array(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use jc_rs_core::traits::Parser;

    #[test]
    fn test_arp_centos_golden() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/arp.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/arp.json"
        ))
        .unwrap();
        let result = ArpParser.parse(input, false).unwrap();
        let actual = serde_json::to_value(result).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_arp_centos_a_golden() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/arp-a.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/arp-a.json"
        ))
        .unwrap();
        let result = ArpParser.parse(input, false).unwrap();
        let actual = serde_json::to_value(result).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_arp_empty() {
        let result = ArpParser.parse("", false).unwrap();
        assert!(matches!(result, ParseOutput::Array(v) if v.is_empty()));
    }

    #[test]
    fn test_arp_registered() {
        assert!(jc_rs_core::registry::find_parser("arp").is_some());
    }
}
