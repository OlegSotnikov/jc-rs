use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::parse_timestamp;
use regex::Regex;
use serde_json::{Map, Value};
use std::sync::OnceLock;

pub struct GitLogParser;

static INFO: ParserInfo = ParserInfo {
    name: "git_log",
    argument: "--git-log",
    version: "1.5.0",
    description: "`git log` command parser",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[
        Platform::Linux,
        Platform::Darwin,
        Platform::FreeBSD,
        Platform::Windows,
    ],
    tags: &[Tag::Command, Tag::Slurpable],
    magic_commands: &["git log"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static GIT_LOG_PARSER: GitLogParser = GitLogParser;

inventory::submit! {
    ParserEntry::new(&GIT_LOG_PARSER)
}

static HASH_RE: OnceLock<Regex> = OnceLock::new();
static CHANGES_RE: OnceLock<Regex> = OnceLock::new();

fn get_hash_re() -> &'static Regex {
    HASH_RE.get_or_init(|| Regex::new(r"^(?:[0-9a-f]){40}$").unwrap())
}

fn get_changes_re() -> &'static Regex {
    CHANGES_RE.get_or_init(|| {
        Regex::new(
            r"\s(?P<files>\d+)\s+files? changed(?:,\s+(?P<insertions>\d+)\s+insertions?\(\+\))?(?:,\s+(?P<deletions>\d+)\s+deletions?\(-\))?",
        )
        .unwrap()
    })
}

fn is_commit_hash(s: &str) -> bool {
    s.len() == 40 && get_hash_re().is_match(s)
}

fn parse_name_email(s: &str) -> (Option<String>, Option<String>) {
    // "Name <email>" or "<email>" or "Name"
    let s = s.trim();
    if let Some(lt_pos) = s.rfind('<') {
        if s.ends_with('>') {
            let name = s[..lt_pos].trim();
            let email = &s[lt_pos + 1..s.len() - 1];
            let name_opt = if name.is_empty() {
                None
            } else {
                Some(name.to_string())
            };
            let email_opt = if email.is_empty() {
                None
            } else {
                Some(email.to_string())
            };
            return (name_opt, email_opt);
        }
    }
    // No email bracket found
    if s.is_empty() {
        (None, None)
    } else {
        (Some(s.to_string()), None)
    }
}

fn add_timestamps(obj: &mut Map<String, Value>, date_str: &str) {
    let ts = parse_timestamp(date_str, Some("%a %b %d %H:%M:%S %Y %z"));
    match ts.naive_epoch {
        Some(n) => obj.insert("epoch".to_string(), Value::Number(n.into())),
        None => obj.insert("epoch".to_string(), Value::Null),
    };
    match ts.utc_epoch {
        Some(n) => obj.insert("epoch_utc".to_string(), Value::Number(n.into())),
        None => obj.insert("epoch_utc".to_string(), Value::Null),
    };
}

pub fn parse_git_log(input: &str) -> Vec<Map<String, Value>> {
    let mut raw_output: Vec<Map<String, Value>> = Vec::new();
    let mut output_line: Map<String, Value> = Map::new();
    let mut message_lines: Vec<String> = Vec::new();
    let mut file_list: Vec<Value> = Vec::new();
    let mut file_stats_list: Vec<Value> = Vec::new();

    fn finalize_entry(
        output_line: &mut Map<String, Value>,
        message_lines: &mut Vec<String>,
        file_list: &mut Vec<Value>,
        file_stats_list: &mut Vec<Value>,
        raw_output: &mut Vec<Map<String, Value>>,
    ) {
        if !output_line.is_empty() {
            if !message_lines.is_empty() {
                output_line.insert(
                    "message".to_string(),
                    Value::String(message_lines.join("\n")),
                );
            }
            if !file_list.is_empty() {
                if let Some(stats) = output_line.get_mut("stats") {
                    if let Some(stats_obj) = stats.as_object_mut() {
                        stats_obj.insert("files".to_string(), Value::Array(file_list.clone()));
                    }
                }
            }
            if !file_stats_list.is_empty() {
                if let Some(stats) = output_line.get_mut("stats") {
                    if let Some(stats_obj) = stats.as_object_mut() {
                        stats_obj.insert(
                            "file_stats".to_string(),
                            Value::Array(file_stats_list.clone()),
                        );
                    }
                }
            }
            raw_output.push(output_line.clone());
            *output_line = Map::new();
            *message_lines = Vec::new();
            *file_list = Vec::new();
            *file_stats_list = Vec::new();
        }
    }

    for line in input.lines() {
        let line_parts: Vec<&str> = line.splitn(2, char::is_whitespace).collect();
        let first_word = line_parts.first().copied().unwrap_or("");

        // Oneline format: "HASH message"
        if !line.starts_with(' ') && !line.starts_with('\t') && is_commit_hash(first_word) {
            finalize_entry(
                &mut output_line,
                &mut message_lines,
                &mut file_list,
                &mut file_stats_list,
                &mut raw_output,
            );
            output_line.insert("commit".to_string(), Value::String(first_word.to_string()));
            if let Some(msg) = line_parts.get(1) {
                output_line.insert("message".to_string(), Value::String(msg.to_string()));
            }
            continue;
        }

        // commit HASH line (medium/full/fuller formats)
        if line.starts_with("commit ") {
            finalize_entry(
                &mut output_line,
                &mut message_lines,
                &mut file_list,
                &mut file_stats_list,
                &mut raw_output,
            );
            if let Some(hash) = line_parts.get(1) {
                let hash = hash.trim();
                output_line.insert("commit".to_string(), Value::String(hash.to_string()));
            }
            continue;
        }

        if line.starts_with("Merge: ") {
            if let Some(val) = line_parts.get(1) {
                output_line.insert("merge".to_string(), Value::String(val.trim().to_string()));
            }
            continue;
        }

        if line.starts_with("Author: ") {
            if let Some(val) = line_parts.get(1) {
                let (name, email) = parse_name_email(val);
                output_line.insert(
                    "author".to_string(),
                    name.map(Value::String).unwrap_or(Value::Null),
                );
                output_line.insert(
                    "author_email".to_string(),
                    email.map(Value::String).unwrap_or(Value::Null),
                );
            }
            continue;
        }

        // "Date:   ..." (with possible extra spaces after colon)
        if line.starts_with("Date:") && !line.starts_with("Date-") {
            let date_val = line["Date:".len()..].trim();
            output_line.insert("date".to_string(), Value::String(date_val.to_string()));
            add_timestamps(&mut output_line, date_val);
            continue;
        }

        if line.starts_with("AuthorDate: ") {
            if let Some(val) = line_parts.get(1) {
                let date_val = val.trim();
                output_line.insert("date".to_string(), Value::String(date_val.to_string()));
                add_timestamps(&mut output_line, date_val);
            }
            continue;
        }

        if line.starts_with("CommitDate: ") {
            if let Some(val) = line_parts.get(1) {
                output_line.insert(
                    "commit_by_date".to_string(),
                    Value::String(val.trim().to_string()),
                );
            }
            continue;
        }

        if line.starts_with("Commit: ") {
            if let Some(val) = line_parts.get(1) {
                let (name, email) = parse_name_email(val);
                output_line.insert(
                    "commit_by".to_string(),
                    name.map(Value::String).unwrap_or(Value::Null),
                );
                output_line.insert(
                    "commit_by_email".to_string(),
                    email.map(Value::String).unwrap_or(Value::Null),
                );
            }
            continue;
        }

        // Message lines start with 4 spaces
        if line.starts_with("    ") {
            message_lines.push(line.trim().to_string());
            continue;
        }

        // File stat lines start with a space
        if line.starts_with(' ') || line.starts_with('\t') {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Summary line: " N files changed, ..."
            if trimmed.contains("changed,")
                || trimmed.contains("changed") && trimmed.contains("file")
            {
                let re = get_changes_re();
                if let Some(caps) = re.captures(line) {
                    let files = caps.name("files").map_or("0", |m| m.as_str());
                    let insertions = caps.name("insertions").map_or("0", |m| m.as_str());
                    let deletions = caps.name("deletions").map_or("0", |m| m.as_str());

                    let mut stats_obj = Map::new();
                    stats_obj.insert(
                        "files_changed".to_string(),
                        Value::Number(files.parse::<i64>().unwrap_or(0).into()),
                    );
                    stats_obj.insert(
                        "insertions".to_string(),
                        Value::Number(insertions.parse::<i64>().unwrap_or(0).into()),
                    );
                    stats_obj.insert(
                        "deletions".to_string(),
                        Value::Number(deletions.parse::<i64>().unwrap_or(0).into()),
                    );
                    output_line.insert("stats".to_string(), Value::Object(stats_obj));
                }
                continue;
            }

            // File detail line: " filename | N ++++----"
            let parts: Vec<&str> = trimmed.splitn(2, '|').collect();
            let file_name = parts[0].trim();
            if !file_name.is_empty() {
                file_list.push(Value::String(file_name.to_string()));

                let mut file_stat_obj = Map::new();
                file_stat_obj.insert("name".to_string(), Value::String(file_name.to_string()));

                let lines_changed_val = if parts.len() > 1 {
                    let stat_part = parts[1].trim();
                    let count_str = stat_part.split_whitespace().next().unwrap_or("");
                    match count_str.parse::<i64>() {
                        Ok(n) => Value::Number(n.into()),
                        Err(_) => Value::Null,
                    }
                } else {
                    Value::Null
                };
                file_stat_obj.insert("lines_changed".to_string(), lines_changed_val);
                file_stats_list.push(Value::Object(file_stat_obj));
            }
        }
    }

    // Flush last entry
    finalize_entry(
        &mut output_line,
        &mut message_lines,
        &mut file_list,
        &mut file_stats_list,
        &mut raw_output,
    );

    raw_output
}

impl Parser for GitLogParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let entries = parse_git_log(input);
        Ok(ParseOutput::Array(entries))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! git_log_test {
        ($name:ident, $input:expr, $expected:expr) => {
            #[test]
            fn $name() {
                let input = include_str!($input);
                let expected: serde_json::Value =
                    serde_json::from_str(include_str!($expected)).unwrap();
                let parser = GitLogParser;
                let result = parser.parse(input, false).unwrap();
                let result_val = serde_json::to_value(result).unwrap();
                assert_eq!(result_val, expected);
            }
        };
    }

    git_log_test!(
        test_git_log_oneline,
        "../../../../tests/fixtures/generic/git-log-oneline.out",
        "../../../../tests/fixtures/generic/git-log-oneline.json"
    );
    git_log_test!(
        test_git_log_medium,
        "../../../../tests/fixtures/generic/git-log-medium.out",
        "../../../../tests/fixtures/generic/git-log-medium.json"
    );
    git_log_test!(
        test_git_log_default,
        "../../../../tests/fixtures/generic/git-log.out",
        "../../../../tests/fixtures/generic/git-log.json"
    );
    git_log_test!(
        test_git_log_blank_author_fix,
        "../../../../tests/fixtures/generic/git-log-blank-author-fix.out",
        "../../../../tests/fixtures/generic/git-log-blank-author-fix.json"
    );
    git_log_test!(
        test_git_log_hash_in_message,
        "../../../../tests/fixtures/generic/git-log-hash-in-message-fix.out",
        "../../../../tests/fixtures/generic/git-log-hash-in-message-fix.json"
    );
    git_log_test!(
        test_git_log_is_hash_regex,
        "../../../../tests/fixtures/generic/git-log-is-hash-regex-fix.out",
        "../../../../tests/fixtures/generic/git-log-is-hash-regex-fix.json"
    );
}
