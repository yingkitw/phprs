//! SimpleXML and XML support.
//!
//! Implements `simplexml_load_string`, `simplexml_load_file`, and the
//! `SimpleXMLElement` class using a lightweight recursive-descent XML parser.
//! Also provides `XMLReader` for streaming-style XML parsing.

use std::collections::HashMap;

/// A simple XML node representing an element with optional attributes,
/// children, and text content.
#[derive(Debug, Clone)]
pub struct XmlNode {
    pub name: String,
    pub attributes: HashMap<String, String>,
    pub children: Vec<XmlNode>,
    pub text: String,
}

impl XmlNode {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            attributes: HashMap::new(),
            children: Vec::new(),
            text: String::new(),
        }
    }
}

/// Parse an XML string into an XmlNode tree.
pub fn parse_xml(input: &str) -> Result<XmlNode, String> {
    let mut parser = XmlParser::new(input);
    parser.skip_decl_and_whitespace();
    parser.parse_element()
}

struct XmlParser<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> XmlParser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input: input.as_bytes(), pos: 0 }
    }

    fn skip_decl_and_whitespace(&mut self) {
        loop {
            self.skip_whitespace();
            if self.starts_with("<?") {
                self.skip_until("?>");
            } else if self.starts_with("<!--") {
                self.skip_until("-->");
            } else if self.starts_with("<!DOCTYPE") || self.starts_with("<!") {
                self.skip_until(">");
            } else {
                break;
            }
        }
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() && self.input[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn starts_with(&self, s: &str) -> bool {
        self.input[self.pos..].starts_with(s.as_bytes())
    }

    fn skip_until(&mut self, s: &str) {
        let needle = s.as_bytes();
        while self.pos + needle.len() <= self.input.len() {
            if &self.input[self.pos..self.pos + needle.len()] == needle {
                self.pos += needle.len();
                return;
            }
            self.pos += 1;
        }
        self.pos = self.input.len();
    }

    fn parse_element(&mut self) -> Result<XmlNode, String> {
        self.skip_whitespace();
        if self.pos >= self.input.len() || self.input[self.pos] != b'<' {
            return Err("expected '<'".into());
        }
        self.pos += 1; // skip '<'
        let name = self.parse_name();
        let mut node = XmlNode::new(&name);

        // Parse attributes
        loop {
            self.skip_whitespace();
            if self.pos >= self.input.len() {
                return Err("unexpected end of input in attributes".into());
            }
            match self.input[self.pos] {
                b'>' => {
                    self.pos += 1;
                    break;
                }
                b'/' => {
                    // Self-closing tag: <name ... />
                    if self.pos + 1 < self.input.len() && self.input[self.pos + 1] == b'>' {
                        self.pos += 2;
                        return Ok(node);
                    }
                    return Err("unexpected '/'".into());
                }
                _ => {
                    let attr_name = self.parse_name();
                    self.skip_whitespace();
                    if self.pos < self.input.len() && self.input[self.pos] == b'=' {
                        self.pos += 1;
                        self.skip_whitespace();
                        let value = self.parse_attr_value();
                        node.attributes.insert(attr_name, value);
                    }
                }
            }
        }

        // Parse content (children and text)
        loop {
            let text = self.parse_text();
            if !text.is_empty() {
                node.text.push_str(&text);
            }
            if self.pos >= self.input.len() {
                break;
            }
            if self.starts_with("</") {
                self.pos += 2;
                let _close_name = self.parse_name();
                self.skip_whitespace();
                if self.pos < self.input.len() && self.input[self.pos] == b'>' {
                    self.pos += 1;
                }
                break;
            } else if self.starts_with("<!--") {
                self.skip_until("-->");
            } else if self.starts_with("<![CDATA[") {
                self.pos += 9;
                let start = self.pos;
                self.skip_until("]]>");
                if start < self.pos - 3 {
                    let cdata = String::from_utf8_lossy(&self.input[start..self.pos - 3]);
                    node.text.push_str(&cdata);
                }
            } else if self.input[self.pos] == b'<' {
                let child = self.parse_element()?;
                node.children.push(child);
            } else {
                break;
            }
        }

        Ok(node)
    }

    fn parse_name(&mut self) -> String {
        let start = self.pos;
        while self.pos < self.input.len() {
            let c = self.input[self.pos];
            if c.is_ascii_alphanumeric() || c == b'_' || c == b':' || c == b'-' || c == b'.' {
                self.pos += 1;
            } else {
                break;
            }
        }
        String::from_utf8_lossy(&self.input[start..self.pos]).to_string()
    }

    fn parse_attr_value(&mut self) -> String {
        if self.pos >= self.input.len() {
            return String::new();
        }
        let quote = self.input[self.pos];
        if quote != b'"' && quote != b'\'' {
            return String::new();
        }
        self.pos += 1;
        let start = self.pos;
        while self.pos < self.input.len() && self.input[self.pos] != quote {
            self.pos += 1;
        }
        let value = String::from_utf8_lossy(&self.input[start..self.pos]).to_string();
        if self.pos < self.input.len() {
            self.pos += 1;
        }
        decode_entities(&value)
    }

    fn parse_text(&mut self) -> String {
        let start = self.pos;
        while self.pos < self.input.len() && self.input[self.pos] != b'<' {
            self.pos += 1;
        }
        let text = String::from_utf8_lossy(&self.input[start..self.pos]).to_string();
        decode_entities(text.trim())
    }
}

fn decode_entities(s: &str) -> String {
    let lt = format!("{}lt;", '&');
    let gt = format!("{}gt;", '&');
    let quot = format!("{}quot;", '&');
    let apos = format!("{}apos;", '&');
    let amp = format!("{}amp;", '&');
    s.replace(&lt, "<")
     .replace(&gt, ">")
     .replace(&quot, "\"")
     .replace(&apos, "'")
     .replace(&amp, "&")
}

fn encode_entities(s: &str) -> String {
    let amp = format!("{}amp;", '&');
    let lt = format!("{}lt;", '&');
    let gt = format!("{}gt;", '&');
    let quot = format!("{}quot;", '&');
    let mut result = s.replace('&', &amp);
    result = result.replace('<', &lt);
    result = result.replace('>', &gt);
    result = result.replace('"', &quot);
    result
}

/// Serialize an XmlNode back to XML.
pub fn serialize_xml(node: &XmlNode, indent: usize) -> String {
    let pad = "  ".repeat(indent);
    let mut result = format!("{pad}<{}", node.name);
    for (k, v) in &node.attributes {
        result.push_str(&format!(" {k}=\"{}\"", encode_entities(v)));
    }
    if node.children.is_empty() && node.text.is_empty() {
        result.push_str(" />");
    } else if node.children.is_empty() {
        result.push_str(&format!(">{}</{}>", encode_entities(&node.text), node.name));
    } else {
        result.push_str(">\n");
        for child in &node.children {
            result.push_str(&serialize_xml(child, indent + 1));
            result.push('\n');
        }
        if !node.text.is_empty() {
            result.push_str(&format!("{}  {}\n", pad, encode_entities(&node.text)));
        }
        result.push_str(&format!("{pad}</{}>", node.name));
    }
    result
}
