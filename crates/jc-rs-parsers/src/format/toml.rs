//! TOML file parser.
//!
//! Parses TOML documents into JSON. Datetime values are converted to
//! Unix timestamps with an additional _iso key for the ISO string.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};

pub struct TomlParser;

static TOML_INFO: ParserInfo = ParserInfo {
    name: "toml",
    argument: "--toml",
    version: "1.0.0",
    description: "TOML file parser",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Universal],
    tags: &[Tag::File],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

/// Convert a toml::Value into a serde_json::Value.
/// Datetime values are converted to Unix timestamps + _iso string.
fn toml_val_to_json(
    key: &str,
    val: toml::Value,
    parent: &mut serde_json::Map<String, serde_json::Value>,
) {
    match val {
        toml::Value::String(s) => {
            parent.insert(key.to_string(), serde_json::Value::String(s));
        }
        toml::Value::Integer(i) => {
            parent.insert(key.to_string(), serde_json::Value::Number(i.into()));
        }
        toml::Value::Float(f) => {
            let n = serde_json::Number::from_f64(f).unwrap_or_else(|| serde_json::Number::from(0));
            parent.insert(key.to_string(), serde_json::Value::Number(n));
        }
        toml::Value::Boolean(b) => {
            parent.insert(key.to_string(), serde_json::Value::Bool(b));
        }
        toml::Value::Datetime(dt) => {
            let iso = dt.to_string();
            // Parse the datetime to get a Unix timestamp
            let ts = parse_toml_datetime_to_unix(&iso);
            parent.insert(key.to_string(), serde_json::Value::Number(ts.into()));
            parent.insert(format!("{key}_iso"), serde_json::Value::String(iso));
        }
        toml::Value::Array(arr) => {
            let json_arr: Vec<serde_json::Value> =
                arr.into_iter().map(toml_val_to_json_value).collect();
            parent.insert(key.to_string(), serde_json::Value::Array(json_arr));
        }
        toml::Value::Table(table) => {
            let mut sub_map = serde_json::Map::new();
            for (k, v) in table {
                toml_val_to_json(&k, v, &mut sub_map);
            }
            parent.insert(key.to_string(), serde_json::Value::Object(sub_map));
        }
    }
}

fn toml_val_to_json_value(val: toml::Value) -> serde_json::Value {
    match val {
        toml::Value::String(s) => serde_json::Value::String(s),
        toml::Value::Integer(i) => serde_json::Value::Number(i.into()),
        toml::Value::Float(f) => serde_json::Number::from_f64(f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        toml::Value::Boolean(b) => serde_json::Value::Bool(b),
        toml::Value::Datetime(dt) => {
            // In arrays, keep as string (jc behavior: arrays keep ISO strings,
            // top-level datetimes become timestamps)
            serde_json::Value::String(dt.to_string())
        }
        toml::Value::Array(arr) => {
            serde_json::Value::Array(arr.into_iter().map(toml_val_to_json_value).collect())
        }
        toml::Value::Table(table) => {
            let mut map = serde_json::Map::new();
            for (k, v) in table {
                toml_val_to_json(&k, v, &mut map);
            }
            serde_json::Value::Object(map)
        }
    }
}

/// Parse a TOML datetime string to a Unix timestamp (i64).
/// Handles offset datetimes and naive datetimes.
fn parse_toml_datetime_to_unix(iso: &str) -> i64 {
    use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};

    // Try offset datetime first (e.g. 1979-05-27T07:32:00-08:00)
    if let Ok(dt) = DateTime::parse_from_rfc3339(iso) {
        return dt.timestamp();
    }

    // Try with various formats
    let formats = [
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M:%SZ",
        "%Y-%m-%d",
    ];

    for fmt in &formats {
        if let Ok(ndt) = NaiveDateTime::parse_from_str(iso, fmt) {
            return Utc.from_utc_datetime(&ndt).timestamp();
        }
    }

    0
}

impl Parser for TomlParser {
    fn info(&self) -> &'static ParserInfo {
        &TOML_INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Err(ParseError::InvalidInput("empty input".to_string()));
        }

        let table: toml::Table = toml::from_str(input)
            .map_err(|e| ParseError::Generic(format!("TOML parse error: {e}")))?;

        let mut map = serde_json::Map::new();
        for (k, v) in table {
            toml_val_to_json(&k, v, &mut map);
        }

        Ok(ParseOutput::Object(map))
    }
}

static TOML_PARSER_INSTANCE: TomlParser = TomlParser;

inventory::submit! {
    ParserEntry::new(&TOML_PARSER_INSTANCE)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/fixtures/generic");

    fn load_fixture(name: &str) -> String {
        std::fs::read_to_string(format!("{FIXTURE_DIR}/{name}"))
            .unwrap_or_else(|e| panic!("failed to read fixture {name}: {e}"))
    }

    fn parse_json_obj(s: &str) -> serde_json::Map<String, serde_json::Value> {
        serde_json::from_str(s).expect("invalid fixture JSON")
    }

    #[test]
    fn test_toml_example() {
        let input = load_fixture("toml-example.toml");
        let expected = parse_json_obj(&load_fixture("toml-example.json"));
        let parser = TomlParser;
        let result = parser.parse(&input, false).unwrap();
        if let ParseOutput::Object(map) = result {
            assert_eq!(map, expected);
        } else {
            panic!("expected Object output");
        }
    }

    #[test]
    fn test_toml_example2() {
        let input = load_fixture("toml-example2.toml");
        let expected = parse_json_obj(&load_fixture("toml-example2.json"));
        let parser = TomlParser;
        let result = parser.parse(&input, false).unwrap();
        if let ParseOutput::Object(map) = result {
            assert_eq!(map, expected);
        } else {
            panic!("expected Object output");
        }
    }
}
