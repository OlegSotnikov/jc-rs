use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct OsReleaseParser;

static INFO: ParserInfo = ParserInfo {
    name: "os_release",
    argument: "--os-release",
    version: "1.2.0",
    description: "Converts `/etc/os-release` file content to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[
        Platform::Linux,
        Platform::Darwin,
        Platform::FreeBSD,
        Platform::Windows,
    ],
    tags: &[Tag::File],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static OS_RELEASE_PARSER: OsReleaseParser = OsReleaseParser;

inventory::submit! {
    ParserEntry::new(&OS_RELEASE_PARSER)
}

impl Parser for OsReleaseParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Object(Map::new()));
        }

        let mut obj = Map::new();

        for line in input.lines() {
            // Strip inline comments (after #, but not inside quotes)
            let line = strip_comment(line);
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some(eq_pos) = line.find('=') {
                let key = line[..eq_pos].trim().to_string();
                let raw_val = line[eq_pos + 1..].trim();
                // Remove surrounding quotes
                let val = remove_quotes(raw_val);
                obj.insert(key, Value::String(val.to_string()));
            }
        }

        Ok(ParseOutput::Object(obj))
    }
}

fn strip_comment(line: &str) -> &str {
    // Simple comment stripping: find first # not inside quotes
    let mut in_quote = false;
    let mut quote_char = ' ';
    for (i, ch) in line.char_indices() {
        if in_quote {
            if ch == quote_char {
                in_quote = false;
            }
        } else if ch == '"' || ch == '\'' {
            in_quote = true;
            quote_char = ch;
        } else if ch == '#' {
            return &line[..i];
        }
    }
    line
}

fn remove_quotes(s: &str) -> &str {
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_os_release_centos() {
        let input = include_str!("../../../../tests/fixtures/generic/os-release-centos");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/os-release-centos.json"
        ))
        .unwrap();
        let parser = OsReleaseParser;
        let result = parser.parse(input, false).unwrap();
        let result_val = serde_json::to_value(result).unwrap();
        assert_eq!(result_val, expected);
    }

    #[test]
    fn test_os_release_ubuntu() {
        let input = include_str!("../../../../tests/fixtures/generic/os-release-ubuntu");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/os-release-ubuntu.json"
        ))
        .unwrap();
        let parser = OsReleaseParser;
        let result = parser.parse(input, false).unwrap();
        let result_val = serde_json::to_value(result).unwrap();
        assert_eq!(result_val, expected);
    }
}
