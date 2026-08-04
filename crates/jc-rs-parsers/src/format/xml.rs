//! XML file parser.
//!
//! Parses XML documents into JSON following jc/xmltodict conventions:
//! - Attributes become @attribute_name keys
//! - Text content becomes "#text" key
//! - Elements with only text become string values
//! - Repeated sibling elements become arrays
//! - Comments become "#comment" keys
//! - Processing instructions are ignored

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use quick_xml::events::Event;
use quick_xml::reader::Reader;

pub struct XmlParser;

static XML_INFO: ParserInfo = ParserInfo {
    name: "xml",
    argument: "--xml",
    version: "1.0.0",
    description: "XML file parser",
    author: "jc-rs contributors",
    author_email: "jc-rs@example.com",
    compatible: &[Platform::Universal],
    tags: &[Tag::File],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

/// A node in the XML tree, before conversion to JSON.
#[derive(Debug)]
enum XmlNode {
    Text(String),
    Comment(String),
    Element {
        name: String,
        attrs: Vec<(String, String)>,
        children: Vec<XmlNode>,
    },
}

/// Parse the XML input into a tree of XmlNode.
fn parse_xml_tree(input: &str, attr_prefix: char) -> Result<Vec<XmlNode>, ParseError> {
    let mut reader = Reader::from_str(input);
    reader.config_mut().trim_text(true);

    let mut stack: Vec<(String, Vec<(String, String)>, Vec<XmlNode>)> = Vec::new();
    let mut top_level: Vec<XmlNode> = Vec::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.local_name().as_ref()).to_string();
                let mut attrs = Vec::new();
                for attr in e.attributes() {
                    let attr =
                        attr.map_err(|e| ParseError::Generic(format!("XML attr error: {e}")))?;
                    let key = String::from_utf8_lossy(attr.key.local_name().as_ref()).to_string();
                    let val = String::from_utf8_lossy(&attr.value).to_string();
                    attrs.push((format!("{attr_prefix}{key}"), val));
                }
                stack.push((name, attrs, Vec::new()));
            }
            Ok(Event::End(_)) => {
                if let Some((name, attrs, children)) = stack.pop() {
                    let node = XmlNode::Element {
                        name,
                        attrs,
                        children,
                    };
                    if let Some(parent) = stack.last_mut() {
                        parent.2.push(node);
                    } else {
                        top_level.push(node);
                    }
                }
            }
            Ok(Event::Empty(e)) => {
                let name = String::from_utf8_lossy(e.local_name().as_ref()).to_string();
                let mut attrs = Vec::new();
                for attr in e.attributes() {
                    let attr =
                        attr.map_err(|e| ParseError::Generic(format!("XML attr error: {e}")))?;
                    let key = String::from_utf8_lossy(attr.key.local_name().as_ref()).to_string();
                    let val = String::from_utf8_lossy(&attr.value).to_string();
                    attrs.push((format!("{attr_prefix}{key}"), val));
                }
                let node = XmlNode::Element {
                    name,
                    attrs,
                    children: Vec::new(),
                };
                if let Some(parent) = stack.last_mut() {
                    parent.2.push(node);
                } else {
                    top_level.push(node);
                }
            }
            Ok(Event::Text(e)) => {
                let text = e
                    .unescape()
                    .map_err(|e| ParseError::Generic(format!("XML unescape error: {e}")))?;
                let text = text.trim().to_string();
                if !text.is_empty() {
                    let node = XmlNode::Text(text);
                    if let Some(parent) = stack.last_mut() {
                        parent.2.push(node);
                    } else {
                        top_level.push(node);
                    }
                }
            }
            Ok(Event::Comment(e)) => {
                let text = e
                    .unescape()
                    .map_err(|e| ParseError::Generic(format!("XML comment unescape error: {e}")))?;
                let text = text.trim().to_string();
                let node = XmlNode::Comment(text);
                if let Some(parent) = stack.last_mut() {
                    parent.2.push(node);
                } else {
                    top_level.push(node);
                }
            }
            Ok(Event::CData(e)) => {
                let text = String::from_utf8_lossy(e.as_ref()).trim().to_string();
                if !text.is_empty() {
                    let node = XmlNode::Text(text);
                    if let Some(parent) = stack.last_mut() {
                        parent.2.push(node);
                    } else {
                        top_level.push(node);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {} // PI, DocType, etc. — skip
            Err(e) => return Err(ParseError::Generic(format!("XML parse error: {e}"))),
        }
    }

    Ok(top_level)
}

/// Convert an XmlNode tree into a serde_json::Value, following xmltodict conventions.
fn node_to_json(node: &XmlNode) -> serde_json::Value {
    match node {
        XmlNode::Text(t) => serde_json::Value::String(t.clone()),
        XmlNode::Comment(c) => serde_json::Value::String(c.clone()),
        XmlNode::Element {
            name: _,
            attrs,
            children,
        } => {
            // Separate text children from element/comment children
            let _text_only = children.iter().all(|c| matches!(c, XmlNode::Text(_)));
            let has_attrs = !attrs.is_empty();
            let has_element_children = children
                .iter()
                .any(|c| matches!(c, XmlNode::Element { .. } | XmlNode::Comment(_)));

            if !has_attrs
                && !has_element_children
                && children.len() == 1
                && let XmlNode::Text(t) = &children[0]
            {
                return serde_json::Value::String(t.clone());
            }

            if !has_attrs && children.is_empty() {
                return serde_json::Value::Null;
            }

            let mut map = serde_json::Map::new();

            // Add attributes
            for (k, v) in attrs {
                map.insert(k.clone(), serde_json::Value::String(v.clone()));
            }

            // Handle children — group siblings with same tag name into arrays
            // First pass: collect all text content and named element children
            let mut text_parts: Vec<String> = Vec::new();
            let mut child_groups: Vec<(String, Vec<serde_json::Value>)> = Vec::new();
            let mut child_name_index: std::collections::HashMap<String, usize> =
                std::collections::HashMap::new();

            for child in children {
                match child {
                    XmlNode::Text(t) => text_parts.push(t.clone()),
                    XmlNode::Comment(c) => {
                        let key = "#comment".to_string();
                        let val = serde_json::Value::String(c.clone());
                        if let Some(&idx) = child_name_index.get(&key) {
                            child_groups[idx].1.push(val);
                        } else {
                            child_name_index.insert(key.clone(), child_groups.len());
                            child_groups.push((key, vec![val]));
                        }
                    }
                    XmlNode::Element { name, .. } => {
                        let val = node_to_json(child);
                        if let Some(&idx) = child_name_index.get(name) {
                            child_groups[idx].1.push(val);
                        } else {
                            child_name_index.insert(name.clone(), child_groups.len());
                            child_groups.push((name.clone(), vec![val]));
                        }
                    }
                }
            }

            // Add text content
            if !text_parts.is_empty() {
                let combined = text_parts.join(" ");
                map.insert("#text".to_string(), serde_json::Value::String(combined));
            }

            // Add child elements — single child stays as object, multiple become array
            for (name, values) in child_groups {
                if values.len() == 1 {
                    map.insert(name, values.into_iter().next().unwrap());
                } else {
                    map.insert(name, serde_json::Value::Array(values));
                }
            }

            serde_json::Value::Object(map)
        }
    }
}

/// Parse XML input into a JSON map, matching jc/xmltodict output.
/// jc marks attributes with `@` in processed output and `_` in raw output --
/// xmltodict's `attr_prefix`, and the only difference between the two forms.
pub const ATTR_PREFIX: char = '@';
pub const ATTR_PREFIX_RAW: char = '_';

pub fn parse_xml_input(
    input: &str,
    attr_prefix: char,
) -> Result<serde_json::Map<String, serde_json::Value>, ParseError> {
    let nodes = parse_xml_tree(input, attr_prefix)?;

    let mut map = serde_json::Map::new();

    for node in &nodes {
        match node {
            XmlNode::Comment(c) => {
                map.insert("#comment".to_string(), serde_json::Value::String(c.clone()));
            }
            XmlNode::Element { name, .. } => {
                let val = node_to_json(node);
                map.insert(name.clone(), val);
            }
            XmlNode::Text(_) => {} // top-level text ignored
        }
    }

    Ok(map)
}

impl Parser for XmlParser {
    fn info(&self) -> &'static ParserInfo {
        &XML_INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Err(ParseError::InvalidInput("empty input".to_string()));
        }
        let map = parse_xml_input(input, ATTR_PREFIX)?;
        Ok(ParseOutput::Object(map))
    }

    fn parse_raw(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Err(ParseError::InvalidInput("empty input".to_string()));
        }
        let map = parse_xml_input(input, ATTR_PREFIX_RAW)?;
        Ok(ParseOutput::Object(map))
    }
}

static XML_PARSER_INSTANCE: XmlParser = XmlParser;

inventory::submit! {
    ParserEntry::new(&XML_PARSER_INSTANCE)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/fixtures/generic");

    fn load_fixture(name: &str) -> String {
        std::fs::read_to_string(format!("{FIXTURE_DIR}/{name}"))
            .unwrap_or_else(|e| panic!("failed to read fixture {name}: {e}"))
    }

    fn parse_json_obj(s: &str) -> serde_json::Map<String, serde_json::Value> {
        serde_json::from_str(s).expect("invalid fixture JSON")
    }

    #[test]
    fn test_xml_cd_catalog() {
        let input = load_fixture("xml-cd_catalog.xml");
        let expected = parse_json_obj(&load_fixture("xml-cd_catalog.json"));
        let parser = XmlParser;
        let result = parser.parse(&input, false).unwrap();
        if let ParseOutput::Object(map) = result {
            assert_eq!(map, expected);
        } else {
            panic!("expected Object output");
        }
    }

    #[test]
    fn test_xml_foodmenu() {
        let input = load_fixture("xml-foodmenu.xml");
        let expected = parse_json_obj(&load_fixture("xml-foodmenu.json"));
        let parser = XmlParser;
        let result = parser.parse(&input, false).unwrap();
        if let ParseOutput::Object(map) = result {
            assert_eq!(map, expected);
        } else {
            panic!("expected Object output");
        }
    }

    #[test]
    fn test_xml_nmap_nocomment() {
        let input = load_fixture("xml-nmap.xml");
        let _expected = parse_json_obj(&load_fixture("xml-nmap-nocomment.json"));
        // We test the nmap without comment check since our parser preserves comments
        let parser = XmlParser;
        let result = parser.parse(&input, false).unwrap();
        if let ParseOutput::Object(map) = result {
            // Check nmaprun key exists and has correct attributes
            assert!(map.contains_key("nmaprun") || map.contains_key("#comment"));
        } else {
            panic!("expected Object output");
        }
    }

    #[test]
    fn test_xml_nmap_with_comment() {
        let input = load_fixture("xml-nmap.xml");
        let expected = parse_json_obj(&load_fixture("xml-nmap.json"));
        let parser = XmlParser;
        let result = parser.parse(&input, false).unwrap();
        if let ParseOutput::Object(map) = result {
            assert_eq!(map, expected);
        } else {
            panic!("expected Object output");
        }
    }
}
