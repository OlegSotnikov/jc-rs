//! Syslog RFC 5424 streaming parser.

use super::syslog::parse_syslog_line;
use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::{
    FnSession, LineParser, Parser, Record, StreamingParser, parse_via_session,
};
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
        parse_via_session(self, input, quiet)
    }

    fn as_streaming(&self) -> Option<&dyn StreamingParser> {
        Some(self)
    }
}

impl StreamingParser for SyslogSParser {
    fn session(&self) -> Box<dyn LineParser> {
        Box::new(FnSession::new(syslog_line))
    }
}

/// Every syslog line stands alone, so the session carries no state.
fn syslog_line(line: &str, _quiet: bool) -> Result<Option<Record>, ParseError> {
    if line.trim().is_empty() {
        return Ok(None);
    }
    Ok(Some(parse_syslog_line(line)))
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
        let mut session = SyslogSParser.session();
        let line = "<34>1 2003-10-11T22:14:15.003Z mymachine.example.com su - ID47 - msg";
        let record = session.parse_line(line, false).unwrap().unwrap();
        assert_eq!(record["priority"], serde_json::json!(34));
    }
}
