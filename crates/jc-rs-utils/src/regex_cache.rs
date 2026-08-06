//! A process-wide memo from pattern text to compiled [`Regex`].
//!
//! Most parsers can name their patterns as `static`s and compile them once.
//! A few cannot: they take the pattern as an argument and are handed a
//! different string literal at each of a dozen call sites, inside a loop over
//! input lines. Compiling there costs microseconds per line to match something
//! that takes nanoseconds.
//!
//! The pattern set is closed — every caller passes a literal from this
//! workspace — so entries are leaked rather than reference-counted, and the
//! returned `&'static Regex` needs no guard to be held across a match.

use regex::Regex;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

type Cache = Mutex<HashMap<Box<str>, Option<&'static Regex>>>;

fn cache() -> &'static Cache {
    static CACHE: OnceLock<Cache> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Compile `pattern`, or return the compilation from a previous call.
///
/// Returns `None` for a pattern that does not compile, and remembers that too,
/// so a bad pattern in a loop costs one compilation rather than one per
/// iteration.
pub fn cached_regex(pattern: &str) -> Option<&'static Regex> {
    let mut map = match cache().lock() {
        Ok(map) => map,
        // A poisoned lock means another thread panicked mid-insert. Falling
        // back to compiling is slower but never wrong.
        Err(poisoned) => poisoned.into_inner(),
    };
    if let Some(entry) = map.get(pattern) {
        return *entry;
    }
    let compiled = Regex::new(pattern)
        .ok()
        .map(|re| &*Box::leak(Box::new(re)) as &'static Regex);
    map.insert(pattern.into(), compiled);
    compiled
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_the_same_compiled_regex_for_a_repeated_pattern() {
        let a = cached_regex(r"(\d+) packets").expect("compiles");
        let b = cached_regex(r"(\d+) packets").expect("compiles");
        assert!(std::ptr::eq(a, b), "second call must reuse the first");
    }

    #[test]
    fn compiled_regex_still_matches() {
        let re = cached_regex(r"(\d+) packets transmitted").expect("compiles");
        let caps = re
            .captures("5 packets transmitted, 5 received")
            .expect("matches");
        assert_eq!(&caps[1], "5");
    }

    #[test]
    fn invalid_pattern_returns_none_and_is_remembered() {
        assert!(cached_regex("(unclosed").is_none());
        assert!(cached_regex("(unclosed").is_none());
    }
}
