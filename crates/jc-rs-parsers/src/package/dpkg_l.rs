//! Parser for `dpkg -l` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::simple_table_parse;
use serde_json::{Map, Value};

pub struct DpkgLParser;

static INFO: ParserInfo = ParserInfo {
    name: "dpkg_l",
    argument: "--dpkg-l",
    version: "1.0.0",
    description: "Converts `dpkg -l` command output to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &["dpkg -l"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static DPKG_L_PARSER: DpkgLParser = DpkgLParser;

inventory::submit! {
    ParserEntry::new(&DPKG_L_PARSER)
}

impl Parser for DpkgLParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(Vec::new()));
        }

        let mut working_lines: Vec<String> = Vec::new();
        let mut header_found = false;

        for line in input.lines() {
            if line.contains("Architecture") {
                header_found = true;
                // Replace the "||/" prefix with "codes" in header
                let normalized = line.to_lowercase().replace("||/", "codes");
                working_lines.push(normalized);
                continue;
            }

            if line.contains("=========") {
                continue;
            }

            if header_found {
                working_lines.push(line.to_string());
            }
        }

        if working_lines.is_empty() {
            return Ok(ParseOutput::Array(Vec::new()));
        }

        // Build a single string for simple_table_parse
        let table_str = working_lines.join("\n");
        let raw: Vec<std::collections::HashMap<String, Value>> = simple_table_parse(&table_str);

        // Convert to Map and process status codes
        let processed: Vec<Map<String, Value>> = raw
            .into_iter()
            .map(|row| {
                let mut entry: Map<String, Value> = row.into_iter().collect();
                process_codes(&mut entry);
                entry
            })
            .collect();

        Ok(ParseOutput::Array(processed))
    }
}

fn process_codes(entry: &mut Map<String, Value>) {
    let desired_map = [
        ('u', "unknown"),
        ('i', "install"),
        ('r', "remove"),
        ('p', "purge"),
        ('h', "hold"),
    ];

    let status_map = [
        ('n', "not installed"),
        ('i', "installed"),
        ('c', "config-files"),
        ('u', "unpacked"),
        ('f', "failed config"),
        ('h', "half installed"),
        ('w', "trigger await"),
        ('t', "trigger pending"),
    ];

    let err_map = [('r', "reinstall required")];

    if let Some(Value::String(codes)) = entry.get("codes") {
        let codes_lower = codes.to_lowercase();
        let mut chars = codes_lower.chars();

        if let Some(desired_ch) = chars.next()
            && let Some((_, val)) = desired_map.iter().find(|(k, _)| *k == desired_ch)
        {
            entry.insert("desired".to_string(), Value::String(val.to_string()));
        }

        if let Some(status_ch) = chars.next()
            && let Some((_, val)) = status_map.iter().find(|(k, _)| *k == status_ch)
        {
            entry.insert("status".to_string(), Value::String(val.to_string()));
        }

        if let Some(err_ch) = chars.next()
            && let Some((_, val)) = err_map.iter().find(|(k, _)| *k == err_ch)
        {
            entry.insert("error".to_string(), Value::String(val.to_string()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dpkg_l_smoke() {
        let input = "Desired=Unknown/Install/Remove/Purge/Hold\n| Status=Not/Inst/Conf-files/Unpacked/halF-conf/Half-inst/trig-aWait/Trig-pend\n|/ Err?=(none)/Reinst-required\n||/ Name         Version      Architecture Description\n+++-============-============-============-===========\nii  foo          1.0          amd64        A test pkg\n";
        let parser = DpkgLParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 1);
            assert_eq!(arr[0].get("name"), Some(&Value::String("foo".into())));
            assert_eq!(
                arr[0].get("desired"),
                Some(&Value::String("install".into()))
            );
            assert_eq!(
                arr[0].get("status"),
                Some(&Value::String("installed".into()))
            );
        } else {
            panic!("Expected Array");
        }
    }
}
