//! Parser for `apt-get -sqq` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use regex::Regex;
use serde_json::{Map, Value};
use std::sync::OnceLock;

pub struct AptGetSqqParser;

static INFO: ParserInfo = ParserInfo {
    name: "apt_get_sqq",
    argument: "--apt-get-sqq",
    version: "1.0.0",
    description: "Converts `apt-get -sqq` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &["apt-get -sqq"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static APT_GET_SQQ_PARSER: AptGetSqqParser = AptGetSqqParser;

inventory::submit! {
    ParserEntry::new(&APT_GET_SQQ_PARSER)
}

fn line_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?P<operation>Inst|Conf|Remv)\s(?P<package>\S+)(?P<broken>\s+\[\S*?\])?\s\((?P<packages_pe>.*?)\[(?P<architecture>\w*)\]\)",
        )
        .unwrap()
    })
}

impl Parser for AptGetSqqParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(Vec::new()));
        }

        let re = line_re();
        let mut results: Vec<Map<String, Value>> = Vec::new();

        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if let Some(caps) = re.captures(line) {
                let operation_raw = caps.name("operation").map(|m| m.as_str()).unwrap_or("");
                let package = caps
                    .name("package")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .to_string();
                let architecture = caps
                    .name("architecture")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .to_string();

                let broken = caps.name("broken").map(|m| {
                    let s = m.as_str().trim();
                    // Strip surrounding [ ]
                    s[1..s.len() - 1].to_string()
                });

                let packages_pe = caps.name("packages_pe").map(|m| m.as_str()).unwrap_or("");
                let mut parts = packages_pe.splitn(2, ',');
                let proposed_pkg_ver = parts.next().map(|s| s.trim().to_string());
                let existing_pkg_ver = parts.next().map(|s| s.trim().to_string());

                // Map operation names
                let operation = match operation_raw {
                    "Inst" => "unpack",
                    "Conf" => "configure",
                    "Remv" => "remove",
                    _ => operation_raw,
                };

                let mut entry: Map<String, Value> = Map::new();
                entry.insert(
                    "operation".to_string(),
                    Value::String(operation.to_string()),
                );
                entry.insert("package".to_string(), Value::String(package));
                entry.insert(
                    "broken".to_string(),
                    broken.map(Value::String).unwrap_or(Value::Null),
                );
                entry.insert(
                    "proposed_pkg_ver".to_string(),
                    proposed_pkg_ver.map(Value::String).unwrap_or(Value::Null),
                );
                entry.insert(
                    "existing_pkg_ver".to_string(),
                    existing_pkg_ver.map(Value::String).unwrap_or(Value::Null),
                );
                entry.insert("architecture".to_string(), Value::String(architecture));

                results.push(entry);
            }
        }

        Ok(ParseOutput::Array(results))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apt_get_sqq_fixture() {
        let fixture_out =
            include_str!("../../../../tests/fixtures/generic/apt_get_sqq--sample.out");
        let fixture_json =
            include_str!("../../../../tests/fixtures/generic/apt_get_sqq--sample.json");

        let parser = AptGetSqqParser;
        let result = parser.parse(&fixture_out, false).unwrap();
        let expected: serde_json::Value =
            serde_json::from_str(&fixture_json).expect("invalid fixture JSON");

        let got = serde_json::to_value(&result).unwrap();
        assert_eq!(got, expected, "apt_get_sqq fixture mismatch");
    }
}
