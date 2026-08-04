//! Parser for `postconf -M` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::{convert_to_bool, convert_to_int, simple_table_parse};
use serde_json::{Map, Value};

pub struct PostconfParser;

static INFO: ParserInfo = ParserInfo {
    name: "postconf",
    argument: "--postconf",
    version: "1.0.0",
    description: "Converts `postconf -M` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &["postconf -M"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static POSTCONF_PARSER: PostconfParser = PostconfParser;

inventory::submit! {
    ParserEntry::new(&POSTCONF_PARSER)
}

impl Parser for PostconfParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(Vec::new()));
        }

        // Prepend a synthetic header row
        let header = "service_name service_type private unprivileged chroot wake_up_time process_limit command";
        let data_lines: Vec<&str> = input.lines().filter(|l| !l.trim().is_empty()).collect();
        let mut all_lines = vec![header.to_string()];
        all_lines.extend(data_lines.iter().map(|l| l.to_string()));
        let table_str = all_lines.join("\n");

        let raw = simple_table_parse(&table_str);

        let processed: Vec<Map<String, Value>> = raw
            .into_iter()
            .map(|row| {
                let mut entry: Map<String, Value> = row.into_iter().collect();
                process_entry(&mut entry);
                entry
            })
            .collect();

        Ok(ParseOutput::Array(processed))
    }
}

fn process_entry(entry: &mut Map<String, Value>) {
    // Handle no_wake_up_before_first_use from wake_up_time
    let no_wake = {
        if let Some(Value::String(wut)) = entry.get("wake_up_time") {
            if wut.ends_with('?') {
                Some(Value::Bool(true))
            } else if wut == "-" {
                Some(Value::Null)
            } else {
                Some(Value::Bool(false))
            }
        } else {
            Some(Value::Null)
        }
    };
    if let Some(v) = no_wake {
        entry.insert("no_wake_up_before_first_use".to_string(), v);
    }

    // Convert dash to null, then booleans/integers
    let bool_fields = ["private", "unprivileged", "chroot"];
    let int_fields = ["wake_up_time", "process_limit"];

    for field in &bool_fields {
        let val = entry.get(*field).cloned();
        match val {
            Some(Value::String(s)) if s == "-" => {
                entry.insert(field.to_string(), Value::Null);
            }
            Some(Value::String(s)) => {
                let b = convert_to_bool(&s);
                entry.insert(field.to_string(), b.map(Value::Bool).unwrap_or(Value::Null));
            }
            _ => {}
        }
    }

    for field in &int_fields {
        let val = entry.get(*field).cloned();
        match val {
            Some(Value::String(s)) if s == "-" => {
                entry.insert(field.to_string(), Value::Null);
            }
            Some(Value::String(s)) => {
                // Strip trailing '?' for wake_up_time
                let clean = s.trim_end_matches('?');
                let n = convert_to_int(clean);
                entry.insert(
                    field.to_string(),
                    n.map(|v| Value::Number(v.into())).unwrap_or(Value::Null),
                );
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_postconf_fixture() {
        let fixture_out = include_str!("../../../../tests/fixtures/generic/postconf-M.out");
        let fixture_json = include_str!("../../../../tests/fixtures/generic/postconf-M.json");

        let parser = PostconfParser;
        let result = parser.parse(fixture_out, false).unwrap();
        let expected: serde_json::Value =
            serde_json::from_str(fixture_json).expect("invalid fixture JSON");

        let got = serde_json::to_value(&result).unwrap();
        assert_eq!(got, expected, "postconf fixture mismatch");
    }
}
