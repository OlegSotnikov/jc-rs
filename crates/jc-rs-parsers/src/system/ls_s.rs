//! Streaming parser for `ls -l` output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::{LineParser, Parser, Record, StreamingParser, parse_via_session};
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::{convert_to_int, parse_timestamp, timestamp::formats};
use regex::Regex;
use serde_json::Value;
use std::sync::LazyLock;

pub struct LsSParser;

static INFO: ParserInfo = ParserInfo {
    name: "ls_s",
    argument: "--ls-s",
    version: "1.0.0",
    description: "`ls` command streaming parser",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[
        Platform::Linux,
        Platform::Darwin,
        Platform::Aix,
        Platform::FreeBSD,
    ],
    tags: &[Tag::Command, Tag::Streaming],
    magic_commands: &[],
    streaming: true,
    hidden: false,
    deprecated: false,
};

static LS_S_PARSER: LsSParser = LsSParser;

inventory::submit! {
    ParserEntry::new(&LS_S_PARSER)
}

/// `ls -l` leaves the date in `Mmm DD HH:MM` form for recent files and a full
/// date for older ones. jc only computes a timestamp for the latter, so the
/// common case costs nothing.
static SHORT_DATE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[a-zA-Z]{3}\s{1,2}\d{1,2}\s{1,2}[0-9:]{4,5}").expect("valid short date pattern")
});

static TOTAL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^total [0-9]+").expect("valid total pattern"));

fn is_long_entry(line: &str) -> bool {
    let re_chars = "-dclpsbDCMnP?";
    let bytes = line.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    let first = bytes[0] as char;
    if !re_chars.contains(first) {
        return false;
    }
    if bytes.len() < 10 {
        return false;
    }
    bytes[1..10].iter().all(|&b| {
        let c = b as char;
        matches!(c, 'r' | 'w' | 'x' | '-' | 's' | 'S' | 't' | 'T' | '+')
    })
}

fn split_ls_long_line(line: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut remaining = line.trim_start();

    for i in 0..9 {
        if remaining.is_empty() {
            break;
        }
        if i == 8 {
            parts.push(remaining);
            break;
        }
        let token_end = remaining
            .find(char::is_whitespace)
            .unwrap_or(remaining.len());
        parts.push(&remaining[..token_end]);
        remaining = remaining[token_end..].trim_start();
    }

    parts
}

impl Parser for LsSParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(Vec::new()));
        }
        parse_via_session(self, input, quiet)
    }

    fn as_streaming(&self) -> Option<&dyn StreamingParser> {
        Some(self)
    }
}

impl StreamingParser for LsSParser {
    fn session(&self) -> Box<dyn LineParser> {
        Box::new(LsSession::default())
    }
}

/// `ls -lR` prints a `dir:` header before each directory's entries; every entry
/// after it belongs to that parent until the next header.
#[derive(Default)]
struct LsSession {
    parent: String,
}

impl LineParser for LsSession {
    fn parse_line(&mut self, line: &str, _quiet: bool) -> Result<Option<Record>, ParseError> {
        if TOTAL_RE.is_match(line) || line.trim().is_empty() {
            return Ok(None);
        }

        if !is_long_entry(line) {
            if line.trim_end().ends_with(':') {
                self.parent = line.trim().trim_end_matches(':').to_string();
                return Ok(None);
            }
            // jc raises here rather than skipping, so `-qq` reports the line
            // instead of quietly dropping it.
            return Err(ParseError::InvalidInput("Not ls -l data".to_string()));
        }

        let parts = split_ls_long_line(line.trim());
        let mut entry = Record::with_capacity(9);

        // A filename that begins with a newline leaves only eight fields.
        let filename_field = parts.get(8).copied().unwrap_or("");
        let (filename, link_to) = match filename_field.split_once(" -> ") {
            Some((name, target)) => (name, Some(target)),
            None => (filename_field, None),
        };
        entry.insert("filename".to_string(), Value::String(filename.to_string()));
        if let Some(target) = link_to {
            entry.insert("link_to".to_string(), Value::String(target.to_string()));
        }

        if !self.parent.is_empty() {
            entry.insert("parent".to_string(), Value::String(self.parent.clone()));
        }

        let field = |i: usize| parts.get(i).copied().unwrap_or("");
        entry.insert("flags".to_string(), Value::String(field(0).to_string()));
        // `links` and `size` are always present, and `-h` sizes like `4.0K`
        // still convert (jc strips non-numeric characters first).
        entry.insert("links".to_string(), int_value(field(1)));
        entry.insert("owner".to_string(), Value::String(field(2).to_string()));
        entry.insert("group".to_string(), Value::String(field(3).to_string()));
        entry.insert("size".to_string(), int_value(field(4)));

        let date = parts
            .get(5..8)
            .map(|fields| fields.join(" "))
            .unwrap_or_default();
        if !SHORT_DATE_RE.is_match(&date) {
            let ts = parse_timestamp(&date, &[formats::F7200]);
            entry.insert(
                "epoch".to_string(),
                ts.naive_epoch
                    .map_or(Value::Null, |n| Value::Number(n.into())),
            );
            entry.insert(
                "epoch_utc".to_string(),
                ts.utc_epoch
                    .map_or(Value::Null, |n| Value::Number(n.into())),
            );
        }
        entry.insert("date".to_string(), Value::String(date));

        Ok(Some(entry))
    }
}

fn int_value(s: &str) -> Value {
    convert_to_int(s).map_or(Value::Null, |n| Value::Number(n.into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ls_s_centos_alh() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/ls-alh.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/ls-alh-streaming.json"
        ))
        .unwrap();
        let result = LsSParser.parse(input, false).unwrap();
        assert_eq!(serde_json::to_value(result).unwrap(), expected);
    }

    #[test]
    fn test_ls_s_human_readable_size_is_not_dropped() {
        let mut session = LsSParser.session();
        let record = session
            .parse_line("drwxr-xr-x. 2 root root 4.0K Aug  9 17:35 bin", false)
            .unwrap()
            .unwrap();
        assert_eq!(record["size"], serde_json::json!(4));
        assert_eq!(record["filename"], "bin");
    }

    #[test]
    fn test_ls_s_recursive_parent_applies_to_following_entries() {
        let mut session = LsSParser.session();
        assert!(session.parse_line("/etc:", false).unwrap().is_none());
        let record = session
            .parse_line("-rw-r--r--. 1 root root 12 Aug  9 17:35 hosts", false)
            .unwrap()
            .unwrap();
        assert_eq!(record["parent"], "/etc");
    }

    #[test]
    fn test_ls_s_rejects_non_long_format() {
        let mut session = LsSParser.session();
        assert!(session.parse_line("file1.txt", false).is_err());
    }
}
