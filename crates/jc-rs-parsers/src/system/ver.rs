//! Parser for version strings.
//!
//! Best-effort attempt to parse various styles of version numbers.
//! Based on distutils/version.py from CPython.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct VerParser;

static INFO: ParserInfo = ParserInfo {
    name: "ver",
    argument: "--ver",
    version: "1.2.0",
    description: "Version string parser",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[
        Platform::Linux,
        Platform::Darwin,
        Platform::FreeBSD,
        Platform::Windows,
    ],
    tags: &[Tag::String, Tag::Slurpable],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static VER_PARSER: VerParser = VerParser;

inventory::submit! {
    ParserEntry::new(&VER_PARSER)
}

impl Parser for VerParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let mut out = Map::new();

        let data = input.trim();
        if data.is_empty() {
            return Ok(ParseOutput::Object(out));
        }

        // Try strict parse first
        if let Some(parsed) = try_strict_parse(data) {
            out.insert("major".to_string(), Value::Number(parsed.0.into()));
            out.insert("minor".to_string(), Value::Number(parsed.1.into()));
            out.insert("patch".to_string(), Value::Number(parsed.2.into()));
            if let Some(pre) = parsed.3 {
                out.insert("prerelease".to_string(), Value::String(pre));
            }
            if let Some(pre_num) = parsed.4 {
                out.insert("prerelease_num".to_string(), Value::Number(pre_num.into()));
            }
            out.insert("strict".to_string(), Value::Bool(true));
        } else {
            // Loose parse
            let components = loose_parse(data);
            let values: Vec<Value> = components
                .into_iter()
                .map(|s| {
                    if let Ok(n) = s.parse::<i64>() {
                        Value::Number(n.into())
                    } else {
                        Value::String(s)
                    }
                })
                .collect();
            out.insert("components".to_string(), Value::Array(values));
            out.insert("strict".to_string(), Value::Bool(false));
        }

        Ok(ParseOutput::Object(out))
    }
}

/// Try to parse as strict version: N.N[.N][[ab]N]
fn try_strict_parse(s: &str) -> Option<(i64, i64, i64, Option<String>, Option<i64>)> {
    // Pattern: ^(\d+)\.(\d+)(\.(\d+))?([ab](\d+))?$
    let s = s.trim();

    let mut rest = s;

    // Parse major
    let (major, r) = parse_digits(rest)?;
    rest = r;
    if !rest.starts_with('.') {
        return None;
    }
    rest = &rest[1..];

    // Parse minor
    let (minor, r) = parse_digits(rest)?;
    rest = r;

    // Optional .patch
    let patch = if rest.starts_with('.') {
        rest = &rest[1..];
        let (p, r) = parse_digits(rest)?;
        rest = r;
        p
    } else {
        0
    };

    // Optional prerelease [ab]N
    let (prerelease, prerelease_num) = if rest.starts_with('a') || rest.starts_with('b') {
        let pre_char = &rest[..1];
        rest = &rest[1..];
        let (n, r) = parse_digits(rest)?;
        rest = r;
        (Some(pre_char.to_string()), Some(n))
    } else {
        (None, None)
    };

    // Must be end of string
    if !rest.is_empty() {
        return None;
    }

    Some((major, minor, patch, prerelease, prerelease_num))
}

fn parse_digits(s: &str) -> Option<(i64, &str)> {
    let end = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    if end == 0 {
        return None;
    }
    let n = s[..end].parse::<i64>().ok()?;
    Some((n, &s[end..]))
}

/// Loose parse: split on digits, letters, dots
fn loose_parse(s: &str) -> Vec<String> {
    // Split on word boundaries between digits and letters
    let mut components = Vec::new();
    let mut current = String::new();
    let mut prev_is_digit: Option<bool> = None;

    for ch in s.chars() {
        if ch == '.' {
            if !current.is_empty() {
                components.push(current.clone());
                current.clear();
            }
            prev_is_digit = None;
            continue;
        }

        let is_digit = ch.is_ascii_digit();
        if let Some(prev) = prev_is_digit
            && prev != is_digit
            && !current.is_empty()
        {
            components.push(current.clone());
            current.clear();
        }
        current.push(ch);
        prev_is_digit = Some(is_digit);
    }

    if !current.is_empty() {
        components.push(current);
    }

    components
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ver_strict() {
        let parser = VerParser;
        let result = parser.parse("1.2a1", false).unwrap();
        if let ParseOutput::Object(obj) = result {
            assert_eq!(obj.get("major"), Some(&Value::Number(1.into())));
            assert_eq!(obj.get("minor"), Some(&Value::Number(2.into())));
            assert_eq!(obj.get("patch"), Some(&Value::Number(0.into())));
            assert_eq!(obj.get("prerelease"), Some(&Value::String("a".to_string())));
            assert_eq!(obj.get("prerelease_num"), Some(&Value::Number(1.into())));
            assert_eq!(obj.get("strict"), Some(&Value::Bool(true)));
        } else {
            panic!("Expected Object");
        }
    }

    #[test]
    fn test_ver_strict_with_patch() {
        let parser = VerParser;
        let result = parser.parse("1.2.3", false).unwrap();
        if let ParseOutput::Object(obj) = result {
            assert_eq!(obj.get("major"), Some(&Value::Number(1.into())));
            assert_eq!(obj.get("minor"), Some(&Value::Number(2.into())));
            assert_eq!(obj.get("patch"), Some(&Value::Number(3.into())));
            assert_eq!(obj.get("strict"), Some(&Value::Bool(true)));
        } else {
            panic!("Expected Object");
        }
    }

    #[test]
    fn test_ver_loose() {
        let parser = VerParser;
        let result = parser.parse("1.2beta3", false).unwrap();
        if let ParseOutput::Object(obj) = result {
            assert_eq!(obj.get("strict"), Some(&Value::Bool(false)));
            if let Some(Value::Array(components)) = obj.get("components") {
                assert!(components.contains(&Value::Number(1.into())));
                assert!(components.contains(&Value::String("beta".to_string())));
                assert!(components.contains(&Value::Number(3.into())));
            }
        } else {
            panic!("Expected Object");
        }
    }

    #[test]
    fn test_ver_empty() {
        let parser = VerParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Object(obj) = result {
            assert!(obj.is_empty());
        } else {
            panic!("Expected Object");
        }
    }
}
