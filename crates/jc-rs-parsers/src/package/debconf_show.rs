//! Parser for `debconf-show` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct DebconfShowParser;

static INFO: ParserInfo = ParserInfo {
    name: "debconf_show",
    argument: "--debconf-show",
    version: "1.0.0",
    description: "Converts `debconf-show` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &["debconf-show"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static DEBCONF_SHOW_PARSER: DebconfShowParser = DebconfShowParser;

inventory::submit! {
    ParserEntry::new(&DEBCONF_SHOW_PARSER)
}

impl Parser for DebconfShowParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(Vec::new()));
        }

        let mut results: Vec<Map<String, Value>> = Vec::new();

        for line in input.lines() {
            let line = line.trim_end();
            if line.trim().is_empty() {
                continue;
            }

            // Format: "* pkg/key: value" or "  pkg/key: value"
            // asked is true when line starts with "*"
            let asked = line.starts_with('*');

            // Find the colon that separates key from value
            let colon_pos = match line.find(':') {
                Some(p) => p,
                None => continue,
            };

            let key_part = &line[..colon_pos];
            let value = line[colon_pos + 1..].trim().to_string();

            // key_part is like "* onlyoffice/jwt-secret" or "  onlyoffice/db-pwd"
            // Strip the leading "* " or "  "
            let pkg_key = key_part.trim_start_matches('*').trim();

            // Split on '/' to get package name and key
            let slash_pos = match pkg_key.find('/') {
                Some(p) => p,
                None => continue,
            };

            let packagename = pkg_key[..slash_pos].to_string();
            let name = pkg_key[slash_pos + 1..].replace('-', "_");

            let mut entry: Map<String, Value> = Map::new();
            entry.insert("asked".to_string(), Value::Bool(asked));
            entry.insert("packagename".to_string(), Value::String(packagename));
            entry.insert("name".to_string(), Value::String(name));
            entry.insert("value".to_string(), Value::String(value));

            results.push(entry);
        }

        Ok(ParseOutput::Array(results))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debconf_show_fixture() {
        let fixture_out = include_str!("../../../../tests/fixtures/generic/debconf-show.out");
        let fixture_json = include_str!("../../../../tests/fixtures/generic/debconf-show.json");

        let parser = DebconfShowParser;
        let result = parser.parse(&fixture_out, false).unwrap();
        let expected: serde_json::Value =
            serde_json::from_str(&fixture_json).expect("invalid fixture JSON");

        let got = serde_json::to_value(&result).unwrap();
        assert_eq!(got, expected, "debconf_show fixture mismatch");
    }
}
