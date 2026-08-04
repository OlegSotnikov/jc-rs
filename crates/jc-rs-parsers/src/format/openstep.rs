//! Old-style (NeXTSTEP / OpenStep) property lists.
//!
//! The format has no types: `1`, `0700` and `YES` are all just text, and jc --
//! which reads these with `pbPlist` -- reports them as strings. The `plist`
//! crate parses the syntax but coerces bare tokens to numbers, which loses the
//! leading zero in `0700` and cannot be recovered afterwards. Hence a parser
//! that keeps the token exactly as written.

use jc_rs_core::error::ParseError;
use serde_json::{Map, Value};

/// Does this look like an old-style plist rather than XML or binary?
pub(crate) fn looks_like_openstep(input: &str) -> bool {
    if input.starts_with("bplist00") {
        return false;
    }
    // Skip the leading comment banner Xcode writes (`// !$*UTF8*$!`) and any
    // whitespace, then see what the document actually opens with.
    let mut rest = input.trim_start_matches('\u{feff}').trim_start();
    loop {
        if let Some(after) = rest.strip_prefix("//") {
            rest = after
                .split_once('\n')
                .map_or("", |(_, tail)| tail)
                .trim_start();
        } else if let Some(after) = rest.strip_prefix("/*") {
            rest = after
                .split_once("*/")
                .map_or("", |(_, tail)| tail)
                .trim_start();
        } else {
            break;
        }
    }
    rest.starts_with('{') || rest.starts_with('(')
}

pub(crate) fn parse(input: &str) -> Result<Map<String, Value>, ParseError> {
    let mut parser = Parser {
        chars: input.as_bytes(),
        pos: 0,
    };
    parser.skip_trivia();
    let value = parser.value()?;
    match value {
        Value::Object(map) => Ok(map),
        other => {
            let mut map = Map::with_capacity(1);
            map.insert("value".to_string(), other);
            Ok(map)
        }
    }
}

struct Parser<'a> {
    chars: &'a [u8],
    pos: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.chars.get(self.pos).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let byte = self.peek()?;
        self.pos += 1;
        Some(byte)
    }

    fn error(&self, what: &str) -> ParseError {
        ParseError::InvalidInput(format!("openstep plist: {what} at byte {}", self.pos))
    }

    /// Whitespace and both comment styles.
    fn skip_trivia(&mut self) {
        loop {
            match self.peek() {
                Some(b) if b.is_ascii_whitespace() => {
                    self.pos += 1;
                }
                Some(b'/') if self.chars.get(self.pos + 1) == Some(&b'/') => {
                    while let Some(b) = self.bump() {
                        if b == b'\n' {
                            break;
                        }
                    }
                }
                Some(b'/') if self.chars.get(self.pos + 1) == Some(&b'*') => {
                    self.pos += 2;
                    while self.pos < self.chars.len() {
                        if self.chars[self.pos] == b'*'
                            && self.chars.get(self.pos + 1) == Some(&b'/')
                        {
                            self.pos += 2;
                            break;
                        }
                        self.pos += 1;
                    }
                }
                _ => return,
            }
        }
    }

    fn value(&mut self) -> Result<Value, ParseError> {
        self.skip_trivia();
        match self.peek().ok_or_else(|| self.error("unexpected end"))? {
            b'{' => self.dictionary(),
            b'(' => self.array(),
            b'<' => self.data(),
            b'"' => Ok(Value::String(self.quoted_string()?)),
            _ => Ok(Value::String(self.bare_string()?)),
        }
    }

    fn dictionary(&mut self) -> Result<Value, ParseError> {
        self.pos += 1; // '{'
        let mut map = Map::new();

        loop {
            self.skip_trivia();
            match self.peek() {
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(Value::Object(map));
                }
                None => return Err(self.error("unterminated dictionary")),
                _ => {}
            }

            let key = if self.peek() == Some(b'"') {
                self.quoted_string()?
            } else {
                self.bare_string()?
            };

            self.skip_trivia();
            if self.peek() != Some(b'=') {
                return Err(self.error("expected '=' after key"));
            }
            self.pos += 1;

            let value = self.value()?;
            map.insert(key, value);

            self.skip_trivia();
            if self.peek() == Some(b';') {
                self.pos += 1;
            }
        }
    }

    fn array(&mut self) -> Result<Value, ParseError> {
        self.pos += 1; // '('
        let mut items = Vec::new();

        loop {
            self.skip_trivia();
            match self.peek() {
                Some(b')') => {
                    self.pos += 1;
                    return Ok(Value::Array(items));
                }
                None => return Err(self.error("unterminated array")),
                _ => {}
            }

            items.push(self.value()?);

            self.skip_trivia();
            if self.peek() == Some(b',') {
                self.pos += 1;
            }
        }
    }

    /// `<0a1b 2c3d>` -- reported the way the rest of the plist parser reports
    /// binary data, as colon-separated hex.
    fn data(&mut self) -> Result<Value, ParseError> {
        self.pos += 1; // '<'
        let start = self.pos;
        while self.peek().is_some_and(|b| b != b'>') {
            self.pos += 1;
        }
        if self.peek() != Some(b'>') {
            return Err(self.error("unterminated data"));
        }
        let hex: String = std::str::from_utf8(&self.chars[start..self.pos])
            .map_err(|_| self.error("data is not utf-8"))?
            .chars()
            .filter(|c| c.is_ascii_hexdigit())
            .collect();
        self.pos += 1; // '>'

        let pairs: Vec<String> = hex
            .as_bytes()
            .chunks(2)
            .map(|c| String::from_utf8_lossy(c).to_string())
            .collect();
        Ok(Value::String(pairs.join(":")))
    }

    fn quoted_string(&mut self) -> Result<String, ParseError> {
        self.pos += 1; // opening quote
        let mut out = String::new();

        loop {
            match self
                .bump()
                .ok_or_else(|| self.error("unterminated string"))?
            {
                b'"' => return Ok(out),
                b'\\' => {
                    let escaped = self.bump().ok_or_else(|| self.error("dangling escape"))?;
                    out.push(match escaped {
                        b'n' => '\n',
                        b't' => '\t',
                        b'r' => '\r',
                        other => other as char,
                    });
                }
                byte => {
                    // Re-assemble multi-byte UTF-8 one byte at a time.
                    let start = self.pos - 1;
                    let len = utf8_len(byte);
                    let end = (start + len).min(self.chars.len());
                    out.push_str(&String::from_utf8_lossy(&self.chars[start..end]));
                    self.pos = end;
                }
            }
        }
    }

    /// An unquoted token. Kept verbatim -- `0700` must not become `700`.
    fn bare_string(&mut self) -> Result<String, ParseError> {
        let start = self.pos;
        while self
            .peek()
            .is_some_and(|b| b.is_ascii_alphanumeric() || b"_$+/:.-".contains(&b) || b >= 0x80)
        {
            self.pos += 1;
        }
        if self.pos == start {
            return Err(self.error("expected a value"));
        }
        Ok(String::from_utf8_lossy(&self.chars[start..self.pos]).to_string())
    }
}

fn utf8_len(first: u8) -> usize {
    match first {
        0x00..=0x7f => 1,
        0xc0..=0xdf => 2,
        0xe0..=0xef => 3,
        _ => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_tokens_keep_their_text() {
        let out = parse("{ archiveVersion = 1; flags = 0700; }").unwrap();
        assert_eq!(out["archiveVersion"], "1");
        assert_eq!(out["flags"], "0700", "a leading zero is part of the token");
    }

    #[test]
    fn comments_and_nesting() {
        let input = r#"// !$*UTF8*$!
{
    objects = {
        A1 /* a target */ = {
            isa = PBXGroup;
            children = ( B2, C3 );
        };
    };
}"#;
        let out = parse(input).unwrap();
        assert_eq!(out["objects"]["A1"]["isa"], "PBXGroup");
        assert_eq!(out["objects"]["A1"]["children"][1], "C3");
    }

    #[test]
    fn quoted_strings_keep_spaces_and_escapes() {
        let out = parse(r#"{ name = "Lemon Setup"; path = "a\"b"; }"#).unwrap();
        assert_eq!(out["name"], "Lemon Setup");
        assert_eq!(out["path"], "a\"b");
    }

    #[test]
    fn detects_the_format() {
        assert!(looks_like_openstep("// !$*UTF8*$!\n{ a = 1; }"));
        assert!(looks_like_openstep("{ a = 1; }"));
        assert!(!looks_like_openstep("<?xml version=\"1.0\"?><plist/>"));
        assert!(!looks_like_openstep("bplist00"));
    }
}
