//! Parser for `mount` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct MountParser;

static INFO: ParserInfo = ParserInfo {
    name: "mount",
    argument: "--mount",
    version: "1.2.0",
    description: "Converts `mount` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux, Platform::Darwin, Platform::FreeBSD],
    tags: &[Tag::Command, Tag::Slurpable],
    magic_commands: &["mount"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static MOUNT_PARSER: MountParser = MountParser;

inventory::submit! {
    ParserEntry::new(&MOUNT_PARSER)
}

impl Parser for MountParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let rows = parse_mount(input);
        Ok(ParseOutput::Array(rows))
    }
}

/// Parse mount output.
///
/// Linux/Linux-like format: "device on mountpoint type fstype (options)"
/// macOS format: "device on mountpoint (options)"  (no "type" keyword)
/// AIX format: tabular with "node mounted mounted-over vfs date options" columns
fn parse_mount(input: &str) -> Vec<Map<String, Value>> {
    let lines: Vec<&str> = input.lines().collect();

    // Detect AIX format: first non-empty line contains "mounted over"
    let first_nonempty = lines.iter().find(|l| !l.trim().is_empty()).copied();
    if let Some(hdr) = first_nonempty
        && hdr.to_lowercase().contains("mounted over")
    {
        return parse_mount_aix(&lines);
    }

    let mut output = Vec::new();
    for line in &lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(record) = parse_mount_line(line) {
            output.push(record);
        }
    }
    output
}

/// Parse AIX `mount` output.
///
/// Format:
/// ```text
///   node       mounted        mounted over    vfs       date        options
///   -------- ---------------  ---------------  ------ ------------ ---------------
///            /dev/hd4         /                jfs2   Sep 06 11:46 rw,log=/dev/hd8
/// ```
///
/// After the dashes line, each data line (with optional leading node column) has:
///   [node] filesystem mount_point vfs month day time options
/// The node column width is determined from the dashes line.
fn parse_mount_aix(lines: &[&str]) -> Vec<Map<String, Value>> {
    // Find width of node column from first dash group in dashes line
    let node_col_end = lines
        .iter()
        .find(|l| l.trim_start().starts_with("---"))
        .and_then(|dl| {
            let start = dl.bytes().position(|b| b == b'-')?;
            let end = dl[start..]
                .bytes()
                .position(|b| b == b' ')
                .map(|p| start + p)?;
            Some(end)
        })
        .unwrap_or(9);

    let mut output = Vec::new();
    let mut past_header = false;

    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        if line.trim_start().starts_with("---") {
            past_header = true;
            continue;
        }
        if !past_header {
            continue;
        }

        // Skip the node column, then tokenize the rest
        let data_part = if line.len() > node_col_end {
            &line[node_col_end..]
        } else {
            line
        };

        // Fields after node: filesystem mount_point vfs month day time [options]
        let tokens: Vec<&str> = data_part.split_whitespace().collect();
        if tokens.len() < 4 {
            continue;
        }

        let filesystem = tokens[0];
        let mount_point = tokens[1];
        let fs_type = tokens[2];
        // tokens[3]=month, tokens[4]=day, tokens[5]=time, tokens[6]=options
        let options_str = tokens.get(6).copied().unwrap_or("");

        let options: Vec<Value> = options_str
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| Value::String(s.to_string()))
            .collect();

        let mut record = Map::new();
        record.insert(
            "filesystem".to_string(),
            Value::String(filesystem.to_string()),
        );
        record.insert(
            "mount_point".to_string(),
            Value::String(mount_point.to_string()),
        );
        record.insert("type".to_string(), Value::String(fs_type.to_string()));
        record.insert("options".to_string(), Value::Array(options));
        output.push(record);
    }

    output
}

fn parse_mount_line(line: &str) -> Option<Map<String, Value>> {
    // Find " on " separator
    let on_pos = line.find(" on ")?;
    let filesystem = line[..on_pos].trim().to_string();
    let rest = &line[on_pos + 4..]; // skip " on "

    // Find options in parens: look for " (" from the right
    // But mountpoint itself can have spaces, so find the last " (" that leads to closing ")"
    let (mount_point, fs_type, options_str) = parse_after_on(rest)?;

    // Parse options: comma-separated list inside parens
    let options: Vec<Value> = options_str
        .split(',')
        .map(|s| Value::String(s.trim().to_string()))
        .filter(|v| {
            if let Value::String(s) = v {
                !s.is_empty()
            } else {
                true
            }
        })
        .collect();

    let mut record = Map::new();
    record.insert("filesystem".to_string(), Value::String(filesystem));
    record.insert("mount_point".to_string(), Value::String(mount_point));
    if let Some(t) = fs_type {
        record.insert("type".to_string(), Value::String(t));
    }
    record.insert("options".to_string(), Value::Array(options));

    Some(record)
}

/// Parse the part after "device on ": split into mountpoint, optional type, options.
///
/// Format possibilities:
///   1. `"/sys type sysfs (rw,nosuid)"`  → mountpoint="/sys", type="sysfs", opts="rw,nosuid"
///   2. `"/ (apfs, sealed, local)"` → mountpoint="/", type=None, opts="apfs, sealed, local"
///   3. `"/media/foo bar type iso9660 (ro,nosuid)"` → mountpoint="/media/foo bar", type=..., opts=...
fn parse_after_on(rest: &str) -> Option<(String, Option<String>, String)> {
    // Find the last opening paren that has a closing paren at the end
    let close_paren = rest.rfind(')')?;
    let open_paren = rest[..close_paren].rfind('(')?;

    let options_str = rest[open_paren + 1..close_paren].to_string();
    let before_parens = rest[..open_paren].trim();

    // Check if "type FSTYPE" appears before the parens
    // Look for " type " pattern
    if let Some(type_pos) = find_type_keyword(before_parens) {
        let mount_point = before_parens[..type_pos].trim().to_string();
        let fs_type = before_parens[type_pos + 6..].trim().to_string(); // " type " is 6 chars
        Some((mount_point, Some(fs_type), options_str))
    } else {
        // No type keyword — macOS style
        Some((before_parens.to_string(), None, options_str))
    }
}

/// Find the position of " type " (with spaces) in the string.
/// Returns the position of the space before "type".
fn find_type_keyword(s: &str) -> Option<usize> {
    s.find(" type ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mount_centos() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/mount.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/mount.json"
        ))
        .unwrap();

        let parser = MountParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_mount_ubuntu18() {
        let input = include_str!("../../../../tests/fixtures/ubuntu-18.04/mount.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/ubuntu-18.04/mount.json"
        ))
        .unwrap();

        let parser = MountParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_mount_osx() {
        let input = include_str!("../../../../tests/fixtures/osx-10.14.6/mount.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/osx-10.14.6/mount.json"
        ))
        .unwrap();

        let parser = MountParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_mount_spaces_in_filename() {
        let input = include_str!("../../../../tests/fixtures/generic/mount-spaces-in-filename.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/mount-spaces-in-filename.json"
        ))
        .unwrap();

        let parser = MountParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_mount_spaces_in_mountpoint() {
        let input =
            include_str!("../../../../tests/fixtures/generic/mount-spaces-in-mountpoint.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/mount-spaces-in-mountpoint.json"
        ))
        .unwrap();

        let parser = MountParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_mount_parens_in_filesystem() {
        let input =
            include_str!("../../../../tests/fixtures/generic/mount-parens-in-filesystem.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/mount-parens-in-filesystem.json"
        ))
        .unwrap();

        let parser = MountParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }
}
