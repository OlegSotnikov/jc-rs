//! Every jc-rs parser, and a small API for using them from your own code.
//!
//! ```
//! let df = "Filesystem     1K-blocks    Used Available Use% Mounted on\n\
//!           devtmpfs         1918816       0   1918816   0% /dev";
//!
//! let output = jc_rs_parsers::parse("df", df).unwrap();
//! let json = serde_json::to_value(output).unwrap();
//! assert_eq!(json[0]["filesystem"], "devtmpfs");
//! assert_eq!(json[0]["use_percent"], 0);
//! ```
//!
//! Parsers register themselves at link time, so depending on this crate is what
//! populates the registry. [`parse`] and [`parsers`] exist partly so that you
//! never have to know that. Reaching for `jc_rs_core::find_parser` from a crate
//! that does not otherwise reference this one returns `None`, because the
//! linker dropped the parsers as unused.
//!
//! Streaming parsers can be driven a line at a time:
//!
//! ```
//! let mut session = jc_rs_parsers::session("clf_s").unwrap();
//! let line = r#"127.0.0.1 - frank [10/Oct/2000:13:55:36 -0700] "GET /a HTTP/1.0" 200 512"#;
//! let record = session.parse_line(line, true).unwrap().unwrap();
//! assert_eq!(record["status"], 200);
//! ```

use jc_rs_core::{LineParser, ParseError, ParseOutput, Parser};

pub use jc_rs_core::{ParserInfo, Platform, Record, Tag};

/// Parse `input` with the named parser.
///
/// The name is the jc parser name in any of its spellings: `"git_log"`,
/// `"git-log"` or `"--git-log"`.
///
/// # Errors
///
/// Returns [`ParseError::InvalidInput`] if no parser has that name, and
/// whatever the parser returns if the input does not fit its format.
pub fn parse(name: &str, input: &str) -> Result<ParseOutput, ParseError> {
    let parser =
        find(name).ok_or_else(|| ParseError::InvalidInput(format!("no parser named {name:?}")))?;
    parser.parse(input, true)
}

/// Look up a parser by name, in any of its spellings.
pub fn find(name: &str) -> Option<&'static dyn Parser> {
    jc_rs_core::find_parser(name)
}

/// Every registered parser, for building your own listing or dispatch.
pub fn parsers() -> impl Iterator<Item = &'static dyn Parser> {
    jc_rs_core::all_parsers()
}

/// Start a line-at-a-time session for a streaming parser.
///
/// Returns `None` if the name is unknown or the parser is not a streaming one.
/// Feed it lines as they arrive and call `finalize()` at the end; see
/// [`jc_rs_core::LineParser`].
pub fn session(name: &str) -> Option<Box<dyn LineParser>> {
    find(name)?.as_streaming().map(|p| p.session())
}

pub mod network;
pub mod universal;

pub mod format;

pub mod system;

pub mod log;
pub mod string;

pub mod proc;

pub mod disk;

pub mod misc;
pub mod package;
pub mod security;

// Example/test parser: proves registration works
mod dummy;

#[cfg(test)]
mod tests {
    use jc_rs_core::registry::{all_parsers, find_parser};

    #[test]
    fn test_dummy_parser_registered() {
        let parser = find_parser("dummy");
        assert!(parser.is_some(), "dummy parser should be registered");
        let p = parser.unwrap();
        assert_eq!(p.info().name, "dummy");
        assert_eq!(p.info().argument, "--dummy");
    }

    #[test]
    fn test_all_parsers_non_empty() {
        let count = all_parsers().count();
        assert!(count >= 1, "at least the dummy parser should be registered");
    }

    #[test]
    fn test_dummy_parser_parse() {
        let parser = find_parser("dummy").unwrap();
        let result = parser.parse("hello world", false);
        assert!(result.is_ok());
    }
}
