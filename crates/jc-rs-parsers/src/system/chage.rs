//! Parser for `chage --list` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ChageParser;

static INFO: ParserInfo = ParserInfo {
    name: "chage",
    argument: "--chage",
    version: "1.1.0",
    description: "Converts `chage --list` command output to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &["chage --list", "chage -l"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static CHAGE_PARSER: ChageParser = ChageParser;

inventory::submit! {
    ParserEntry::new(&CHAGE_PARSER)
}

impl Parser for ChageParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let mut out = Map::new();

        if input.trim().is_empty() {
            return Ok(ParseOutput::Object(out));
        }

        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if let Some(colon_pos) = line.find(':') {
                let key = line[..colon_pos].trim();
                let val = line[colon_pos + 1..].trim().to_string();

                let field = match key {
                    "Last password change" => "password_last_changed",
                    "Password expires" => "password_expires",
                    "Password inactive" => "password_inactive",
                    "Account expires" => "account_expires",
                    "Minimum number of days between password change" => {
                        "min_days_between_password_change"
                    }
                    "Maximum number of days between password change" => {
                        "max_days_between_password_change"
                    }
                    "Number of days of warning before password expires" => {
                        "warning_days_before_password_expires"
                    }
                    _ => continue,
                };

                // Convert integer fields
                if (field == "min_days_between_password_change"
                    || field == "max_days_between_password_change"
                    || field == "warning_days_before_password_expires")
                    && let Ok(n) = val.parse::<i64>()
                {
                    out.insert(field.to_string(), Value::Number(n.into()));
                    continue;
                }

                out.insert(field.to_string(), Value::String(val));
            }
        }

        Ok(ParseOutput::Object(out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chage_basic() {
        let input = "Last password change\t\t\t\t\t: never\nPassword expires\t\t\t\t\t: never\nPassword inactive\t\t\t\t\t: never\nAccount expires\t\t\t\t\t\t: never\nMinimum number of days between password change\t\t: 0\nMaximum number of days between password change\t\t: 99999\nNumber of days of warning before password expires\t: 7\n";
        let parser = ChageParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Object(obj) = result {
            assert_eq!(
                obj.get("password_last_changed"),
                Some(&Value::String("never".to_string()))
            );
            assert_eq!(
                obj.get("min_days_between_password_change"),
                Some(&Value::Number(0.into()))
            );
            assert_eq!(
                obj.get("max_days_between_password_change"),
                Some(&Value::Number(99999.into()))
            );
            assert_eq!(
                obj.get("warning_days_before_password_expires"),
                Some(&Value::Number(7.into()))
            );
        } else {
            panic!("Expected Object");
        }
    }

    #[test]
    fn test_chage_empty() {
        let parser = ChageParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Object(obj) = result {
            assert!(obj.is_empty());
        } else {
            panic!("Expected Object");
        }
    }
}
