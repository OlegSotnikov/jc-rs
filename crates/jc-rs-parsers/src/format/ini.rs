//! INI file parser.
//!
//! Parses standard INI files into a nested JSON object.
//! Sections become top-level keys with nested key/value objects.
//! Top-level (no-section) keys and DEFAULT section appear at the top level.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use std::collections::BTreeMap;

pub struct IniParser;

static INI_INFO: ParserInfo = ParserInfo {
    name: "ini",
    argument: "--ini",
    version: "1.0.0",
    description: "INI file parser",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
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
    // Find the first `=` or `:`
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
        // Key with no value (like `skip_external_locking`)
        let key = line.trim().to_string();
        if key.is_empty() {
            None
        } else {
            Some((key, String::new()))
        }
    }
}

/// Parsed representation: sections map to (key → last_value).
pub struct ParsedIni {
    /// Top-level keys (no section / DEFAULT section merged)
    pub top_level: BTreeMap<String, String>,
    /// Sections in insertion order. We use a Vec to preserve order.
    pub sections: Vec<(String, BTreeMap<String, String>)>,
}

pub fn parse_ini_str(input: &str) -> Result<ParsedIni, ParseError> {
    let mut top_level: BTreeMap<String, String> = BTreeMap::new();
    let mut sections: Vec<(String, BTreeMap<String, String>)> = Vec::new();
    let mut current_section: Option<String> = None;

    // Track section index for fast lookup
    let mut section_index: BTreeMap<String, usize> = BTreeMap::new();

    for line in input.lines() {
        let trimmed = line.trim();

        // Skip blank lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }

        // Section header
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

        // Key=value line
        if let Some((key, val)) = parse_kv_line(trimmed) {
            match &current_section {
                None => {
                    top_level.insert(key, val);
                }
                Some(sec) => {
                    let idx = *section_index.get(sec).unwrap();
                    sections[idx].1.insert(key, val);
                }
            }
        }
    }

    Ok(ParsedIni {
        top_level,
        sections,
    })
}

/// Convert ParsedIni into a serde_json Map, matching jc's schema.
/// All sections (including DEFAULT) become nested objects.
/// Top-level (no-section) keys appear directly in the output map.
pub fn ini_to_json(parsed: &ParsedIni) -> serde_json::Map<String, serde_json::Value> {
    let mut map = serde_json::Map::new();

    // Top-level (no-section) keys
    for (k, v) in &parsed.top_level {
        map.insert(k.clone(), serde_json::Value::String(v.clone()));
    }

    // Sections: all of them (including DEFAULT) become nested objects
    for (section_name, kv) in &parsed.sections {
        let mut section_map = serde_json::Map::new();
        for (k, v) in kv {
            section_map.insert(k.clone(), serde_json::Value::String(v.clone()));
        }
        map.insert(section_name.clone(), serde_json::Value::Object(section_map));
    }

    map
}

impl Parser for IniParser {
    fn info(&self) -> &'static ParserInfo {
        &INI_INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Err(ParseError::InvalidInput("empty input".to_string()));
        }
        let parsed = parse_ini_str(input)?;
        let map = ini_to_json(&parsed);
        Ok(ParseOutput::Object(map))
    }
}

static INI_PARSER_INSTANCE: IniParser = IniParser;

inventory::submit! {
    ParserEntry::new(&INI_PARSER_INSTANCE)
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
    fn test_ini_test() {
        let input = load_fixture("ini-test.ini");
        let expected = parse_json_obj(&load_fixture("ini-test.json"));
        let parser = IniParser;
        let result = parser.parse(&input, false).unwrap();
        if let ParseOutput::Object(map) = result {
            assert_eq!(map, expected);
        } else {
            panic!("expected Object output");
        }
    }

    #[test]
    fn test_ini_iptelserver() {
        let input = load_fixture("ini-iptelserver.ini");
        let expected = parse_json_obj(&load_fixture("ini-iptelserver.json"));
        let parser = IniParser;
        let result = parser.parse(&input, false).unwrap();
        if let ParseOutput::Object(map) = result {
            assert_eq!(map, expected);
        } else {
            panic!("expected Object output");
        }
    }

    #[test]
    fn test_ini_mariadb() {
        let input = load_fixture("ini-mariadb.ini");
        let expected = parse_json_obj(&load_fixture("ini-mariadb.json"));
        let parser = IniParser;
        let result = parser.parse(&input, false).unwrap();
        if let ParseOutput::Object(map) = result {
            assert_eq!(map, expected);
        } else {
            panic!("expected Object output");
        }
    }

    #[test]
    fn test_ini_double_quote() {
        let input = load_fixture("ini-double-quote.ini");
        let expected = parse_json_obj(&load_fixture("ini-double-quote.json"));
        let parser = IniParser;
        let result = parser.parse(&input, false).unwrap();
        if let ParseOutput::Object(map) = result {
            assert_eq!(map, expected);
        } else {
            panic!("expected Object output");
        }
    }

    #[test]
    fn test_ini_single_quote() {
        let input = load_fixture("ini-single-quote.ini");
        let expected = parse_json_obj(&load_fixture("ini-single-quote.json"));
        let parser = IniParser;
        let result = parser.parse(&input, false).unwrap();
        if let ParseOutput::Object(map) = result {
            assert_eq!(map, expected);
        } else {
            panic!("expected Object output");
        }
    }
}
