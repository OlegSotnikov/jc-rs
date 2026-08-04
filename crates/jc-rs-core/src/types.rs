use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Output from a parser -- either a single JSON object or an array of objects.
///
/// Standard parsers return `Object` for single-record commands (e.g. `date`,
/// `uname`) and `Array` for multi-record commands (e.g. `ps`, `ls`).
/// Streaming parsers yield one `Object` per line.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ParseOutput {
    /// A single JSON object (key-value map).
    Object(serde_json::Map<String, Value>),
    /// An ordered list of JSON objects.
    Array(Vec<serde_json::Map<String, Value>>),
}

/// Platform compatibility tag.
///
/// Maps to jc's `compatible` list entries. `Universal` means the parser works
/// on all platforms (typically for file/string parsers).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Linux,
    Darwin,
    Windows,
    #[serde(rename = "freebsd")]
    FreeBSD,
    #[serde(rename = "openbsd")]
    OpenBSD,
    #[serde(rename = "netbsd")]
    NetBSD,
    Aix,
    Universal,
}

/// Semantic tag for categorizing parsers.
///
/// Mirrors jc's tag system. A parser may have multiple tags.
/// - `Command`: parses the stdout of a CLI command
/// - `File`: parses the contents of a file format
/// - `String`: parses a string/text pattern
/// - `Slurpable`: supports `--slurp` mode (multiple inputs merged)
/// - `Streaming`: implements `StreamingParser` for line-by-line processing
/// - `Hidden`: not shown in default help listings
/// - `Deprecated`: scheduled for removal
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tag {
    Command,
    File,
    String,
    Slurpable,
    Streaming,
    Hidden,
    Deprecated,
}

/// Static metadata describing a parser.
///
/// Every parser must expose a `&'static ParserInfo` via the `Parser::info()`
/// method. All fields use `&'static` references so the info can live in
/// read-only data without allocation.
///
/// This struct intentionally does NOT derive `Deserialize` because of the
/// `&'static` references.
#[derive(Debug, Clone, Serialize)]
pub struct ParserInfo {
    /// Module name in snake_case (e.g. `"apt_get_sqq"`)
    pub name: &'static str,
    /// CLI argument form with `--` prefix (e.g. `"--apt-get-sqq"`)
    pub argument: &'static str,
    /// Semantic version string (e.g. `"1.0.0"`)
    pub version: &'static str,
    /// Human-readable one-line description
    pub description: &'static str,
    /// Author name
    pub author: &'static str,
    /// Author email
    pub author_email: &'static str,
    /// Platforms this parser is compatible with
    pub compatible: &'static [Platform],
    /// Semantic tags for this parser
    pub tags: &'static [Tag],
    /// Shell commands that trigger magic-mode auto-detection.
    /// Each entry is the command basename (e.g. `"df"`, `"ls -al"`).
    pub magic_commands: &'static [&'static str],
    /// Whether this parser implements `StreamingParser`
    pub streaming: bool,
    /// Whether this parser is hidden from default listings
    pub hidden: bool,
    /// Whether this parser is deprecated
    pub deprecated: bool,
}

impl ParserInfo {
    /// Returns `true` if the parser has the given tag.
    pub fn has_tag(&self, tag: Tag) -> bool {
        self.tags.contains(&tag)
    }

    /// Returns `true` if the parser is compatible with the given platform.
    pub fn is_compatible_with(&self, platform: Platform) -> bool {
        self.compatible.contains(&Platform::Universal) || self.compatible.contains(&platform)
    }

    /// Returns `true` if this parser supports `--slurp` mode.
    pub fn is_slurpable(&self) -> bool {
        self.has_tag(Tag::Slurpable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Map, json};

    // --- ParseOutput serialization ---

    #[test]
    fn parse_output_object_serializes_to_json_object() {
        let mut map = Map::new();
        map.insert("key".to_string(), json!("value"));
        let output = ParseOutput::Object(map);
        let s = serde_json::to_string(&output).unwrap();
        assert_eq!(s, r#"{"key":"value"}"#);
    }

    #[test]
    fn parse_output_array_serializes_to_json_array() {
        let mut map = Map::new();
        map.insert("n".to_string(), json!(42));
        let output = ParseOutput::Array(vec![map]);
        let s = serde_json::to_string(&output).unwrap();
        assert_eq!(s, r#"[{"n":42}]"#);
    }

    #[test]
    fn parse_output_empty_array_serializes_correctly() {
        let output = ParseOutput::Array(vec![]);
        let s = serde_json::to_string(&output).unwrap();
        assert_eq!(s, "[]");
    }

    // --- ParserInfo helpers ---

    static INFO_WITH_UNIVERSAL: ParserInfo = ParserInfo {
        name: "test_universal",
        argument: "--test-universal",
        version: "1.0.0",
        description: "universal test parser",
        author: "Author",
        author_email: "a@b.com",
        compatible: &[Platform::Universal],
        tags: &[Tag::Command, Tag::Slurpable],
        magic_commands: &[],
        streaming: false,
        hidden: false,
        deprecated: false,
    };

    static INFO_LINUX_ONLY: ParserInfo = ParserInfo {
        name: "test_linux_only",
        argument: "--test-linux-only",
        version: "1.0.0",
        description: "linux-only test parser",
        author: "Author",
        author_email: "a@b.com",
        compatible: &[Platform::Linux],
        tags: &[Tag::File],
        magic_commands: &[],
        streaming: false,
        hidden: false,
        deprecated: false,
    };

    #[test]
    fn has_tag_returns_true_for_present_tag() {
        assert!(INFO_WITH_UNIVERSAL.has_tag(Tag::Command));
        assert!(INFO_WITH_UNIVERSAL.has_tag(Tag::Slurpable));
    }

    #[test]
    fn has_tag_returns_false_for_absent_tag() {
        assert!(!INFO_WITH_UNIVERSAL.has_tag(Tag::File));
        assert!(!INFO_WITH_UNIVERSAL.has_tag(Tag::Hidden));
    }

    #[test]
    fn is_compatible_with_universal_matches_linux() {
        assert!(INFO_WITH_UNIVERSAL.is_compatible_with(Platform::Linux));
    }

    #[test]
    fn is_compatible_with_universal_matches_macos() {
        assert!(INFO_WITH_UNIVERSAL.is_compatible_with(Platform::Darwin));
    }

    #[test]
    fn is_compatible_with_universal_matches_windows() {
        assert!(INFO_WITH_UNIVERSAL.is_compatible_with(Platform::Windows));
    }

    #[test]
    fn is_compatible_with_direct_platform_match() {
        assert!(INFO_LINUX_ONLY.is_compatible_with(Platform::Linux));
    }

    #[test]
    fn is_compatible_with_returns_false_for_unlisted_platform() {
        assert!(!INFO_LINUX_ONLY.is_compatible_with(Platform::Darwin));
        assert!(!INFO_LINUX_ONLY.is_compatible_with(Platform::Windows));
    }

    #[test]
    fn is_slurpable_returns_true_when_tag_present() {
        assert!(INFO_WITH_UNIVERSAL.is_slurpable());
    }

    #[test]
    fn is_slurpable_returns_false_when_tag_absent() {
        assert!(!INFO_LINUX_ONLY.is_slurpable());
    }
}
