//! Syslog RFC 5424 streaming parser.

use super::syslog::parse_syslog_line;
use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::{Parser, StreamingParser};
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};

struct SyslogSParser;

static INFO: ParserInfo = ParserInfo {
    name: "syslog_s",
    argument: "--syslog-s",
    version: "1.0.0",
    description: "Syslog RFC 5424 string streaming parser",
    author: "jc-rs contributors",
    author_email: "jc-rs@example.com",
    compatible: &[Platform::Universal],
    tags: &[Tag::String, Tag::File, Tag::Streaming],
    magic_commands: &[],
    streaming: true,
    hidden: false,
    deprecated: false,
};

impl Parser for SyslogSParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, quiet: bool) -> Result<ParseOutput, ParseError> {
        let mut records = Vec::new();
        for line in input.lines() {
            if line.trim().is_empty() {
                continue;
            }
            match self.parse_line(line, quiet)? {
                Some(ParseOutput::Object(map)) => records.push(map),
                _ => {}
            }
        }
        Ok(ParseOutput::Array(records))
    }
}

impl StreamingParser for SyslogSParser {
    fn parse_line(&self, line: &str, _quiet: bool) -> Result<Option<ParseOutput>, ParseError> {
        if line.trim().is_empty() {
            return Ok(None);
        }
        Ok(Some(ParseOutput::Object(parse_syslog_line(line))))
    }
}

static INSTANCE: SyslogSParser = SyslogSParser;

inventory::submit! {
    ParserEntry::new(&INSTANCE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jc_rs_core::registry::find_parser;

    #[test]
    fn test_syslog_s_registered() {
        assert!(find_parser("syslog_s").is_some());
    }

    #[test]
    fn test_syslog_s_parse_line() {
        let p = SyslogSParser;
        let line = "<34>1 2003-10-11T22:14:15.003Z mymachine.example.com su - ID47 - msg";
        let result = p.parse_line(line, false).unwrap().unwrap();
        match result {
            ParseOutput::Object(map) => {
                assert_eq!(map["priority"], serde_json::json!(34));
            }
            _ => panic!("expected object"),
        }
    }
}
