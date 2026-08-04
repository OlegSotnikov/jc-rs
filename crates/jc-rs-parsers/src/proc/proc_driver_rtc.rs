//! Parser for `/proc/driver/rtc`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ProcDriverRtcParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_driver_rtc",
    argument: "--proc-driver-rtc",
    version: "1.0.0",
    description: "Converts `/proc/driver/rtc` file to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &["/proc/driver/rtc"],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static PROC_DRIVER_RTC_PARSER: ProcDriverRtcParser = ProcDriverRtcParser;
inventory::submit! { ParserEntry::new(&PROC_DRIVER_RTC_PARSER) }

impl Parser for ProcDriverRtcParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Object(Map::new()));
        }

        let mut out: Map<String, Value> = Map::new();

        for line in input.lines() {
            if line.trim().is_empty() {
                continue;
            }

            // Format: "key\t: value" (tab-colon format)
            let Some(colon_pos) = line.find(':') else {
                continue;
            };

            let key = line[..colon_pos].trim().to_string();
            let val = line[colon_pos + 1..].trim().to_string();

            // Convert value: "yes" -> true, "no" -> false, try int, otherwise string
            let json_val = if val == "yes" {
                Value::Bool(true)
            } else if val == "no" {
                Value::Bool(false)
            } else if let Ok(i) = val.parse::<i64>() {
                Value::Number(i.into())
            } else {
                Value::String(val)
            };

            out.insert(key, json_val);
        }

        Ok(ParseOutput::Object(out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_driver_rtc() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/driver_rtc");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/driver_rtc.json"
        ))
        .unwrap();
        let parser = ProcDriverRtcParser;
        let result = parser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}
