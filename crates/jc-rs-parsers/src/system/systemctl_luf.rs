//! Parser for `systemctl list-unit-files` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct SystemctlLufParser;

static INFO: ParserInfo = ParserInfo {
    name: "systemctl_luf",
    argument: "--systemctl-luf",
    version: "1.5.0",
    description: "Converts `systemctl list-unit-files` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &["systemctl list-unit-files"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static SYSTEMCTL_LUF_PARSER: SystemctlLufParser = SystemctlLufParser;

inventory::submit! {
    ParserEntry::new(&SYSTEMCTL_LUF_PARSER)
}

impl Parser for SystemctlLufParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let rows = parse_systemctl_luf(input);
        Ok(ParseOutput::Array(rows))
    }
}

fn parse_systemctl_luf(input: &str) -> Vec<Map<String, Value>> {
    let mut lines = input.lines().filter(|l| !l.trim().is_empty());

    let header_line = match lines.next() {
        Some(l) => l,
        None => return Vec::new(),
    };

    // Normalize header: "UNIT FILE" -> "unit_file"
    let header_text = header_line.to_lowercase().replace("unit file", "unit_file");
    let header_list: Vec<String> = header_text
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();

    let mut output = Vec::new();

    for line in lines {
        if line.contains("unit files listed.") {
            break;
        }

        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.is_empty() {
            continue;
        }

        let mut record = Map::new();

        for (i, header) in header_list.iter().enumerate() {
            if i < header_list.len() - 1 {
                if let Some(&val) = tokens.get(i) {
                    record.insert(header.clone(), Value::String(val.to_string()));
                }
            } else {
                if tokens.len() > i {
                    let val = tokens[i..].join(" ");
                    record.insert(header.clone(), Value::String(val));
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
    fn test_systemctl_luf_basic() {
        let input = "UNIT FILE                              STATE\n\
                     proc-sys-fs-binfmt_misc.automount      static\n\
                     dev-hugepages.mount                    static\n\
                     \n\
                     2 unit files listed.";

        let parser = SystemctlLufParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 2);
            assert_eq!(
                arr[0].get("unit_file"),
                Some(&Value::String(
                    "proc-sys-fs-binfmt_misc.automount".to_string()
                ))
            );
            assert_eq!(
                arr[0].get("state"),
                Some(&Value::String("static".to_string()))
            );
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_systemctl_luf_empty() {
        let parser = SystemctlLufParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert!(arr.is_empty());
        } else {
            panic!("Expected Array");
        }
    }
}
