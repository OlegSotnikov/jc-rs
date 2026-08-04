// Parser implementations — registered via inventory crate.

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

// Example/test parser — proves registration works
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
