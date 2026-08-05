//! Magic-mode execution: run a shell command and pipe its stdout to a parser.
//!
//! Mirrors `JcCli::do_magic()` and `JcCli::run_user_command()` from the original Python.

use jc_rs_core::{find_magic_parser, traits::Parser};
use std::fs;
use std::process::Command;

/// Result of a magic execution.
pub struct MagicResult {
    /// The matched parser (static reference).
    pub parser: &'static dyn Parser,
    /// The stdout captured from the command (or file contents for /proc).
    pub stdout: String,
    /// The stderr captured (printed to stderr before parsing).
    pub stderr: String,
    /// The exit code of the subprocess (0 for /proc reads).
    pub exit_code: i32,
    /// For multiple /proc files: the list of file paths.
    pub input_list: Option<Vec<String>>,
    /// True if multiple /proc files were read (slurp mode should be enabled).
    pub multi_proc: bool,
}

/// Try to find a parser and execute the magic command.
///
/// Returns `Ok(MagicResult)` on success.
/// Returns `Err(String)` with a human-readable error message on failure.
pub fn run_magic(words: &[String]) -> Result<MagicResult, String> {
    if words.is_empty() {
        return Err("No command provided for magic mode.".to_string());
    }

    // -----------------------------------------------------------------------
    // /proc special case: if the first word starts with /proc, read file(s)
    // -----------------------------------------------------------------------
    if words[0].starts_with("/proc") {
        return run_proc_magic(words);
    }

    // -----------------------------------------------------------------------
    // Normal magic: find parser by two-word or one-word command
    // -----------------------------------------------------------------------
    let word_refs: Vec<&str> = words.iter().map(|s| s.as_str()).collect();
    // Try two-word match first
    let parser = if words.len() >= 2 {
        let two_words: Vec<&str> = vec![word_refs[0], word_refs[1]];
        find_magic_parser(&two_words).or_else(|| find_magic_parser(&[word_refs[0]]))
    } else {
        find_magic_parser(&[word_refs[0]])
    };

    let parser = match parser {
        Some(p) => p,
        None => {
            let cmd_str = words.join(" ");
            return Err(format!(
                "\"{}\" cannot be used with Magic syntax. Use \"jc-rs -h\" for help.",
                cmd_str
            ));
        }
    };

    // Execute the command
    match Command::new(&words[0]).args(&words[1..]).output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let stdout = if stdout.is_empty() {
                "\n".to_string()
            } else {
                stdout
            };
            let exit_code = output.status.code().unwrap_or(0);
            Ok(MagicResult {
                parser,
                stdout,
                stderr,
                exit_code,
                input_list: None,
                multi_proc: false,
            })
        }
        Err(e) => {
            let cmd_str = words.join(" ");
            Err(format!("\"{}\" command could not be run: {}.", cmd_str, e))
        }
    }
}

/// Handle /proc file reading (single or multiple files).
fn run_proc_magic(words: &[String]) -> Result<MagicResult, String> {
    // Find the "proc" parser
    let parser = jc_rs_core::find_parser("proc").ok_or_else(|| {
        "proc parser not found. Make sure jc-rs-parsers includes the proc parser.".to_string()
    })?;

    if words.len() == 1 {
        // Single /proc file
        let path = &words[0];
        match fs::read_to_string(path) {
            Ok(content) => Ok(MagicResult {
                parser,
                stdout: content,
                stderr: String::new(),
                exit_code: 0,
                input_list: None,
                multi_proc: false,
            }),
            Err(e) => Err(format!("\"{}\" file could not be opened: {}.", path, e)),
        }
    } else {
        // Multiple /proc files → slurp mode
        let mut contents: Vec<String> = Vec::new();
        let file_list: Vec<String> = words.to_vec();

        for path in &file_list {
            match fs::read_to_string(path) {
                Ok(content) => contents.push(content),
                Err(e) => {
                    return Err(format!("\"{}\" file could not be opened: {}.", path, e));
                }
            }
        }

        // Concatenate with a separator; the proc parser will need to handle
        // each chunk. We pass the joined content and the input_list separately.
        // The CLI will handle the multi-proc slurp.
        let joined = contents.join("\n---proc-separator---\n");

        Ok(MagicResult {
            parser,
            stdout: joined,
            stderr: String::new(),
            exit_code: 0,
            input_list: Some(file_list),
            multi_proc: true,
        })
    }
}
