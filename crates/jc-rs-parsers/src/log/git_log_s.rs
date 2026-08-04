use super::git_log::parse_git_log;
use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::{Parser, StreamingParser};
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

    /// For streaming mode, accumulate all input and parse as a full git log.
    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }
        let entries = parse_git_log(input);
        Ok(ParseOutput::Array(entries))
    }
}

impl StreamingParser for GitLogSParser {
    /// Single-line streaming not supported for git log (multi-line format).
    /// Returns None for all lines.
    fn parse_line(&self, _line: &str, _quiet: bool) -> Result<Option<ParseOutput>, ParseError> {
        Ok(None)
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
