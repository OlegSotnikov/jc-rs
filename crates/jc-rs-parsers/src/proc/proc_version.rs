//! Parser for `/proc/version`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use regex::Regex;
use serde_json::{Map, Value};

pub struct ProcVersionParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_version",
    argument: "--proc-version",
    version: "1.0.0",
    description: "Converts `/proc/version` file to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &["/proc/version"],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static PROC_VERSION_PARSER: ProcVersionParser = ProcVersionParser;

inventory::submit! { ParserEntry::new(&PROC_VERSION_PARSER) }

impl Parser for ProcVersionParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let line = input.trim();
        if line.is_empty() {
            return Ok(ParseOutput::Object(Map::new()));
        }

        let re = Regex::new(
            r"(?x)
            Linux\s+version\s+(?P<version>\S+)\s
            \((?P<email>\S+?)\)\s
            \((?P<gcc>gcc.+)\)\s
            (?P<build>\#\d+\S*)\s
            (?P<flags>.*?)
            (?P<date>(?:Sun|Mon|Tue|Wed|Thu|Fri|Sat).+)",
        )
        .map_err(|e| ParseError::Regex(e.to_string()))?;

        let mut out = Map::new();

        if let Some(caps) = re.captures(line) {
            if let Some(m) = caps.name("version") {
                out.insert("version".to_string(), Value::String(m.as_str().to_string()));
            }
            if let Some(m) = caps.name("email") {
                out.insert("email".to_string(), Value::String(m.as_str().to_string()));
            }
            if let Some(m) = caps.name("gcc") {
                out.insert(
                    "gcc".to_string(),
                    Value::String(m.as_str().trim().to_string()),
                );
            }
            if let Some(m) = caps.name("build") {
                out.insert(
                    "build".to_string(),
                    Value::String(m.as_str().trim().to_string()),
                );
            }
            if let Some(m) = caps.name("flags") {
                let flags = m.as_str().trim();
                if flags.is_empty() {
                    out.insert("flags".to_string(), Value::Null);
                } else {
                    out.insert("flags".to_string(), Value::String(flags.to_string()));
                }
            }
            if let Some(m) = caps.name("date") {
                out.insert(
                    "date".to_string(),
                    Value::String(m.as_str().trim().to_string()),
                );
            }
        }

        Ok(ParseOutput::Object(out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_version() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/version");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/version.json"
        ))
        .unwrap();
        let parser = ProcVersionParser;
        let result = parser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }

    #[test]
    fn test_proc_version2() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/version2");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/version2.json"
        ))
        .unwrap();
        let parser = ProcVersionParser;
        let result = parser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }

    #[test]
    fn test_proc_version3() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/version3");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/version3.json"
        ))
        .unwrap();
        let parser = ProcVersionParser;
        let result = parser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}
