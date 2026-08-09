//! Parser for `update-alternatives --get-selections` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct UpdateAltGsParser;

static INFO: ParserInfo = ParserInfo {
    name: "update_alt_gs",
    argument: "--update-alt-gs",
    version: "1.0.0",
    description: "Converts `update-alternatives --get-selections` command output to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &["update-alternatives --get-selections"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static UPDATE_ALT_GS_PARSER: UpdateAltGsParser = UpdateAltGsParser;

inventory::submit! {
    ParserEntry::new(&UPDATE_ALT_GS_PARSER)
}

impl Parser for UpdateAltGsParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let rows = parse_update_alt_gs(input);
        Ok(ParseOutput::Array(rows))
    }
}

fn parse_update_alt_gs(input: &str) -> Vec<Map<String, Value>> {
    let mut output = Vec::new();

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Split into at most 3 parts: name, status, current
        let parts: Vec<&str> = line
            .splitn(3, char::is_whitespace)
            .filter(|s| !s.is_empty())
            .collect();

        // Re-collect with whitespace splitting
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.len() < 3 {
            continue;
        }

        let _ = parts;
        let name = tokens[0].to_string();
        let status = tokens[1].to_string();
        let current = tokens[2..].join(" ");

        let mut record = Map::new();
        record.insert("name".to_string(), Value::String(name));
        record.insert("status".to_string(), Value::String(status));
        record.insert("current".to_string(), Value::String(current));

        output.push(record);
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_alt_gs_basic() {
        let input = "arptables               auto    /usr/sbin/arptables-nft\n\
                     awk                     auto    /usr/bin/gawk\n\
                     builtins.7.gz           auto    /usr/share/man/man7/bash-builtins.7.gz";

        let parser = UpdateAltGsParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 3);
            assert_eq!(
                arr[0].get("name"),
                Some(&Value::String("arptables".to_string()))
            );
            assert_eq!(
                arr[0].get("status"),
                Some(&Value::String("auto".to_string()))
            );
            assert_eq!(
                arr[0].get("current"),
                Some(&Value::String("/usr/sbin/arptables-nft".to_string()))
            );
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_update_alt_gs_empty() {
        let parser = UpdateAltGsParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert!(arr.is_empty());
        } else {
            panic!("Expected Array");
        }
    }
}
