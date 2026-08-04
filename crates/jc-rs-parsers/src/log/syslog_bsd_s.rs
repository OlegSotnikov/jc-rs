//! Syslog BSD RFC 3164 streaming parser.

use super::syslog_bsd::parse_syslog_bsd_line;
use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::{
    FnSession, LineParser, Parser, Record, StreamingParser, parse_via_session,
};
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
        parse_via_session(self, input, quiet)
    }

    fn as_streaming(&self) -> Option<&dyn StreamingParser> {
        Some(self)
    }
}

impl StreamingParser for SyslogBsdSParser {
    fn session(&self) -> Box<dyn LineParser> {
        Box::new(FnSession::new(syslog_bsd_line))
    }
}

/// Every BSD syslog line stands alone, so the session carries no state.
fn syslog_bsd_line(line: &str, _quiet: bool) -> Result<Option<Record>, ParseError> {
    if line.trim().is_empty() {
        return Ok(None);
    }
    Ok(Some(parse_syslog_bsd_line(line)))
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
        let mut session = SyslogBsdSParser.session();
        let line = "<34>Oct 11 22:14:15 mymachine su: msg";
        let record = session.parse_line(line, false).unwrap().unwrap();
        assert_eq!(record["priority"], serde_json::json!(34));
    }
}
