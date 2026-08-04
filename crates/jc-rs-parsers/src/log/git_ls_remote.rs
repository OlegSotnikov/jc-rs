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
        let mut obj = Map::new();
        for (reference, commit) in refs(input) {
            obj.insert(reference, Value::String(commit));
        }
        Ok(ParseOutput::Object(obj))
    }

    /// jc's raw form keeps one record per ref; `_process` collapses them into a
    /// single `{reference: commit}` object.
    fn parse_raw(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let records = refs(input)
            .map(|(reference, commit)| {
                let mut record = Map::with_capacity(2);
                record.insert("reference".to_string(), Value::String(reference));
                record.insert("commit".to_string(), Value::String(commit));
                record
            })
            .collect();
        Ok(ParseOutput::Array(records))
    }
}

/// `<commit>\t<reference>` per line, blank and malformed lines skipped.
fn refs(input: &str) -> impl Iterator<Item = (String, String)> + '_ {
    input.lines().filter_map(|line| {
        let line = line.trim();
        let (commit, reference) = line.split_once(char::is_whitespace)?;
        Some((reference.trim().to_string(), commit.trim().to_string()))
    })
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
