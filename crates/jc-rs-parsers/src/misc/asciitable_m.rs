use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use regex::Regex;
use serde_json::{Map, Value};
use std::sync::LazyLock;
use std::sync::OnceLock;

pub struct AsciitableMParser;

static INFO: ParserInfo = ParserInfo {
    name: "asciitable_m",
    argument: "--asciitable-m",
    version: "1.2.0",
    description: "Multi-line ASCII and Unicode table parser",
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

static ASCIITABLE_M_PARSER: AsciitableMParser = AsciitableMParser;

inventory::submit! {
    ParserEntry::new(&ASCIITABLE_M_PARSER)
}

static ANSI_RE: OnceLock<Regex> = OnceLock::new();

static UNDERSCORES_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"__+").expect("valid pattern"));

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
        ("+=", "=+"),
        ("+-", "-+"),
    ];
    starts_ends
        .iter()
        .any(|(s, e)| line.starts_with(s) && line.ends_with(e))
}

fn table_sniff(s: &str) -> &'static str {
    for line in s.lines() {
        let line = line.trim();
        if ["╞", "├", "┡", "┣", "┢", "┟", "┞", "┠", "┝", "╟", "╠"]
            .iter()
            .any(|p| line.starts_with(p))
            && ["╡", "┤", "┩", "┫", "┪", "┧", "┦", "┨", "┥", "╢", "╣"]
                .iter()
                .any(|p| line.ends_with(p))
        {
            return "pretty";
        }
        if (line.starts_with("+=") && line.ends_with("=+"))
            || (line.starts_with("+-") && line.ends_with("-+"))
        {
            return "pretty";
        }
    }

    let lines: Vec<&str> = s.lines().collect();
    if lines.len() > 1 {
        let second = lines[1].trim();
        if second.starts_with("|-") && second.ends_with("-|") {
            return "markdown";
        }
    }

    "simple"
}

fn normalize_col_sep(line: &str) -> String {
    line.replace(['│', '┃', '┆', '┇', '┊', '┋', '╎', '╏', '║'], "|")
}

fn fixup_separators(line: &str) -> String {
    let line = normalize_col_sep(line);
    let mut chars: Vec<char> = line.chars().collect();
    // Remove first '|' if it is the first char
    if chars.first() == Some(&'|') {
        chars[0] = ' ';
    }
    // Remove last '|' if it is the last char
    if chars.last() == Some(&'|') {
        let last = chars.len() - 1;
        chars[last] = ' ';
    }
    chars.into_iter().collect()
}

fn snake_case_m(s: &str) -> String {
    // Replace special chars (not alphanumeric, space, or pipe) with _
    let mut result = String::new();
    for ch in s.chars() {
        if ch.is_alphanumeric()
            || ch == ' '
            || ch == '|'
            || ch == '│'
            || ch == '┃'
            || ch == '┆'
            || ch == '┇'
            || ch == '┊'
            || ch == '┋'
            || ch == '╎'
            || ch == '╏'
            || ch == '║'
        {
            result.push(ch);
        } else {
            result.push('_');
        }
    }
    // Replace spaces between words with _
    // Treat pipe chars as non-word separators (like space)
    fn is_sep(c: char) -> bool {
        c == ' '
            || c == '|'
            || c == '│'
            || c == '┃'
            || c == '┆'
            || c == '┇'
            || c == '┊'
            || c == '┋'
            || c == '╎'
            || c == '╏'
            || c == '║'
    }
    let mut final_result = String::new();
    let chars: Vec<char> = result.chars().collect();
    for i in 0..chars.len() {
        if chars[i] == ' ' {
            let prev_word = i > 0 && !is_sep(chars[i - 1]);
            let next_word = i + 1 < chars.len() && !is_sep(chars[i + 1]);
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

/// Normalize rows: returns Vec<(row_counter, Vec<cell_string>)>
fn normalize_rows(lines: &[&str]) -> Vec<(usize, Vec<String>)> {
    let mut result = Vec::new();
    let mut header_found = false;
    let mut data_found = false;
    let mut row_counter: usize = 0;

    for line in lines {
        if line.trim().is_empty() {
            continue;
        }

        if !header_found && !data_found && is_separator(line) {
            continue;
        }

        if !header_found && !data_found && !is_separator(line) {
            header_found = true;
            let line = snake_case_m(line);
            let line = fixup_separators(&line);
            let cells: Vec<String> = line.split('|').map(|s| s.trim().to_string()).collect();
            result.push((row_counter, cells));
            continue;
        }

        if header_found && !data_found && !is_separator(line) {
            let line = snake_case_m(line);
            let line = fixup_separators(&line);
            let cells: Vec<String> = line.split('|').map(|s| s.trim().to_string()).collect();
            result.push((row_counter, cells));
            continue;
        }

        if header_found && !data_found && is_separator(line) {
            data_found = true;
            row_counter += 1;
            continue;
        }

        if header_found && data_found && !is_separator(line) {
            let line = fixup_separators(line);
            let cells: Vec<String> = line.split('|').map(|s| s.trim().to_string()).collect();
            result.push((row_counter, cells));
            continue;
        }

        if header_found && data_found && is_separator(line) {
            row_counter += 1;
            continue;
        }
    }

    result
}

fn get_headers(table: &[(usize, Vec<String>)]) -> Vec<Vec<String>> {
    table
        .iter()
        .filter(|(n, _)| *n == 0)
        .map(|(_, cells)| cells.clone())
        .collect()
}

fn get_data(table: &[(usize, Vec<String>)]) -> Vec<Vec<Vec<String>>> {
    let mut result: Vec<Vec<Vec<String>>> = Vec::new();
    let mut current_row: usize = 1;
    let mut this_row: Vec<Vec<String>> = Vec::new();

    for (row_num, cells) in table {
        if *row_num == 0 {
            continue;
        }
        if *row_num != current_row {
            if !this_row.is_empty() {
                result.push(this_row.clone());
            }
            this_row = Vec::new();
            current_row = *row_num;
        }
        this_row.push(cells.clone());
    }
    if !this_row.is_empty() {
        result.push(this_row);
    }
    result
}

fn collapse_headers(table: &[Vec<String>]) -> Vec<String> {
    if table.is_empty() {
        return Vec::new();
    }
    let mut result = table[0].clone();
    for line in &table[1..] {
        let mut new_line = Vec::new();
        for (i, header) in line.iter().enumerate() {
            let prev = result.get(i).map(|s| s.as_str()).unwrap_or("");
            if !header.is_empty() {
                let combined = format!("{}_{}", prev, header);
                // Remove consecutive underscores
                let combined = UNDERSCORES_RE.replace_all(&combined, "_").to_string();
                new_line.push(combined);
            } else {
                new_line.push(prev.to_string());
            }
        }
        result = new_line;
    }
    result
}

fn collapse_data(table: &[Vec<Vec<String>>], _quiet: bool) -> Vec<Vec<String>> {
    let mut result = Vec::new();
    for row in table {
        if row.is_empty() {
            continue;
        }
        let mut new_row: Vec<String> = Vec::new();
        for cells in row {
            if new_row.is_empty() {
                new_row = cells.clone();
            } else {
                for (i, cell) in cells.iter().enumerate() {
                    if i < new_row.len() {
                        let combined = format!("{}\n{}", new_row[i], cell).trim().to_string();
                        new_row[i] = combined;
                    }
                }
            }
        }
        result.push(new_row);
    }
    result
}

fn create_table_dict(headers: &[String], data: &[Vec<String>]) -> Vec<Map<String, Value>> {
    data.iter()
        .map(|row| {
            let mut obj = Map::new();
            for (i, header) in headers.iter().enumerate() {
                // Lowercase headers
                let h = header.to_lowercase();
                let v = row.get(i).map(|s| s.as_str()).unwrap_or("");
                obj.insert(
                    h,
                    if v.is_empty() {
                        Value::Null
                    } else {
                        Value::String(v.to_string())
                    },
                );
            }
            obj
        })
        .collect()
}

fn parse_pretty(data: &str, quiet: bool) -> Vec<Map<String, Value>> {
    let lines: Vec<&str> = data.lines().collect();
    let clean = normalize_rows(&lines);
    let raw_headers = get_headers(&clean);
    let raw_data = get_data(&clean);

    let new_headers = collapse_headers(&raw_headers);
    let new_data = collapse_data(&raw_data, quiet);

    create_table_dict(&new_headers, &new_data)
}

impl Parser for AsciitableMParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let data = remove_ansi(input);
        let data = strip_table(&data);

        if data.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let table_type = table_sniff(&data);

        let result = match table_type {
            "pretty" => parse_pretty(&data, quiet),
            "markdown" => {
                return Err(ParseError::InvalidInput(
                    "Only \"pretty\" tables supported with multiline. \"markdown\" table detected. Please try the \"asciitable\" parser.".to_string(),
                ))
            }
            _ => {
                return Err(ParseError::InvalidInput(
                    "Only \"pretty\" tables supported with multiline. \"simple\" table detected. Please try the \"asciitable\" parser.".to_string(),
                ))
            }
        };

        Ok(ParseOutput::Array(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asciitable_m_simple() {
        let input = "+----------+---------+--------+\n| foo      | bar     | baz    |\n|          |         | buz    |\n+==========+=========+========+\n| good day | 12345   |        |\n| mate     |         |        |\n+----------+---------+--------+\n| hi there | abc def | 3.14   |\n|          |         |        |\n+==========+=========+========+";
        let parser = AsciitableMParser;
        let result = parser.parse(input, false).unwrap();
        match result {
            ParseOutput::Array(arr) => {
                assert_eq!(arr.len(), 2);
                assert_eq!(arr[0]["foo"], Value::String("good day\nmate".to_string()));
                assert_eq!(arr[0]["bar"], Value::String("12345".to_string()));
                assert_eq!(arr[0]["baz_buz"], Value::Null);
            }
            _ => panic!("expected array"),
        }
    }
}
