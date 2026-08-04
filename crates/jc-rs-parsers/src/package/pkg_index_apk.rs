//! Parser for Alpine Linux APK package index files.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use regex::Regex;
use serde_json::{Map, Value};
use std::sync::OnceLock;

pub struct PkgIndexApkParser;

static INFO: ParserInfo = ParserInfo {
    name: "pkg_index_apk",
    argument: "--pkg-index-apk",
    version: "1.0.0",
    description: "Alpine Linux Package Index file parser",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[
        Platform::Linux,
        Platform::Darwin,
        Platform::Windows,
        Platform::Aix,
        Platform::FreeBSD,
    ],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static PKG_INDEX_APK_PARSER: PkgIndexApkParser = PkgIndexApkParser;

inventory::submit! {
    ParserEntry::new(&PKG_INDEX_APK_PARSER)
}

fn maintainer_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^(.*) <(.*)>$").unwrap())
}

impl Parser for PkgIndexApkParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(Vec::new()));
        }

        let mut results: Vec<Map<String, Value>> = Vec::new();
        let mut raw_pkg: Map<String, Value> = Map::new();

        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() {
                if !raw_pkg.is_empty() {
                    let processed = convert_package(raw_pkg);
                    results.push(processed);
                    raw_pkg = Map::new();
                }
                continue;
            }

            if line.len() < 2 {
                continue;
            }

            let key = &line[..1];
            let value = if line.len() > 2 { line[2..].trim() } else { "" };
            raw_pkg.insert(key.to_string(), Value::String(value.to_string()));
        }

        if !raw_pkg.is_empty() {
            let processed = convert_package(raw_pkg);
            results.push(processed);
        }

        Ok(ParseOutput::Array(results))
    }
}

fn convert_package(raw: Map<String, Value>) -> Map<String, Value> {
    let key_map = [
        ("C", "checksum"),
        ("P", "package"),
        ("V", "version"),
        ("A", "architecture"),
        ("S", "package_size"),
        ("I", "installed_size"),
        ("T", "description"),
        ("U", "url"),
        ("L", "license"),
        ("o", "origin"),
        ("m", "maintainer"),
        ("t", "build_time"),
        ("c", "commit"),
        ("k", "provider_priority"),
        ("D", "dependencies"),
        ("p", "provides"),
        ("i", "install_if"),
    ];

    let int_keys = ["S", "I", "t", "k"];
    let split_keys = ["D", "p", "i"];

    let mut entry: Map<String, Value> = Map::new();

    for (raw_key, friendly_key) in &key_map {
        if let Some(Value::String(val)) = raw.get(*raw_key) {
            let val = val.clone();

            if int_keys.contains(raw_key) {
                if let Ok(n) = val.parse::<i64>() {
                    entry.insert(friendly_key.to_string(), Value::Number(n.into()));
                } else {
                    entry.insert(friendly_key.to_string(), Value::String(val));
                }
            } else if split_keys.contains(raw_key) {
                let parts: Vec<Value> = val
                    .split_whitespace()
                    .map(|s| Value::String(s.to_string()))
                    .collect();
                entry.insert(friendly_key.to_string(), Value::Array(parts));
            } else if *raw_key == "m" {
                // Parse maintainer "Name <email>"
                let maintainer = if let Some(caps) = maintainer_re().captures(&val) {
                    let mut m: Map<String, Value> = Map::new();
                    m.insert(
                        "name".to_string(),
                        Value::String(caps[1].trim().to_string()),
                    );
                    m.insert(
                        "email".to_string(),
                        Value::String(caps[2].trim().to_string()),
                    );
                    Value::Object(m)
                } else {
                    let mut m: Map<String, Value> = Map::new();
                    m.insert("name".to_string(), Value::String(val));
                    Value::Object(m)
                };
                entry.insert(friendly_key.to_string(), maintainer);
            } else {
                entry.insert(friendly_key.to_string(), Value::String(val));
            }
        }
    }

    entry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pkg_index_apk_fixture() {
        let fixture_out = include_str!("../../../../tests/fixtures/generic/pkg-index-apk.out");
        let fixture_json = include_str!("../../../../tests/fixtures/generic/pkg-index-apk.json");

        let parser = PkgIndexApkParser;
        let result = parser.parse(&fixture_out, false).unwrap();
        let expected: serde_json::Value =
            serde_json::from_str(&fixture_json).expect("invalid fixture JSON");

        let got = serde_json::to_value(&result).unwrap();
        assert_eq!(got, expected, "pkg_index_apk fixture mismatch");
    }
}
