//! Parser for Debian package index files.
//!
//! Uses the same key:value block format as rpm_qi / apt_cache_show.

use crate::package::rpm_qi::{parse_raw, process};
use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};

pub struct PkgIndexDebParser;

static INFO: ParserInfo = ParserInfo {
    name: "pkg_index_deb",
    argument: "--pkg-index-deb",
    version: "1.0.0",
    description: "Debian Package Index file parser",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[
        Platform::Linux,
        Platform::Darwin,
        Platform::Windows,
        Platform::Aix,
        Platform::FreeBSD,
    ],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static PKG_INDEX_DEB_PARSER: PkgIndexDebParser = PkgIndexDebParser;

inventory::submit! {
    ParserEntry::new(&PKG_INDEX_DEB_PARSER)
}

impl Parser for PkgIndexDebParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let raw = parse_raw(input);
        let processed = process(raw);
        Ok(ParseOutput::Array(processed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pkg_index_deb_smoke() {
        let input = "Package: foo\nVersion: 1.0\nInstalled-Size: 100\nDepends: bar, baz\nDescription-en: A test package\n short description\n";
        let parser = PkgIndexDebParser;
        let result = parser.parse(input, false).unwrap();
        if let jc_rs_core::types::ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 1);
            assert_eq!(
                arr[0].get("package"),
                Some(&serde_json::Value::String("foo".into()))
            );
        } else {
            panic!("Expected Array");
        }
    }
}
