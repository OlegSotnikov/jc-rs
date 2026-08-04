//! INI file parser with duplicate key support.
//!
//! Like the ini parser, but duplicate keys become arrays of values.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use std::collections::BTreeMap;

pub struct IniDupParser;

static INI_DUP_INFO: ParserInfo = ParserInfo {
    name: "ini_dup",
    argument: "--ini-dup",
    version: "1.0.0",
    description: "INI file parser with duplicate key support",
    author: "jc-rs contributors",
    author_email: "jc-rs@example.com",
    compatible: &[Platform::Universal],
    tags: &[Tag::File],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

/// Remove outer quotes (single or double) from a value string.
fn remove_quotes(s: &str) -> String {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

/// Parse a line into (key, value) splitting on first `=` or `:`.
fn parse_kv_line(line: &str) -> Option<(String, String)> {
    let eq_pos = line.find('=');
    let colon_pos = line.find(':');

    let sep_pos = match (eq_pos, colon_pos) {
        (Some(e), Some(c)) => Some(e.min(c)),
        (Some(e), None) => Some(e),
        (None, Some(c)) => Some(c),
        (None, None) => None,
    };

    if let Some(pos) = sep_pos {
        let key = line[..pos].trim().to_string();
        let val = remove_quotes(line[pos + 1..].trim());
        Some((key, val))
    } else {
        let key = line.trim().to_string();
        if key.is_empty() {
            None
        } else {
            Some((key, String::new()))
        }
    }
}

/// Parsed ini with duplicate key lists.
struct ParsedIniDup {
    top_level: BTreeMap<String, Vec<String>>,
    sections: Vec<(String, BTreeMap<String, Vec<String>>)>,
}

fn parse_ini_dup_str(input: &str) -> Result<ParsedIniDup, ParseError> {
    let mut top_level: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut sections: Vec<(String, BTreeMap<String, Vec<String>>)> = Vec::new();
    let mut section_index: BTreeMap<String, usize> = BTreeMap::new();
    let mut current_section: Option<String> = None;

    for line in input.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let section_name = trimmed[1..trimmed.len() - 1].trim().to_string();
            current_section = Some(section_name.clone());

            section_index
                .entry(section_name.clone())
                .or_insert_with(|| {
                    let idx = sections.len();
                    sections.push((section_name.clone(), BTreeMap::new()));
                    idx
                });
            continue;
        }

        if let Some((key, val)) = parse_kv_line(trimmed) {
            match &current_section {
                None => {
                    top_level.entry(key).or_default().push(val);
                }
                Some(sec) => {
                    let idx = *section_index.get(sec).unwrap();
                    sections[idx].1.entry(key).or_default().push(val);
                }
            }
        }
    }

    Ok(ParsedIniDup {
        top_level,
        sections,
    })
}

fn ini_dup_to_json(parsed: &ParsedIniDup) -> serde_json::Map<String, serde_json::Value> {
    let mut map = serde_json::Map::new();

    for (k, values) in &parsed.top_level {
        let arr: Vec<serde_json::Value> = values
            .iter()
            .map(|v| serde_json::Value::String(v.clone()))
            .collect();
        map.insert(k.clone(), serde_json::Value::Array(arr));
    }

    // All sections (including DEFAULT) become nested objects with array values
    for (section_name, kv) in &parsed.sections {
        let mut section_map = serde_json::Map::new();
        for (k, values) in kv {
            let arr: Vec<serde_json::Value> = values
                .iter()
                .map(|v| serde_json::Value::String(v.clone()))
                .collect();
            section_map.insert(k.clone(), serde_json::Value::Array(arr));
        }
        map.insert(section_name.clone(), serde_json::Value::Object(section_map));
    }

    map
}

impl Parser for IniDupParser {
    fn info(&self) -> &'static ParserInfo {
        &INI_DUP_INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Err(ParseError::InvalidInput("empty input".to_string()));
        }
        let parsed = parse_ini_dup_str(input)?;
        let map = ini_dup_to_json(&parsed);
        Ok(ParseOutput::Object(map))
    }
}

static INI_DUP_PARSER_INSTANCE: IniDupParser = IniDupParser;

inventory::submit! {
    ParserEntry::new(&INI_DUP_PARSER_INSTANCE)
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
    fn test_ini_dup_test() {
        // Use the same ini-test.ini but check against ini-dup-test.json
        let input = load_fixture("ini-test.ini");
        let expected = parse_json_obj(&load_fixture("ini-dup-test.json"));
        let parser = IniDupParser;
        let result = parser.parse(&input, false).unwrap();
        if let ParseOutput::Object(map) = result {
            assert_eq!(map, expected);
        } else {
            panic!("expected Object output");
        }
    }

    #[test]
    fn test_ini_dup_iptelserver() {
        let input = load_fixture("ini-iptelserver.ini");
        let expected = parse_json_obj(&load_fixture("ini-dup-iptelserver.json"));
        let parser = IniDupParser;
        let result = parser.parse(&input, false).unwrap();
        if let ParseOutput::Object(map) = result {
            assert_eq!(map, expected);
        } else {
            panic!("expected Object output");
        }
    }

    #[test]
    fn test_ini_dup_mariadb() {
        let input = load_fixture("ini-mariadb.ini");
        let expected = parse_json_obj(&load_fixture("ini-dup-mariadb.json"));
        let parser = IniDupParser;
        let result = parser.parse(&input, false).unwrap();
        if let ParseOutput::Object(map) = result {
            assert_eq!(map, expected);
        } else {
            panic!("expected Object output");
        }
    }

    #[test]
    fn test_ini_dup_double_quote() {
        let input = load_fixture("ini-double-quote.ini");
        let expected = parse_json_obj(&load_fixture("ini-dup-double-quote.json"));
        let parser = IniDupParser;
        let result = parser.parse(&input, false).unwrap();
        if let ParseOutput::Object(map) = result {
            assert_eq!(map, expected);
        } else {
            panic!("expected Object output");
        }
    }

    #[test]
    fn test_ini_dup_single_quote() {
        let input = load_fixture("ini-single-quote.ini");
        let expected = parse_json_obj(&load_fixture("ini-dup-single-quote.json"));
        let parser = IniDupParser;
        let result = parser.parse(&input, false).unwrap();
        if let ParseOutput::Object(map) = result {
            assert_eq!(map, expected);
        } else {
            panic!("expected Object output");
        }
    }
}
