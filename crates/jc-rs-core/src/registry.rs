//! Global parser registry using the `inventory` crate.
//!
//! Parsers register themselves at link time via `inventory::submit!`.
//! The CLI and library can then discover all available parsers without
//! maintaining a central list.
//!
//! # Registration (in jc-rs-parsers or plugin crates)
//!
//! ```ignore
//! use jc_rs_core::registry::ParserEntry;
//! use jc_rs_core::traits::Parser;
//!
//! struct MyParser;
//! impl Parser for MyParser { /* ... */ }
//!
//! static MY_PARSER: MyParser = MyParser;
//!
//! inventory::submit! {
//!     ParserEntry::new(&MY_PARSER)
//! }
//! ```

use crate::traits::Parser;

/// A wrapper that holds a reference to a statically-allocated parser.
///
/// This is the unit of registration: each parser crate creates a `ParserEntry`
/// and submits it via `inventory::submit!`.
pub struct ParserEntry {
    parser: &'static dyn Parser,
}

impl ParserEntry {
    /// Create a new registry entry wrapping a static parser reference.
    pub const fn new(parser: &'static dyn Parser) -> Self {
        Self { parser }
    }

    /// Get the wrapped parser reference.
    pub fn parser(&self) -> &'static dyn Parser {
        self.parser
    }
}

// Tell inventory how to collect ParserEntry items at link time.
inventory::collect!(ParserEntry);

/// Iterate over all registered parsers.
///
/// The order is determined by the linker and is not guaranteed to be stable.
/// Callers that need sorted output should collect and sort by
/// `parser.info().name`.
pub fn all_parsers() -> impl Iterator<Item = &'static dyn Parser> {
    inventory::iter::<ParserEntry>
        .into_iter()
        .map(|e| e.parser())
}

/// Find a registered parser by its module name (snake_case, e.g. `"apt_get_sqq"`).
///
/// Also accepts the CLI argument form (`"--apt-get-sqq"`) and the kebab-case
/// form (`"apt-get-sqq"`) for convenience. The lookup normalises dashes to
/// underscores before comparing.
pub fn find_parser(name: &str) -> Option<&'static dyn Parser> {
    let normalised = name.trim_start_matches('-').replace('-', "_");
    all_parsers().find(|p| p.info().name == normalised)
}

/// Find a parser whose `magic_commands` list contains a matching command.
///
/// `words` is the argv of the command the user ran (e.g. `["df", "-h"]`).
/// The function joins the words with a space and checks whether any parser's
/// magic commands list contains an entry that matches the beginning of
/// the joined command string.
///
/// Returns the first matching parser, or `None`.
pub fn find_magic_parser(words: &[&str]) -> Option<&'static dyn Parser> {
    if words.is_empty() {
        return None;
    }
    let cmd = words.join(" ");
    let cmd_base = words[0];

    all_parsers().find(|p| {
        p.info().magic_commands.iter().any(|mc| {
            // Match either the full command string or just the base command name
            *mc == cmd || *mc == cmd_base
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ParseError;
    use crate::traits::Parser;
    use crate::types::{ParseOutput, ParserInfo, Platform, Tag};

    // --- Test parsers registered via inventory ---

    struct AlphaParser;
    static ALPHA_INFO: ParserInfo = ParserInfo {
        name: "alpha_parser",
        argument: "--alpha-parser",
        version: "1.0.0",
        description: "Alpha test parser",
        author: "Test",
        author_email: "t@t.com",
        compatible: &[Platform::Linux, Platform::Darwin],
        tags: &[Tag::Command],
        magic_commands: &["alpha_cmd"],
        streaming: false,
        hidden: false,
        deprecated: false,
    };
    impl Parser for AlphaParser {
        fn info(&self) -> &'static ParserInfo {
            &ALPHA_INFO
        }
        fn parse(&self, _: &str, _: bool) -> Result<ParseOutput, ParseError> {
            Ok(ParseOutput::Array(vec![]))
        }
    }
    static ALPHA: AlphaParser = AlphaParser;
    inventory::submit! { ParserEntry::new(&ALPHA) }

    struct BetaParser;
    static BETA_INFO: ParserInfo = ParserInfo {
        name: "beta_parser",
        argument: "--beta-parser",
        version: "1.0.0",
        description: "Beta test parser with two-word magic command",
        author: "Test",
        author_email: "t@t.com",
        compatible: &[Platform::Linux],
        tags: &[Tag::Command],
        magic_commands: &["beta cmd"],
        streaming: false,
        hidden: false,
        deprecated: false,
    };
    impl Parser for BetaParser {
        fn info(&self) -> &'static ParserInfo {
            &BETA_INFO
        }
        fn parse(&self, _: &str, _: bool) -> Result<ParseOutput, ParseError> {
            Ok(ParseOutput::Array(vec![]))
        }
    }
    static BETA: BetaParser = BetaParser;
    inventory::submit! { ParserEntry::new(&BETA) }

    // --- Tests ---

    #[test]
    fn find_parser_exact_name() {
        let p = find_parser("alpha_parser");
        assert!(p.is_some());
        assert_eq!(p.unwrap().info().name, "alpha_parser");
    }

    #[test]
    fn find_parser_dash_to_underscore_normalization() {
        // "alpha-parser" should normalise to "alpha_parser"
        let p = find_parser("alpha-parser");
        assert!(p.is_some());
        assert_eq!(p.unwrap().info().name, "alpha_parser");
    }

    #[test]
    fn find_parser_strips_leading_dashes() {
        // "--alpha-parser" should strip "--" and normalise dashes
        let p = find_parser("--alpha-parser");
        assert!(p.is_some());
        assert_eq!(p.unwrap().info().name, "alpha_parser");
    }

    #[test]
    fn find_parser_returns_none_for_unknown() {
        let p = find_parser("nonexistent_xyz_parser_99");
        assert!(p.is_none());
    }

    #[test]
    fn find_magic_parser_two_word_command() {
        // ["beta", "cmd"] joins to "beta cmd" which matches BETA_INFO magic_commands
        let p = find_magic_parser(&["beta", "cmd"]);
        assert!(p.is_some());
        assert_eq!(p.unwrap().info().name, "beta_parser");
    }

    #[test]
    fn find_magic_parser_one_word_command() {
        // ["alpha_cmd"] matches cmd_base == "alpha_cmd" in ALPHA_INFO magic_commands
        let p = find_magic_parser(&["alpha_cmd"]);
        assert!(p.is_some());
        assert_eq!(p.unwrap().info().name, "alpha_parser");
    }

    #[test]
    fn find_magic_parser_returns_none_for_unknown_command() {
        let p = find_magic_parser(&["totally_unknown_xyz_cmd_99"]);
        assert!(p.is_none());
    }

    #[test]
    fn find_magic_parser_empty_slice_returns_none() {
        let p = find_magic_parser(&[]);
        assert!(p.is_none());
    }

    #[test]
    fn all_parsers_returns_non_empty() {
        let count = all_parsers().count();
        assert!(
            count > 0,
            "Expected at least one registered parser, got {count}"
        );
    }
}
