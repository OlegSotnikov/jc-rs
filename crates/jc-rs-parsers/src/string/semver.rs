//! Semantic Version string parser.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use regex::Regex;
use serde_json::{Map, Value};
use std::sync::OnceLock;

struct SemverParser;

static SEMVER_INFO: ParserInfo = ParserInfo {
    name: "semver",
    argument: "--semver",
    version: "1.0.0",
    description: "Semantic Version string parser",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Universal],
    tags: &[Tag::String],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static SEMVER_RE: OnceLock<Regex> = OnceLock::new();

fn get_semver_re() -> &'static Regex {
    SEMVER_RE.get_or_init(|| {
        Regex::new(
            r"(?x)
            ^(?P<major>0|[1-9]\d*)\.
            (?P<minor>0|[1-9]\d*)\.
            (?P<patch>0|[1-9]\d*)
            (?:-(?P<prerelease>(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)
                (?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?
            (?:\+(?P<build>[0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?$
            ",
        )
        .expect("semver regex compile error")
    })
}

impl Parser for SemverParser {
    fn info(&self) -> &'static ParserInfo {
        &SEMVER_INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let input = input.trim();
        if input.is_empty() {
            return Err(ParseError::InvalidInput("empty input".to_string()));
        }

        let re = get_semver_re();
        let caps = re.captures(input).ok_or_else(|| {
            ParseError::InvalidInput(format!("'{}' does not conform to semver spec", input))
        })?;

        let major: i64 = caps["major"]
            .parse()
            .map_err(|e| ParseError::Generic(format!("parse major: {}", e)))?;
        let minor: i64 = caps["minor"]
            .parse()
            .map_err(|e| ParseError::Generic(format!("parse minor: {}", e)))?;
        let patch: i64 = caps["patch"]
            .parse()
            .map_err(|e| ParseError::Generic(format!("parse patch: {}", e)))?;

        let prerelease = caps.name("prerelease").map(|m| m.as_str().to_string());
        let build = caps.name("build").map(|m| m.as_str().to_string());

        let mut map = Map::new();
        map.insert("major".to_string(), Value::Number(major.into()));
        map.insert("minor".to_string(), Value::Number(minor.into()));
        map.insert("patch".to_string(), Value::Number(patch.into()));
        map.insert(
            "prerelease".to_string(),
            match prerelease {
                Some(s) => Value::String(s),
                None => Value::Null,
            },
        );
        map.insert(
            "build".to_string(),
            match build {
                Some(s) => Value::String(s),
                None => Value::Null,
            },
        );

        Ok(ParseOutput::Object(map))
    }
}

static SEMVER_PARSER_INSTANCE: SemverParser = SemverParser;

inventory::submit! {
    ParserEntry::new(&SEMVER_PARSER_INSTANCE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jc_rs_core::types::ParseOutput;

    fn parse_to_value(input: &str) -> serde_json::Value {
        let parser = SemverParser;
        let result = parser.parse(input, false).unwrap();
        match result {
            ParseOutput::Object(map) => serde_json::Value::Object(map),
            _ => panic!("expected object"),
        }
    }

    #[test]
    fn test_semver_basic() {
        let v = parse_to_value("1.2.3");
        assert_eq!(v["major"], 1);
        assert_eq!(v["minor"], 2);
        assert_eq!(v["patch"], 3);
        assert!(v["prerelease"].is_null());
        assert!(v["build"].is_null());
    }

    #[test]
    fn test_semver_full() {
        let v = parse_to_value("1.2.3-rc.1+44837");
        assert_eq!(v["major"], 1);
        assert_eq!(v["minor"], 2);
        assert_eq!(v["patch"], 3);
        assert_eq!(v["prerelease"], "rc.1");
        assert_eq!(v["build"], "44837");
    }

    #[test]
    fn test_semver_invalid() {
        let parser = SemverParser;
        let result = parser.parse("1.2", false);
        assert!(result.is_err());
    }

    #[test]
    fn test_semver_prerelease_only() {
        let v = parse_to_value("1.0.0-alpha.1");
        assert_eq!(v["major"], 1);
        assert_eq!(v["minor"], 0);
        assert_eq!(v["patch"], 0);
        assert_eq!(v["prerelease"], "alpha.1");
        assert!(v["build"].is_null());
    }
}
