use thiserror::Error;

/// Errors that occur during parsing of input data.
///
/// Parser implementations should use these variants to report failures
/// within their `parse()` or `parse_line()` methods.
#[derive(Debug, Error)]
pub enum ParseError {
    /// A catch-all parse failure with a descriptive message.
    #[error("parse error: {0}")]
    Generic(String),

    /// The input data does not match the expected format for this parser.
    #[error("unexpected input format: {0}")]
    InvalidInput(String),

    /// The input contained invalid UTF-8 sequences.
    #[error("utf-8 decode error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    /// JSON serialization or deserialization failed.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// A regular expression failed to compile or match unexpectedly.
    #[error("regex error: {0}")]
    Regex(String),
}

/// Top-level errors for the jc-rs application.
///
/// These wrap `ParseError` with additional context (e.g. which parser failed)
/// and cover non-parse failures such as I/O and slice operations. The CLI
/// layer should convert these into user-facing messages and exit codes.
#[derive(Debug, Error)]
pub enum CjError {
    /// The requested parser name does not match any registered parser.
    #[error("parser not found: {0}")]
    ParserNotFound(String),

    /// A parser was found but its `parse()` call failed.
    #[error("parse failed for '{parser}': {source}")]
    ParseFailed {
        parser: String,
        #[source]
        source: ParseError,
    },

    /// An I/O error (reading stdin, files, etc.).
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// An error in `--slice` range parsing or application.
    #[error("slice error: {0}")]
    SliceError(String),

    /// Any other error that does not fit the above categories.
    #[error("{0}")]
    Other(String),
}

impl From<ParseError> for CjError {
    fn from(e: ParseError) -> Self {
        CjError::ParseFailed {
            parser: String::from("<unknown>"),
            source: e,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- ParseError display ---

    #[test]
    fn parse_error_invalid_input_display() {
        let e = ParseError::InvalidInput("bad format".to_string());
        assert_eq!(e.to_string(), "unexpected input format: bad format");
    }

    #[test]
    fn parse_error_generic_display() {
        let e = ParseError::Generic("something went wrong".to_string());
        assert_eq!(e.to_string(), "parse error: something went wrong");
    }

    #[test]
    fn parse_error_regex_display() {
        let e = ParseError::Regex("invalid regex".to_string());
        assert_eq!(e.to_string(), "regex error: invalid regex");
    }

    #[test]
    fn parse_error_json_display() {
        // Construct a serde_json error via actual deserialization failure
        let json_err: Result<serde_json::Value, _> = serde_json::from_str("{bad}");
        let e = ParseError::Json(json_err.unwrap_err());
        assert!(e.to_string().starts_with("json error:"));
    }

    #[test]
    fn parse_error_debug_contains_variant_name() {
        let e = ParseError::Generic("test".to_string());
        assert!(format!("{e:?}").contains("Generic"));
    }

    // --- CjError display ---

    #[test]
    fn cj_error_parser_not_found_display() {
        let e = CjError::ParserNotFound("df".to_string());
        assert_eq!(e.to_string(), "parser not found: df");
    }

    #[test]
    fn cj_error_parse_failed_display() {
        let e = CjError::ParseFailed {
            parser: "df".to_string(),
            source: ParseError::Generic("failed".to_string()),
        };
        assert_eq!(e.to_string(), "parse failed for 'df': parse error: failed");
    }

    #[test]
    fn cj_error_slice_error_display() {
        let e = CjError::SliceError("out of range".to_string());
        assert_eq!(e.to_string(), "slice error: out of range");
    }

    #[test]
    fn cj_error_other_display() {
        let e = CjError::Other("some other error".to_string());
        assert_eq!(e.to_string(), "some other error");
    }

    #[test]
    fn cj_error_debug_contains_variant_name() {
        let e = CjError::ParserNotFound("test".to_string());
        assert!(format!("{e:?}").contains("ParserNotFound"));
    }

    // --- From<ParseError> for CjError ---

    #[test]
    fn cj_error_from_parse_error_uses_unknown_parser() {
        let pe = ParseError::InvalidInput("bad input".to_string());
        let ce = CjError::from(pe);
        match ce {
            CjError::ParseFailed { parser, source } => {
                assert_eq!(parser, "<unknown>");
                assert_eq!(source.to_string(), "unexpected input format: bad input");
            }
            _ => panic!("expected ParseFailed variant"),
        }
    }

    #[test]
    fn cj_error_from_parse_error_generic_variant() {
        let pe = ParseError::Generic("oops".to_string());
        let ce = CjError::from(pe);
        match ce {
            CjError::ParseFailed { parser, .. } => assert_eq!(parser, "<unknown>"),
            _ => panic!("expected ParseFailed variant"),
        }
    }
}
