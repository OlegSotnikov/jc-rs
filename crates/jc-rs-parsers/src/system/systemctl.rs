//! Parser for `systemctl` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct SystemctlParser;

static INFO: ParserInfo = ParserInfo {
    name: "systemctl",
    argument: "--systemctl",
    version: "1.5.0",
    description: "Converts `systemctl` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &["systemctl"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static SYSTEMCTL_PARSER: SystemctlParser = SystemctlParser;

inventory::submit! {
    ParserEntry::new(&SYSTEMCTL_PARSER)
}

impl Parser for SystemctlParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let rows = parse_systemctl(input);
        Ok(ParseOutput::Array(rows))
    }
}

fn parse_systemctl(input: &str) -> Vec<Map<String, Value>> {
    let mut lines = input.lines().filter(|l| !l.trim().is_empty());

    let header_line = match lines.next() {
        Some(l) => l,
        None => return Vec::new(),
    };

    let header_list: Vec<String> = header_line
        .to_lowercase()
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();

    let mut output = Vec::new();

    for line in lines {
        // Stop at legend lines
        if line.contains("LOAD   = ") || line.contains("ACTIVE = ") {
            break;
        }

        // Strip bullet character (●) that appears before failed/not-found units
        let line = line.replace('●', " ");
        let line = line.as_str();
        // Split into at most 5 parts: unit, load, active, sub, description
        let parts: Vec<&str> = line
            .trim_end()
            .splitn(5, char::is_whitespace)
            .filter(|s| !s.is_empty())
            .collect();

        if parts.is_empty() {
            continue;
        }

        // Re-split properly: first 4 fields by whitespace, rest is description
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.is_empty() {
            continue;
        }

        // Build entry using up to header_list length fields
        // The last header field (description) gets the remainder
        let mut record = Map::new();
        let _ = parts; // suppress warning

        for (i, header) in header_list.iter().enumerate() {
            if i < header_list.len() - 1 {
                // Normal field
                if let Some(&val) = tokens.get(i) {
                    record.insert(header.clone(), Value::String(val.to_string()));
                }
            } else {
                // Last field: join remaining tokens
                if tokens.len() > i {
                    let desc = tokens[i..].join(" ");
                    record.insert(header.clone(), Value::String(desc));
                }
            }
        }

        output.push(record);
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_systemctl_basic() {
        let input = "UNIT                                LOAD   ACTIVE SUB       DESCRIPTION\n\
                     proc-sys-fs-binfmt_misc.automount   loaded active waiting   Arbitrary Formats\n\
                     dev-block-8:2.device                loaded active plugged   LVM PV\n\
                     \n\
                     LOAD   = Reflects whether the unit definition was properly loaded.";

        let parser = SystemctlParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 2);
            assert_eq!(
                arr[0].get("unit"),
                Some(&Value::String(
                    "proc-sys-fs-binfmt_misc.automount".to_string()
                ))
            );
            assert_eq!(
                arr[0].get("load"),
                Some(&Value::String("loaded".to_string()))
            );
            assert_eq!(
                arr[0].get("active"),
                Some(&Value::String("active".to_string()))
            );
            assert_eq!(
                arr[0].get("sub"),
                Some(&Value::String("waiting".to_string()))
            );
            assert_eq!(
                arr[0].get("description"),
                Some(&Value::String("Arbitrary Formats".to_string()))
            );
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_systemctl_empty() {
        let parser = SystemctlParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert!(arr.is_empty());
        } else {
            panic!("Expected Array");
        }
    }
}
