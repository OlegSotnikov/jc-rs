use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::sparse_table_parse;
use regex::Regex;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::OnceLock;

pub struct AsciitableParser;

static INFO: ParserInfo = ParserInfo {
    name: "asciitable",
    argument: "--asciitable",
    version: "1.2.0",
    description: "ASCII and Unicode table parser",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[
        Platform::Linux,
        Platform::Darwin,
        Platform::FreeBSD,
        Platform::Windows,
    ],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static ASCIITABLE_PARSER: AsciitableParser = AsciitableParser;

inventory::submit! {
    ParserEntry::new(&ASCIITABLE_PARSER)
}

static ANSI_RE: OnceLock<Regex> = OnceLock::new();

fn get_ansi_re() -> &'static Regex {
    ANSI_RE.get_or_init(|| Regex::new(r"(\x9B|\x1B\[)[0-?]*[ -/]*[@-~]").unwrap())
}

fn remove_ansi(s: &str) -> String {
    get_ansi_re().replace_all(s, "").to_string()
}

fn lstrip_table(s: &str) -> String {
    let lines: Vec<&str> = s.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.is_empty() {
        return String::new();
    }
    let min_indent = lines
        .iter()
        .map(|l| l.len() - l.trim_start().len())
        .min()
        .unwrap_or(0);
    lines
        .iter()
        .map(|l| {
            if l.len() >= min_indent {
                &l[min_indent..]
            } else {
                l
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn rstrip_table(s: &str) -> String {
    let lines: Vec<&str> = s.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.is_empty() {
        return String::new();
    }
    let max_len = lines.iter().map(|l| l.trim_end().len()).max().unwrap_or(0);
    lines
        .iter()
        .map(|l| {
            let trimmed_len = l.trim_end().len();
            format!("{}{}", l.trim_end(), " ".repeat(max_len - trimmed_len))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn strip_table(s: &str) -> String {
    let s = lstrip_table(s);
    rstrip_table(&s)
}

fn is_separator(line: &str) -> bool {
    let line = line.trim();
    if line.is_empty() {
        return false;
    }
    let starts_ends = [
        ("|-", "-|"),
        ("━━", "━━"),
        ("──", "──"),
        ("┄┄", "┄┄"),
        ("┅┅", "┅┅"),
        ("┈┈", "┈┈"),
        ("┉┉", "┉┉"),
        ("══", "══"),
        ("--", "--"),
        ("==", "=="),
        ("+=", "=+"),
        ("+-", "-+"),
        ("╒", "╕"),
        ("╞", "╡"),
        ("╘", "╛"),
        ("┏", "┓"),
        ("┣", "┫"),
        ("┗", "┛"),
        ("┡", "┩"),
        ("┢", "┪"),
        ("┟", "┧"),
        ("┞", "┦"),
        ("┠", "┨"),
        ("┝", "┥"),
        ("┍", "┑"),
        ("┕", "┙"),
        ("┎", "┒"),
        ("┖", "┚"),
        ("╓", "╖"),
        ("╟", "╢"),
        ("╙", "╜"),
        ("╔", "╗"),
        ("╠", "╣"),
        ("╚", "╝"),
        ("┌", "┐"),
        ("├", "┤"),
        ("└", "┘"),
        ("╭", "╮"),
        ("╰", "╯"),
    ];
    starts_ends
        .iter()
        .any(|(s, e)| line.starts_with(s) && line.ends_with(e))
}

// Unicode replacement char used as header padding placeholder
const PAD_CHAR: char = '\u{FFFD}';

fn snake_case_header(s: &str) -> String {
    // Replace special chars with _ (except PAD_CHAR and alphanumeric/space)
    let mut result = String::new();
    for ch in s.chars() {
        if ch == PAD_CHAR || ch.is_alphanumeric() || ch == ' ' {
            result.push(ch);
        } else {
            result.push('_');
        }
    }
    // Replace spaces between words with _
    let mut final_result = String::new();
    let chars: Vec<char> = result.chars().collect();
    for i in 0..chars.len() {
        if chars[i] == ' ' {
            // Check if surrounded by word chars (not space/pad)
            let prev_word = i > 0 && chars[i - 1] != ' ' && chars[i - 1] != PAD_CHAR;
            let next_word = i + 1 < chars.len() && chars[i + 1] != ' ' && chars[i + 1] != PAD_CHAR;
            if prev_word && next_word {
                final_result.push('_');
            } else {
                final_result.push(' ');
            }
        } else {
            final_result.push(chars[i]);
        }
    }
    final_result
}

fn normalize_col_separators(line: &str) -> String {
    line.replace('│', "|")
        .replace('┃', "|")
        .replace('┆', "|")
        .replace('┇', "|")
        .replace('┊', "|")
        .replace('┋', "|")
        .replace('╎', "|")
        .replace('╏', "|")
        .replace('║', "|")
}

fn normalize_rows(table: &str) -> Vec<String> {
    let mut result: Vec<String> = Vec::new();

    for line in table.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if is_separator(line) {
            continue;
        }

        if result.is_empty() {
            // Header row
            let mut line = normalize_col_separators(line);

            // Handle non-left-justified headers after separators
            // Find patterns like "| " followed by spaces and header text
            let problem_re = Regex::new(r"\| ( +)([^|]+)").unwrap();
            let mut new_line = line.clone();
            for cap in problem_re.captures_iter(&line) {
                let spaces = cap.get(1).map_or("", |m| m.as_str());
                let header = cap.get(2).map_or("", |m| m.as_str());
                let old = format!("| {}{}", spaces, header);
                let pad = PAD_CHAR.to_string().repeat(spaces.len());
                let new = format!("| {}{}", pad, header);
                new_line = new_line.replacen(&old, &new, 1);
            }
            line = new_line;

            // Remove column separators
            line = line.replace('|', " ");
            result.push(snake_case_header(&line));
        } else {
            // Data row - remove column separators
            let line = normalize_col_separators(line);
            let line = line
                .replace('│', " ")
                .replace('┃', " ")
                .replace('┆', " ")
                .replace('┇', " ")
                .replace('┊', " ")
                .replace('┋', " ")
                .replace('╎', " ")
                .replace('╏', " ")
                .replace('║', " ")
                .replace('|', " ");
            result.push(line);
        }
    }

    result
}

fn fixup_headers(table: Vec<HashMap<String, Value>>) -> Vec<Map<String, Value>> {
    table
        .into_iter()
        .map(|row| {
            let mut new_row = Map::new();
            for (k, v) in row {
                // Remove PAD_CHAR, consecutive underscores, trailing underscores
                let k_new = k.chars().filter(|&c| c != PAD_CHAR).collect::<String>();
                let k_new = regex_replace_consecutive_underscores(&k_new);
                let k_new = k_new.trim_end_matches('_').to_string();
                let k_new = k_new.to_lowercase();
                new_row.insert(k_new, v);
            }
            new_row
        })
        .collect()
}

fn regex_replace_consecutive_underscores(s: &str) -> String {
    let mut result = String::new();
    let mut prev_underscore = false;
    for ch in s.chars() {
        if ch == '_' {
            if !prev_underscore {
                result.push(ch);
            }
            prev_underscore = true;
        } else {
            result.push(ch);
            prev_underscore = false;
        }
    }
    result
}

impl Parser for AsciitableParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let data = remove_ansi(input);
        let data = strip_table(&data);

        if data.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let rows = normalize_rows(&data);
        if rows.is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let table_str = rows.join("\n");
        let raw_table = sparse_table_parse(&table_str);
        let processed = fixup_headers(raw_table);

        let result: Vec<Map<String, Value>> = processed
            .into_iter()
            .map(|mut row| {
                // Convert empty strings to null
                for v in row.values_mut() {
                    if v == &Value::String(String::new()) {
                        *v = Value::Null;
                    }
                }
                row
            })
            .collect();

        Ok(ParseOutput::Array(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asciitable_simple() {
        let input =
            "foo        bar       baz\ngood day             12345\nhi there   abc def   3.14";
        let parser = AsciitableParser;
        let result = parser.parse(input, false).unwrap();
        match result {
            ParseOutput::Array(arr) => {
                assert_eq!(arr.len(), 2);
                assert!(arr[0].contains_key("foo"));
                assert!(arr[0].contains_key("bar"));
                assert!(arr[0].contains_key("baz"));
            }
            _ => panic!("expected array"),
        }
    }
}
