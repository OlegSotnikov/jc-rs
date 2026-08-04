//! Parser for `lsb_release` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct LsbReleaseParser;

static INFO: ParserInfo = ParserInfo {
    name: "lsb_release",
    argument: "--lsb-release",
    version: "1.2.0",
    description: "Converts `lsb_release` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux, Platform::Darwin, Platform::FreeBSD],
    tags: &[Tag::Command],
    magic_commands: &["lsb_release"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static LSB_RELEASE_PARSER: LsbReleaseParser = LsbReleaseParser;

inventory::submit! {
    ParserEntry::new(&LSB_RELEASE_PARSER)
}

impl Parser for LsbReleaseParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let mut out = Map::new();

        if input.trim().is_empty() {
            return Ok(ParseOutput::Object(out));
        }

        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Split on first ':'
            if let Some(colon_pos) = line.find(':') {
                let key = line[..colon_pos].trim().to_string();
                let val = line[colon_pos + 1..].trim().to_string();
                out.insert(key, Value::String(val));
            }
        }

        Ok(ParseOutput::Object(out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lsb_release_basic() {
        let input = "Distributor ID:\tUbuntu\nDescription:\tUbuntu 16.04.6 LTS\nRelease:\t16.04\nCodename:\txenial\n";
        let parser = LsbReleaseParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Object(obj) = result {
            assert_eq!(
                obj.get("Distributor ID"),
                Some(&Value::String("Ubuntu".to_string()))
            );
            assert_eq!(
                obj.get("Description"),
                Some(&Value::String("Ubuntu 16.04.6 LTS".to_string()))
            );
            assert_eq!(
                obj.get("Release"),
                Some(&Value::String("16.04".to_string()))
            );
            assert_eq!(
                obj.get("Codename"),
                Some(&Value::String("xenial".to_string()))
            );
        } else {
            panic!("Expected Object");
        }
    }

    #[test]
    fn test_lsb_release_empty() {
        let parser = LsbReleaseParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Object(obj) = result {
            assert!(obj.is_empty());
        } else {
            panic!("Expected Object");
        }
    }
}
