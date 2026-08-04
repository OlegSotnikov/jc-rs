use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::{LineParser, Parser, Record};
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::parse_timestamp;
use regex::Regex;
use serde_json::{Map, Value};
use std::sync::LazyLock;

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

/// jc's own patterns, compiled once. `hash_pattern` is anchored at the start
/// only (as `re.match` is) and paired with a length check, which together mean
/// "exactly 40 hex characters".
static HASH_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:[0-9]|[a-f]){40}").expect("valid hash pattern"));

static CHANGES_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^\s(?P<files>\d+)\s+files? changed(?:,\s+(?P<insertions>\d+)\s+insertions?\(\+\))?(?:,\s+(?P<deletions>\d+)\s+deletions?\(-\))?",
    )
    .expect("valid changes pattern")
});

fn is_commit_hash(s: &str) -> bool {
    s.len() == 40 && HASH_RE.is_match(s)
}

/// Split `Name <email>` the way jc does: everything before the last whitespace
/// is the name, and the final token is the email only when it is bracketed.
/// A trailing word that is not bracketed is therefore dropped, and `<>` becomes
/// null rather than an empty string -- both are jc's behaviour, and jc is the
/// schema authority.
fn parse_name_email(line: &str) -> (Option<String>, Option<String>) {
    let mut name = None;
    let mut email = None;

    match line.trim_end().rsplit_once(char::is_whitespace) {
        Some((first, last)) => {
            name = Some(first.to_string());
            if let Some(addr) = last.strip_prefix('<').and_then(|v| v.strip_suffix('>')) {
                email = Some(addr.to_string());
            }
        }
        None => {
            let value = line.trim_start();
            match value.strip_prefix('<').and_then(|v| v.strip_suffix('>')) {
                Some(addr) => email = Some(addr.to_string()),
                None => name = Some(value.to_string()),
            }
        }
    }

    (
        name.filter(|v| !v.is_empty()),
        email.filter(|v| !v.is_empty()),
    )
}

fn add_timestamps(obj: &mut Map<String, Value>, date_str: &str) {
    let ts = parse_timestamp(date_str, Some("%a %b %d %H:%M:%S %Y %z"));
    obj.insert(
        "epoch".to_string(),
        ts.naive_epoch
            .map_or(Value::Null, |n| Value::Number(n.into())),
    );
    obj.insert(
        "epoch_utc".to_string(),
        ts.utc_epoch
            .map_or(Value::Null, |n| Value::Number(n.into())),
    );
}

fn int_or_null(s: &str) -> Value {
    s.trim()
        .parse::<i64>()
        .map_or(Value::Null, |n| Value::Number(n.into()))
}

/// One commit under construction.
///
/// `git log` is a stream of commits whose end is only known when the next one
/// begins, so the session emits the *previous* commit when it sees a new
/// `commit <hash>` line and the last one from `finalize()`.
#[derive(Default)]
pub(crate) struct GitLogSession {
    entry: Map<String, Value>,
    message_lines: Vec<String>,
    file_list: Vec<Value>,
    file_stats: Vec<Value>,
    /// jc leaks this between iterations: a file line with no `|` reuses the
    /// previous line's count. Replicated deliberately -- see `parse_name_email`.
    last_lines_changed: Option<String>,
}

impl GitLogSession {
    /// Close the commit under construction and hand it back, if there is one.
    ///
    /// `join_message` is false on the oneline path, where the message came from
    /// the commit line itself and jc does not overwrite it.
    fn flush(&mut self, join_message: bool) -> Option<Record> {
        if self.entry.is_empty() {
            return None;
        }

        if join_message && !self.message_lines.is_empty() {
            self.entry.insert(
                "message".to_string(),
                Value::String(self.message_lines.join("\n")),
            );
        }

        if !self.file_list.is_empty()
            && let Some(stats) = self.entry.get_mut("stats").and_then(Value::as_object_mut)
        {
            stats.insert(
                "files".to_string(),
                Value::Array(std::mem::take(&mut self.file_list)),
            );
        }

        if !self.file_stats.is_empty()
            && let Some(stats) = self.entry.get_mut("stats").and_then(Value::as_object_mut)
        {
            stats.insert(
                "file_stats".to_string(),
                Value::Array(std::mem::take(&mut self.file_stats)),
            );
        }

        let entry = std::mem::take(&mut self.entry);
        self.message_lines.clear();
        self.file_list.clear();
        self.file_stats.clear();
        Some(entry)
    }

    fn push_file(&mut self, line: &str) {
        let (name_part, stat_part) = match line.split_once('|') {
            Some((name, stats)) => (name, Some(stats)),
            None => (line, None),
        };
        let file_name = name_part.trim();
        self.file_list.push(Value::String(file_name.to_string()));

        if let Some(stats) = stat_part {
            self.last_lines_changed = stats
                .trim()
                .split(' ')
                .next()
                .map(|count| count.trim().to_string());
        }

        let mut file_stat = Map::with_capacity(2);
        file_stat.insert("name".to_string(), Value::String(file_name.to_string()));
        file_stat.insert(
            "lines_changed".to_string(),
            self.last_lines_changed
                .as_deref()
                .map_or(Value::Null, int_or_null),
        );
        self.file_stats.push(Value::Object(file_stat));
    }

    fn push_stats(&mut self, line: &str) {
        let Some(caps) = CHANGES_RE.captures(line) else {
            return;
        };
        let field = |name: &str| -> Value {
            caps.name(name)
                .map_or(Value::Number(0.into()), |m| int_or_null(m.as_str()))
        };
        let mut stats = Map::with_capacity(3);
        stats.insert("files_changed".to_string(), field("files"));
        stats.insert("insertions".to_string(), field("insertions"));
        stats.insert("deletions".to_string(), field("deletions"));
        self.entry.insert("stats".to_string(), Value::Object(stats));
    }
}

impl LineParser for GitLogSession {
    fn parse_line(&mut self, line: &str, _quiet: bool) -> Result<Option<Record>, ParseError> {
        let rest = |prefix: &str| line[prefix.len()..].trim().to_string();
        let first_word = line.split_whitespace().next().unwrap_or("");

        // Oneline style: "<hash> <subject>".
        if !line.starts_with(' ') && is_commit_hash(first_word) {
            let previous = self.flush(false);
            self.entry
                .insert("commit".to_string(), Value::String(first_word.to_string()));
            let message = line[first_word.len()..].trim_start();
            self.entry
                .insert("message".to_string(), Value::String(message.to_string()));
            return Ok(previous);
        }

        // Every other format opens with "commit <hash>".
        if let Some(hash) = line.strip_prefix("commit ") {
            let previous = self.flush(true);
            self.entry
                .insert("commit".to_string(), Value::String(hash.trim().to_string()));
            return Ok(previous);
        }

        if line.starts_with("Merge: ") {
            self.entry
                .insert("merge".to_string(), Value::String(rest("Merge: ")));
            return Ok(None);
        }

        if line.starts_with("Author: ") {
            let (name, email) = parse_name_email(&rest("Author: "));
            self.entry.insert(
                "author".to_string(),
                name.map_or(Value::Null, Value::String),
            );
            self.entry.insert(
                "author_email".to_string(),
                email.map_or(Value::Null, Value::String),
            );
            return Ok(None);
        }

        if line.starts_with("Date: ") || line.starts_with("AuthorDate: ") {
            let date = if line.starts_with("Date: ") {
                rest("Date: ")
            } else {
                rest("AuthorDate: ")
            };
            add_timestamps(&mut self.entry, &date);
            self.entry.insert("date".to_string(), Value::String(date));
            return Ok(None);
        }

        if line.starts_with("CommitDate: ") {
            self.entry.insert(
                "commit_by_date".to_string(),
                Value::String(rest("CommitDate: ")),
            );
            return Ok(None);
        }

        if line.starts_with("Commit: ") {
            let (name, email) = parse_name_email(&rest("Commit: "));
            self.entry.insert(
                "commit_by".to_string(),
                name.map_or(Value::Null, Value::String),
            );
            self.entry.insert(
                "commit_by_email".to_string(),
                email.map_or(Value::Null, Value::String),
            );
            return Ok(None);
        }

        // Message body is indented four spaces; `--stat` output one.
        if line.starts_with("    ") {
            self.message_lines.push(line.trim().to_string());
            return Ok(None);
        }

        if line.starts_with(' ') {
            if line.contains("changed, ") {
                self.push_stats(line);
            } else {
                self.push_file(line);
            }
        }

        Ok(None)
    }

    fn finalize(&mut self, _quiet: bool) -> Result<Option<Record>, ParseError> {
        Ok(self.flush(true))
    }
}

pub fn parse_git_log(input: &str) -> Vec<Map<String, Value>> {
    let mut session = GitLogSession::default();
    let mut entries = Vec::new();

    for line in input.lines() {
        // The session is infallible; `expect` here would be dead code.
        if let Ok(Some(entry)) = session.parse_line(line, true) {
            entries.push(entry);
        }
    }
    if let Ok(Some(entry)) = session.finalize(true) {
        entries.push(entry);
    }

    entries
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
