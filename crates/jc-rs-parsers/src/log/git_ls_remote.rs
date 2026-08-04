use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct GitLsRemoteParser;

static INFO: ParserInfo = ParserInfo {
    name: "git_ls_remote",
    argument: "--git-ls-remote",
    version: "1.0.0",
    description: "`git ls-remote` command parser",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[
        Platform::Linux,
        Platform::Darwin,
        Platform::FreeBSD,
        Platform::Windows,
    ],
    tags: &[Tag::Command],
    magic_commands: &["git ls-remote"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static GIT_LS_REMOTE_PARSER: GitLsRemoteParser = GitLsRemoteParser;

inventory::submit! {
    ParserEntry::new(&GIT_LS_REMOTE_PARSER)
}

impl Parser for GitLsRemoteParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    /// Default (processed) output: a single Object mapping reference -> commit hash.
    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Object(Map::new()));
        }

        let mut obj = Map::new();

        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.splitn(2, char::is_whitespace).collect();
            if parts.len() != 2 {
                continue;
            }

            let commit = parts[0].trim();
            let reference = parts[1].trim();

            obj.insert(reference.to_string(), Value::String(commit.to_string()));
        }

        Ok(ParseOutput::Object(obj))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_ls_remote_golden() {
        let input = include_str!("../../../../tests/fixtures/generic/git-ls-remote.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/git-ls-remote.json"
        ))
        .unwrap();
        let parser = GitLsRemoteParser;
        let result = parser.parse(input, false).unwrap();
        let result_val = serde_json::to_value(result).unwrap();
        assert_eq!(result_val, expected);
    }
}
