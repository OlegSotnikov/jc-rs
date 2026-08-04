//! Parser for `/proc/uptime`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ProcUptimeParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_uptime",
    argument: "--proc-uptime",
    version: "1.0.0",
    description: "Converts `/proc/uptime` file to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &["/proc/uptime"],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static PROC_UPTIME_PARSER: ProcUptimeParser = ProcUptimeParser;

inventory::submit! { ParserEntry::new(&PROC_UPTIME_PARSER) }

impl Parser for ProcUptimeParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let line = input.trim();
        if line.is_empty() {
            return Ok(ParseOutput::Object(Map::new()));
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            return Err(ParseError::InvalidInput(
                "Expected 2 fields in /proc/uptime".to_string(),
            ));
        }

        let mut out = Map::new();

        if let Ok(v) = parts[0].parse::<f64>() {
            out.insert(
                "up_time".to_string(),
                Value::Number(serde_json::Number::from_f64(v).unwrap()),
            );
        }
        if let Ok(v) = parts[1].parse::<f64>() {
            out.insert(
                "idle_time".to_string(),
                Value::Number(serde_json::Number::from_f64(v).unwrap()),
            );
        }

        Ok(ParseOutput::Object(out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_uptime() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/uptime");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/uptime.json"
        ))
        .unwrap();
        let parser = ProcUptimeParser;
        let result = parser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}
