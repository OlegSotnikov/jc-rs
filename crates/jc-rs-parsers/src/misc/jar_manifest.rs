use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct JarManifestParser;

static INFO: ParserInfo = ParserInfo {
    name: "jar_manifest",
    argument: "--jar-manifest",
    version: "0.1.0",
    description: "Java MANIFEST.MF file parser",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[
        Platform::Linux,
        Platform::Darwin,
        Platform::FreeBSD,
        Platform::Windows,
    ],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static JAR_MANIFEST_PARSER: JarManifestParser = JarManifestParser;

inventory::submit! {
    ParserEntry::new(&JAR_MANIFEST_PARSER)
}

impl Parser for JarManifestParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let mut lines: Vec<&str> = input.lines().collect();

        // Strip trailing "N archives were successfully processed." line
        if let Some(last) = lines.last()
            && last.ends_with("archives were successfully processed.")
        {
            lines.pop();
        }

        // Split into archive blocks on empty lines
        let mut archives: Vec<Vec<&str>> = Vec::new();
        let mut current: Vec<&str> = Vec::new();

        for line in &lines {
            if line.is_empty() {
                if !current.is_empty() {
                    archives.push(current.clone());
                    current.clear();
                }
            } else {
                current.push(line);
            }
        }
        if !current.is_empty() {
            archives.push(current);
        }

        let mut result = Vec::new();

        for mut archive_lines in archives {
            // Remove "inflating: META-INF/MANIFEST.MF" lines
            archive_lines.retain(|l| {
                !l.trim_start().to_lowercase().starts_with("inflating:")
                    || !l.to_lowercase().contains("manifest.mf")
            });

            if archive_lines.is_empty() {
                continue;
            }

            // Process continuation lines (lines starting with whitespace)
            let mut processed: Vec<(String, String)> = Vec::new();

            let mut i = 0;
            while i < archive_lines.len() {
                let line = archive_lines[i];

                if line.starts_with(' ') || line.starts_with('\t') {
                    // Continuation line - append to previous entry
                    if let Some(last) = processed.last_mut() {
                        let continuation = line.trim_start();
                        last.1.push_str(continuation);
                    }
                    i += 1;
                    continue;
                }

                if let Some(colon_pos) = line.find(':') {
                    let key = line[..colon_pos].trim().to_string();
                    let val = line[colon_pos + 1..].trim().to_string();

                    // Look ahead for continuation lines
                    let mut full_val = val;
                    i += 1;
                    while i < archive_lines.len() {
                        let next = archive_lines[i];
                        if next.starts_with(' ') || next.starts_with('\t') {
                            full_val.push_str(next.trim_start());
                            i += 1;
                        } else {
                            break;
                        }
                    }

                    // Normalize key: remove whitespace, replace - with _
                    let normalized_key = key
                        .chars()
                        .filter(|c| !c.is_whitespace())
                        .collect::<String>()
                        .replace('-', "_");

                    processed.push((normalized_key, full_val));
                } else {
                    i += 1;
                }
            }

            if processed.is_empty() {
                continue;
            }

            let mut obj = Map::new();
            for (k, v) in processed {
                obj.insert(k, Value::String(v));
            }
            result.push(obj);
        }

        Ok(ParseOutput::Array(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jar_manifest_simple() {
        let input = "Manifest-Version: 1.0\nBuilt-By: developer\nCreated-By: Apache Maven 3.8.4\n";
        let parser = JarManifestParser;
        let result = parser.parse(input, false).unwrap();
        match result {
            ParseOutput::Array(arr) => {
                assert_eq!(arr.len(), 1);
                assert_eq!(arr[0]["Manifest_Version"], Value::String("1.0".to_string()));
                assert_eq!(arr[0]["Built_By"], Value::String("developer".to_string()));
            }
            _ => panic!("expected array"),
        }
    }

    #[test]
    fn test_jar_manifest_continuation() {
        // Continuation line starts with space
        let input = "Import-Package: com.example;\n resolution:=optional\nManifest-Version: 1.0\n";
        let parser = JarManifestParser;
        let result = parser.parse(input, false).unwrap();
        match result {
            ParseOutput::Array(arr) => {
                assert_eq!(arr.len(), 1);
                assert!(
                    arr[0]["Import_Package"]
                        .as_str()
                        .unwrap()
                        .contains("resolution:=optional")
                );
            }
            _ => panic!("expected array"),
        }
    }

    #[test]
    fn test_jar_manifest_multiple_archives() {
        let input = "Manifest-Version: 1.0\nArchive: archive1.jar\n\nManifest-Version: 1.0\nArchive: archive2.jar\n";
        let parser = JarManifestParser;
        let result = parser.parse(input, false).unwrap();
        match result {
            ParseOutput::Array(arr) => {
                assert_eq!(arr.len(), 2);
            }
            _ => panic!("expected array"),
        }
    }
}
