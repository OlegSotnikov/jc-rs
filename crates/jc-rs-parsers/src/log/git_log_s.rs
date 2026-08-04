use super::git_log::GitLogSession;
use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::{LineParser, Parser, StreamingParser, parse_via_session};
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};

pub struct GitLogSParser;

static INFO: ParserInfo = ParserInfo {
    name: "git_log_s",
    argument: "--git-log-s",
    version: "1.5.0",
    description: "`git log` command streaming parser",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[
        Platform::Linux,
        Platform::Darwin,
        Platform::FreeBSD,
        Platform::Windows,
    ],
    tags: &[Tag::Command, Tag::Streaming],
    magic_commands: &[],
    streaming: true,
    hidden: false,
    deprecated: false,
};

static GIT_LOG_S_PARSER: GitLogSParser = GitLogSParser;

inventory::submit! {
    ParserEntry::new(&GIT_LOG_S_PARSER)
}

impl Parser for GitLogSParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }
        parse_via_session(self, input, quiet)
    }

    fn as_streaming(&self) -> Option<&dyn StreamingParser> {
        Some(self)
    }
}

impl StreamingParser for GitLogSParser {
    /// A commit spans many lines, so records come out one commit behind the
    /// input -- the session emits the previous commit when the next one starts.
    fn session(&self) -> Box<dyn LineParser> {
        Box::new(GitLogSession::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_log_s_golden() {
        let input = include_str!("../../../../tests/fixtures/generic/git-log.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/git-log-streaming.json"
        ))
        .unwrap();
        let parser = GitLogSParser;
        let result = parser.parse(input, false).unwrap();
        let result_val = serde_json::to_value(result).unwrap();
        assert_eq!(result_val, expected);
    }
}
