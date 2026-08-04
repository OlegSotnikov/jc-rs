//! Syslog BSD RFC 3164 streaming parser.

use super::syslog_bsd::parse_syslog_bsd_line;
use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::{Parser, StreamingParser};
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};

struct SyslogBsdSParser;

static INFO: ParserInfo = ParserInfo {
    name: "syslog_bsd_s",
    argument: "--syslog-bsd-s",
    version: "1.0.0",
    description: "Syslog BSD RFC 3164 string streaming parser",
    author: "jc-rs contributors",
    author_email: "jc-rs@example.com",
    compatible: &[Platform::Universal],
    tags: &[Tag::String, Tag::File, Tag::Streaming],
    magic_commands: &[],
    streaming: true,
    hidden: false,
    deprecated: false,
};

impl Parser for SyslogBsdSParser {
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

impl StreamingParser for SyslogBsdSParser {
    fn parse_line(&self, line: &str, _quiet: bool) -> Result<Option<ParseOutput>, ParseError> {
        if line.trim().is_empty() {
            return Ok(None);
        }
        Ok(Some(ParseOutput::Object(parse_syslog_bsd_line(line))))
    }
}

static INSTANCE: SyslogBsdSParser = SyslogBsdSParser;

inventory::submit! {
    ParserEntry::new(&INSTANCE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jc_rs_core::registry::find_parser;

    #[test]
    fn test_syslog_bsd_s_registered() {
        assert!(find_parser("syslog_bsd_s").is_some());
    }

    #[test]
    fn test_syslog_bsd_s_parse_line() {
        let p = SyslogBsdSParser;
        let line = "<34>Oct 11 22:14:15 mymachine su: msg";
        let result = p.parse_line(line, false).unwrap().unwrap();
        match result {
            ParseOutput::Object(map) => {
                assert_eq!(map["priority"], serde_json::json!(34));
            }
            _ => panic!("expected object"),
        }
    }
}
