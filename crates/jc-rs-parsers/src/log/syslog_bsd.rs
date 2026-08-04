//! Syslog BSD RFC 3164 parser.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use regex::Regex;
use serde_json::{Map, Value};
use std::sync::OnceLock;

struct SyslogBsdParser;

static INFO: ParserInfo = ParserInfo {
    name: "syslog_bsd",
    argument: "--syslog-bsd",
    version: "1.0.0",
    description: "Syslog BSD RFC 3164 string parser",
    author: "jc-rs contributors",
    author_email: "jc-rs@example.com",
    compatible: &[Platform::Universal],
    tags: &[Tag::String, Tag::File, Tag::Slurpable],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

// Matches optional priority + BSD date: "MMM [D]D HH:MM:SS"
static BSD_RE: OnceLock<Regex> = OnceLock::new();

fn get_bsd_re() -> &'static Regex {
    BSD_RE.get_or_init(|| {
        Regex::new(
            r"^(?:<(?P<priority>\d{1,3})>)?(?P<date>\w{3} {1,2}\d{1,2} \d{2}:\d{2}:\d{2}) (?P<rest>.+)$"
        ).expect("bsd syslog regex")
    })
}

pub fn parse_syslog_bsd_line(line: &str) -> Map<String, Value> {
    let mut map = Map::new();

    let re = get_bsd_re();
    let caps = match re.captures(line) {
        Some(c) => c,
        None => {
            map.insert("unparsable".to_string(), Value::String(line.to_string()));
            return map;
        }
    };

    // priority
    let priority = caps
        .name("priority")
        .and_then(|m| m.as_str().parse::<i64>().ok());
    // Validate: RFC 3164 max priority = 191
    if let Some(p) = priority {
        if p > 191 {
            map.insert("unparsable".to_string(), Value::String(line.to_string()));
            return map;
        }
    }
    map.insert(
        "priority".to_string(),
        match priority {
            Some(n) => Value::Number(n.into()),
            None => Value::Null,
        },
    );

    // date
    let date = caps.name("date").map(|m| m.as_str()).unwrap_or("");
    map.insert("date".to_string(), Value::String(date.to_string()));

    // rest = "hostname [tag[stuff]: ]content"
    let rest = caps.name("rest").map(|m| m.as_str()).unwrap_or("");

    // Split hostname from remainder
    // If the first token ends with ':', it's "hostname:" with no tag — content follows directly
    let (hostname, after_host, has_colon_host) = if let Some(sp) = rest.find(' ') {
        let host_token = &rest[..sp];
        let after = &rest[sp + 1..];
        if host_token.ends_with(':') {
            (&host_token[..host_token.len() - 1], after, true)
        } else {
            (host_token, after, false)
        }
    } else {
        let host_token = rest;
        if host_token.ends_with(':') {
            (&host_token[..host_token.len() - 1], "", true)
        } else {
            (host_token, "", false)
        }
    };

    map.insert("hostname".to_string(), Value::String(hostname.to_string()));

    // Parse tag and content from after_host
    // If the hostname had a colon (e.g. "avas: content"), there's no tag
    let (tag, content) = if has_colon_host {
        (None, after_host.to_string())
    } else {
        parse_tag_content(after_host)
    };

    map.insert(
        "tag".to_string(),
        match tag {
            Some(t) => Value::String(t),
            None => Value::Null,
        },
    );
    map.insert(
        "content".to_string(),
        Value::String(content.trim_end().to_string()),
    );

    map
}

/// Parse tag and content from the message portion after the hostname.
/// Tag = leading `\w+` characters; content = everything after tag
/// (stripping leading `: ` only if the first non-tag char is `:`).
fn parse_tag_content(s: &str) -> (Option<String>, String) {
    if s.is_empty() {
        return (None, String::new());
    }

    // Find the end of leading word characters (tag)
    let tag_end = s
        .char_indices()
        .find(|(_, c)| !c.is_alphanumeric() && *c != '_')
        .map(|(i, _)| i)
        .unwrap_or(s.len());

    if tag_end == 0 {
        // Starts with non-word char — no tag
        return (None, s.to_string());
    }

    let tag = &s[..tag_end];
    let after_tag = &s[tag_end..];

    // Content starts after the tag; strip leading ': ' if present
    let content = if after_tag.starts_with(": ") {
        after_tag[2..].to_string()
    } else if after_tag.starts_with(':') && after_tag.len() > 1 {
        // `:content` without space — still strip the colon
        after_tag[1..].to_string()
    } else {
        after_tag.to_string()
    };

    (Some(tag.to_string()), content)
}

impl Parser for SyslogBsdParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let records: Vec<Map<String, Value>> = input
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(parse_syslog_bsd_line)
            .collect();
        Ok(ParseOutput::Array(records))
    }
}

static INSTANCE: SyslogBsdParser = SyslogBsdParser;

inventory::submit! {
    ParserEntry::new(&INSTANCE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jc_rs_core::registry::find_parser;
    use jc_rs_core::types::ParseOutput;

    fn get_fixture(rel_path: &str) -> String {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        let paths = [
            format!("{manifest}/../../tests/fixtures/{rel_path}"),
            format!("{manifest}/../../../tests/fixtures/{rel_path}"),
        ];
        for p in &paths {
            if let Ok(c) = std::fs::read_to_string(p) {
                return c;
            }
        }
        panic!("fixture not found: {rel_path}");
    }

    #[test]
    fn test_syslog_bsd_registered() {
        assert!(find_parser("syslog_bsd").is_some());
    }

    #[test]
    fn test_syslog_bsd_basic() {
        let line = "<34>Oct 11 22:14:15 mymachine su: 'su root' failed for lonvick on /dev/pts/8";
        let map = parse_syslog_bsd_line(line);
        assert_eq!(map["priority"], serde_json::json!(34));
        assert_eq!(map["date"], serde_json::json!("Oct 11 22:14:15"));
        assert_eq!(map["hostname"], serde_json::json!("mymachine"));
        assert_eq!(map["tag"], serde_json::json!("su"));
        assert_eq!(
            map["content"],
            serde_json::json!("'su root' failed for lonvick on /dev/pts/8")
        );
    }

    #[test]
    fn test_syslog_bsd_no_priority() {
        let line = "Oct 11 22:14:15 mymachine su: 'su root' failed for lonvick on /dev/pts/8";
        let map = parse_syslog_bsd_line(line);
        assert!(map["priority"].is_null());
        assert_eq!(map["tag"], serde_json::json!("su"));
    }

    #[test]
    fn test_syslog_bsd_pid_in_tag() {
        let line = "<35>Mar  7 04:02:16 avas clamd[11165]: /var/amavis/file: Worm.Mydoom.F FOUND";
        let map = parse_syslog_bsd_line(line);
        assert_eq!(map["tag"], serde_json::json!("clamd"));
        assert_eq!(
            map["content"],
            serde_json::json!("[11165]: /var/amavis/file: Worm.Mydoom.F FOUND")
        );
    }

    #[test]
    fn test_syslog_bsd_no_tag() {
        let line = "Mar  8 15:18:40 avas: last message repeated 11 times";
        let map = parse_syslog_bsd_line(line);
        assert_eq!(map["hostname"], serde_json::json!("avas"));
        assert!(map["tag"].is_null());
        assert_eq!(
            map["content"],
            serde_json::json!("last message repeated 11 times")
        );
    }

    #[test]
    fn test_syslog_bsd_ipv6_hostname() {
        let line = "Mar  8 15:18:40 127:0:ab::1 sshd: unauthorized request";
        let map = parse_syslog_bsd_line(line);
        assert_eq!(map["hostname"], serde_json::json!("127:0:ab::1"));
        assert_eq!(map["tag"], serde_json::json!("sshd"));
        assert_eq!(map["content"], serde_json::json!("unauthorized request"));
    }

    #[test]
    fn test_syslog_bsd_unparsable_priority() {
        let line = "<3444>Oct 11 22:14:15 mymachine su: msg";
        let map = parse_syslog_bsd_line(line);
        assert!(map.contains_key("unparsable"));
    }

    #[test]
    fn test_syslog_bsd_unparsable_no_date() {
        let line = "<7>unparsable line";
        let map = parse_syslog_bsd_line(line);
        assert!(map.contains_key("unparsable"));
    }

    #[test]
    fn test_syslog_bsd_fixture() {
        let input = get_fixture("generic/syslog-3164.out");
        let parser = find_parser("syslog_bsd").unwrap();
        let result = parser.parse(&input, true).unwrap();
        let arr = match result {
            ParseOutput::Array(v) => v,
            _ => panic!("expected array"),
        };
        assert!(!arr.is_empty());
        let has_unparsable = arr.iter().any(|r| r.contains_key("unparsable"));
        assert!(has_unparsable);
        // First record
        assert_eq!(arr[0]["priority"], serde_json::json!(34));
        assert_eq!(arr[0]["hostname"], serde_json::json!("mymachine"));
        assert_eq!(arr[0]["tag"], serde_json::json!("su"));
    }
}
