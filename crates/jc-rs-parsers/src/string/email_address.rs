//! Email address string parser.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

struct EmailAddressParser;

static EMAIL_INFO: ParserInfo = ParserInfo {
    name: "email_address",
    argument: "--email-address",
    version: "1.0.0",
    description: "Email Address string parser",
    author: "jc-rs contributors",
    author_email: "jc-rs@example.com",
    compatible: &[Platform::Universal],
    tags: &[Tag::String],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

impl Parser for EmailAddressParser {
    fn info(&self) -> &'static ParserInfo {
        &EMAIL_INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let input = input.trim();
        if input.is_empty() {
            return Err(ParseError::InvalidInput("empty input".to_string()));
        }

        // Split on last '@'
        let at_pos = input
            .rfind('@')
            .ok_or_else(|| ParseError::InvalidInput("no '@' found in email address".to_string()))?;

        let local = &input[..at_pos];
        let domain = &input[at_pos + 1..];

        // Check for plus suffix in local part
        let (username, local_plus_suffix) = if let Some(plus_pos) = local.find('+') {
            (&local[..plus_pos], Some(&local[plus_pos + 1..]))
        } else {
            (local, None)
        };

        let mut map = Map::new();
        map.insert("username".to_string(), Value::String(username.to_string()));
        map.insert("domain".to_string(), Value::String(domain.to_string()));
        map.insert("local".to_string(), Value::String(local.to_string()));
        map.insert(
            "local_plus_suffix".to_string(),
            match local_plus_suffix {
                Some(s) => Value::String(s.to_string()),
                None => Value::Null,
            },
        );

        Ok(ParseOutput::Object(map))
    }
}

static EMAIL_PARSER_INSTANCE: EmailAddressParser = EmailAddressParser;

inventory::submit! {
    ParserEntry::new(&EMAIL_PARSER_INSTANCE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jc_rs_core::types::ParseOutput;

    fn parse_to_value(input: &str) -> serde_json::Value {
        let parser = EmailAddressParser;
        let result = parser.parse(input, false).unwrap();
        match result {
            ParseOutput::Object(map) => serde_json::Value::Object(map),
            _ => panic!("expected object"),
        }
    }

    #[test]
    fn test_email_basic() {
        let v = parse_to_value("joe.user@gmail.com");
        assert_eq!(v["username"], "joe.user");
        assert_eq!(v["domain"], "gmail.com");
        assert_eq!(v["local"], "joe.user");
        assert!(v["local_plus_suffix"].is_null());
    }

    #[test]
    fn test_email_with_plus() {
        let v = parse_to_value("joe.user+spam@gmail.com");
        assert_eq!(v["username"], "joe.user");
        assert_eq!(v["domain"], "gmail.com");
        assert_eq!(v["local"], "joe.user+spam");
        assert_eq!(v["local_plus_suffix"], "spam");
    }

    #[test]
    fn test_email_no_at() {
        let parser = EmailAddressParser;
        let result = parser.parse("notanemail", false);
        assert!(result.is_err());
    }
}
