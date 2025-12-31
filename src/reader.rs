//! Low-level XML reader/tokenizer.
//!
//! This module provides a fast, zero-copy XML tokenizer that produces events
//! for elements, attributes, text content, and other XML constructs.

use crate::error::{Error, Position, Result};
use crate::escape::unescape;
use memchr::memchr;
use std::borrow::Cow;

/// An XML event produced by the reader.
#[derive(Debug, Clone, PartialEq)]
pub enum XmlEvent<'a> {
    /// XML declaration: <?xml version="1.0"?>
    XmlDecl {
        /// XML version (e.g., "1.0").
        version: Cow<'a, str>,
        /// Character encoding (e.g., "UTF-8").
        encoding: Option<Cow<'a, str>>,
        /// Standalone declaration.
        standalone: Option<bool>,
    },
    /// Start of an element: <name attr="value">
    StartElement {
        /// Element name.
        name: Cow<'a, str>,
        /// Element attributes.
        attributes: Vec<Attribute<'a>>,
    },
    /// End of an element: </name>
    EndElement {
        /// Element name.
        name: Cow<'a, str>,
    },
    /// Empty element: <name attr="value"/>
    EmptyElement {
        /// Element name.
        name: Cow<'a, str>,
        /// Element attributes.
        attributes: Vec<Attribute<'a>>,
    },
    /// Text content between elements.
    Text(Cow<'a, str>),
    /// CDATA section: <![CDATA[...]]>
    CData(Cow<'a, str>),
    /// Comment: <!-- ... -->
    Comment(Cow<'a, str>),
    /// Processing instruction: <?target data?>
    ProcessingInstruction {
        /// Processing instruction target.
        target: Cow<'a, str>,
        /// Processing instruction data.
        data: Option<Cow<'a, str>>,
    },
    /// End of document.
    Eof,
}

/// An XML attribute.
#[derive(Debug, Clone, PartialEq)]
pub struct Attribute<'a> {
    /// The attribute name.
    pub name: Cow<'a, str>,
    /// The attribute value.
    pub value: Cow<'a, str>,
}

/// A fast, zero-copy XML reader.
pub struct XmlReader<'a> {
    input: &'a [u8],
    pos: usize,
    line: usize,
    col: usize,
    /// Stack of open element names for validation.
    element_stack: Vec<String>,
}

impl<'a> XmlReader<'a> {
    /// Creates a new XML reader from a string.
    #[inline]
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &'a str) -> Self {
        Self::from_bytes(s.as_bytes())
    }

    /// Creates a new XML reader from bytes.
    #[inline]
    pub fn from_bytes(input: &'a [u8]) -> Self {
        Self {
            input,
            pos: 0,
            line: 1,
            col: 1,
            element_stack: Vec::new(),
        }
    }

    /// Returns the current position in the input.
    #[inline]
    pub fn position(&self) -> Position {
        Position {
            line: self.line,
            column: self.col,
            offset: self.pos,
        }
    }

    /// Returns whether there are any open elements.
    #[inline]
    pub fn depth(&self) -> usize {
        self.element_stack.len()
    }

    /// Reads the next XML event.
    pub fn next_event(&mut self) -> Result<XmlEvent<'a>> {
        self.skip_whitespace();

        if self.pos >= self.input.len() {
            if !self.element_stack.is_empty() {
                let tag = self.element_stack.pop().unwrap();
                return Err(Error::unclosed_tag(tag).with_position(self.position()));
            }
            return Ok(XmlEvent::Eof);
        }

        if self.input[self.pos] == b'<' {
            self.read_tag()
        } else {
            self.read_text()
        }
    }

    /// Skips whitespace characters.
    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() {
            match self.input[self.pos] {
                b' ' | b'\t' | b'\r' => {
                    self.pos += 1;
                    self.col += 1;
                }
                b'\n' => {
                    self.pos += 1;
                    self.line += 1;
                    self.col = 1;
                }
                _ => break,
            }
        }
    }

    /// Reads text content.
    fn read_text(&mut self) -> Result<XmlEvent<'a>> {
        let start = self.pos;

        // Find the end of text (start of next tag or end of input)
        while self.pos < self.input.len() && self.input[self.pos] != b'<' {
            if self.input[self.pos] == b'\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
            self.pos += 1;
        }

        let text = std::str::from_utf8(&self.input[start..self.pos])
            .map_err(|_| Error::new(crate::error::ErrorKind::InvalidUtf8))?;

        // Trim whitespace from text
        let trimmed = text.trim();
        if trimmed.is_empty() {
            // Skip whitespace-only text and read the next event
            return self.next_event();
        }

        // Unescape XML entities
        match unescape(trimmed) {
            Ok(unescaped) => Ok(XmlEvent::Text(unescaped)),
            Err(e) => Err(Error::invalid_escape(e.entity)),
        }
    }

    /// Reads a tag (element, comment, CDATA, PI, or declaration).
    fn read_tag(&mut self) -> Result<XmlEvent<'a>> {
        debug_assert_eq!(self.input[self.pos], b'<');
        self.pos += 1;
        self.col += 1;

        if self.pos >= self.input.len() {
            return Err(Error::unexpected_eof().with_position(self.position()));
        }

        match self.input[self.pos] {
            b'/' => self.read_end_element(),
            b'?' => self.read_processing_instruction(),
            b'!' => self.read_special(),
            _ => self.read_start_element(),
        }
    }

    /// Reads a start element or empty element.
    fn read_start_element(&mut self) -> Result<XmlEvent<'a>> {
        let name = self.read_name()?;
        let attributes = self.read_attributes()?;

        self.skip_whitespace();

        if self.pos >= self.input.len() {
            return Err(Error::unexpected_eof().with_position(self.position()));
        }

        if self.input[self.pos] == b'/' {
            // Empty element: <name/>
            self.pos += 1;
            self.col += 1;
            self.expect_char(b'>')?;
            Ok(XmlEvent::EmptyElement {
                name: Cow::Borrowed(name),
                attributes,
            })
        } else if self.input[self.pos] == b'>' {
            // Start element: <name>
            self.pos += 1;
            self.col += 1;
            self.element_stack.push(name.to_string());
            Ok(XmlEvent::StartElement {
                name: Cow::Borrowed(name),
                attributes,
            })
        } else {
            Err(Error::syntax("expected '>' or '/>'").with_position(self.position()))
        }
    }

    /// Reads an end element.
    fn read_end_element(&mut self) -> Result<XmlEvent<'a>> {
        debug_assert_eq!(self.input[self.pos], b'/');
        self.pos += 1;
        self.col += 1;

        let name = self.read_name()?;
        self.skip_whitespace();
        self.expect_char(b'>')?;

        // Validate matching tags
        match self.element_stack.pop() {
            Some(expected) if expected == name => Ok(XmlEvent::EndElement {
                name: Cow::Borrowed(name),
            }),
            Some(expected) => Err(Error::mismatched_tag(expected, name.to_string()).with_position(self.position())),
            None => Err(Error::syntax(format!("unexpected closing tag: {}", name))
                .with_position(self.position())),
        }
    }

    /// Reads a processing instruction.
    fn read_processing_instruction(&mut self) -> Result<XmlEvent<'a>> {
        debug_assert_eq!(self.input[self.pos], b'?');
        self.pos += 1;
        self.col += 1;

        let target = self.read_name()?;

        // Check for XML declaration
        if target.eq_ignore_ascii_case("xml") {
            return self.read_xml_decl();
        }

        self.skip_whitespace();

        // Read data until ?>
        let data_start = self.pos;
        while self.pos + 1 < self.input.len() {
            if self.input[self.pos] == b'?' && self.input[self.pos + 1] == b'>' {
                let data = std::str::from_utf8(&self.input[data_start..self.pos])
                    .map_err(|_| Error::new(crate::error::ErrorKind::InvalidUtf8))?;
                self.pos += 2;
                self.col += 2;
                return Ok(XmlEvent::ProcessingInstruction {
                    target: Cow::Borrowed(target),
                    data: if data.trim().is_empty() {
                        None
                    } else {
                        Some(Cow::Borrowed(data.trim()))
                    },
                });
            }
            if self.input[self.pos] == b'\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
            self.pos += 1;
        }

        Err(Error::syntax("unterminated processing instruction").with_position(self.position()))
    }

    /// Reads an XML declaration.
    fn read_xml_decl(&mut self) -> Result<XmlEvent<'a>> {
        let attributes = self.read_attributes()?;
        self.skip_whitespace();

        if self.pos + 1 >= self.input.len()
            || self.input[self.pos] != b'?'
            || self.input[self.pos + 1] != b'>'
        {
            return Err(Error::syntax("expected '?>'").with_position(self.position()));
        }
        self.pos += 2;
        self.col += 2;

        let mut version = None;
        let mut encoding = None;
        let mut standalone = None;

        for attr in attributes {
            match attr.name.as_ref() {
                "version" => version = Some(attr.value),
                "encoding" => encoding = Some(attr.value),
                "standalone" => {
                    standalone = Some(attr.value.as_ref() == "yes");
                }
                _ => {}
            }
        }

        Ok(XmlEvent::XmlDecl {
            version: version.unwrap_or(Cow::Borrowed("1.0")),
            encoding,
            standalone,
        })
    }

    /// Reads special constructs (comments, CDATA, DOCTYPE).
    fn read_special(&mut self) -> Result<XmlEvent<'a>> {
        debug_assert_eq!(self.input[self.pos], b'!');
        self.pos += 1;
        self.col += 1;

        if self.pos >= self.input.len() {
            return Err(Error::unexpected_eof().with_position(self.position()));
        }

        // Check for comment: <!--
        if self.pos + 1 < self.input.len()
            && self.input[self.pos] == b'-'
            && self.input[self.pos + 1] == b'-'
        {
            return self.read_comment();
        }

        // Check for CDATA: <![CDATA[
        if self.pos + 6 < self.input.len() && &self.input[self.pos..self.pos + 7] == b"[CDATA[" {
            return self.read_cdata();
        }

        // Check for DOCTYPE
        if self.pos + 6 < self.input.len() && self.input[self.pos..].starts_with(b"DOCTYPE") {
            return self.skip_doctype();
        }

        Err(Error::syntax("unknown construct after '<!'").with_position(self.position()))
    }

    /// Reads a comment.
    fn read_comment(&mut self) -> Result<XmlEvent<'a>> {
        self.pos += 2; // Skip --
        self.col += 2;
        let start = self.pos;

        while self.pos + 2 < self.input.len() {
            if self.input[self.pos] == b'-'
                && self.input[self.pos + 1] == b'-'
                && self.input[self.pos + 2] == b'>'
            {
                let comment = std::str::from_utf8(&self.input[start..self.pos])
                    .map_err(|_| Error::new(crate::error::ErrorKind::InvalidUtf8))?;
                self.pos += 3;
                self.col += 3;
                return Ok(XmlEvent::Comment(Cow::Borrowed(comment.trim())));
            }
            if self.input[self.pos] == b'\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
            self.pos += 1;
        }

        Err(Error::syntax("unterminated comment").with_position(self.position()))
    }

    /// Reads a CDATA section.
    fn read_cdata(&mut self) -> Result<XmlEvent<'a>> {
        self.pos += 7; // Skip [CDATA[
        self.col += 7;
        let start = self.pos;

        while self.pos + 2 < self.input.len() {
            if self.input[self.pos] == b']'
                && self.input[self.pos + 1] == b']'
                && self.input[self.pos + 2] == b'>'
            {
                let data = std::str::from_utf8(&self.input[start..self.pos])
                    .map_err(|_| Error::new(crate::error::ErrorKind::InvalidUtf8))?;
                self.pos += 3;
                self.col += 3;
                return Ok(XmlEvent::CData(Cow::Borrowed(data)));
            }
            if self.input[self.pos] == b'\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
            self.pos += 1;
        }

        Err(Error::syntax("unterminated CDATA section").with_position(self.position()))
    }

    /// Skips a DOCTYPE declaration.
    fn skip_doctype(&mut self) -> Result<XmlEvent<'a>> {
        let mut depth = 1;

        while self.pos < self.input.len() && depth > 0 {
            match self.input[self.pos] {
                b'<' => depth += 1,
                b'>' => depth -= 1,
                b'\n' => {
                    self.line += 1;
                    self.col = 1;
                    self.pos += 1;
                    continue;
                }
                _ => {}
            }
            self.col += 1;
            self.pos += 1;
        }

        // Skip to next event
        self.next_event()
    }

    /// Reads an XML name.
    fn read_name(&mut self) -> Result<&'a str> {
        let start = self.pos;

        // First character must be a name start char
        if self.pos >= self.input.len() {
            return Err(Error::unexpected_eof().with_position(self.position()));
        }

        let first = self.input[self.pos];
        if !is_name_start_char(first) {
            return Err(Error::invalid_name(format!("invalid name start character: {:?}", first as char))
                .with_position(self.position()));
        }
        self.pos += 1;
        self.col += 1;

        // Subsequent characters
        while self.pos < self.input.len() && is_name_char(self.input[self.pos]) {
            self.pos += 1;
            self.col += 1;
        }

        std::str::from_utf8(&self.input[start..self.pos])
            .map_err(|_| Error::new(crate::error::ErrorKind::InvalidUtf8))
    }

    /// Reads element attributes.
    fn read_attributes(&mut self) -> Result<Vec<Attribute<'a>>> {
        let mut attributes = Vec::new();

        loop {
            self.skip_whitespace();

            if self.pos >= self.input.len() {
                break;
            }

            // Check for end of attributes
            let c = self.input[self.pos];
            if c == b'>' || c == b'/' || c == b'?' {
                break;
            }

            // Read attribute name
            let name = self.read_name()?;
            self.skip_whitespace();

            // Expect '='
            self.expect_char(b'=')?;
            self.skip_whitespace();

            // Read attribute value
            let value = self.read_attribute_value()?;

            attributes.push(Attribute {
                name: Cow::Borrowed(name),
                value,
            });
        }

        Ok(attributes)
    }

    /// Reads an attribute value.
    fn read_attribute_value(&mut self) -> Result<Cow<'a, str>> {
        if self.pos >= self.input.len() {
            return Err(Error::unexpected_eof().with_position(self.position()));
        }

        let quote = self.input[self.pos];
        if quote != b'"' && quote != b'\'' {
            return Err(Error::syntax("expected quote").with_position(self.position()));
        }
        self.pos += 1;
        self.col += 1;

        let start = self.pos;

        // Find closing quote
        match memchr(quote, &self.input[self.pos..]) {
            Some(offset) => {
                let value = std::str::from_utf8(&self.input[start..self.pos + offset])
                    .map_err(|_| Error::new(crate::error::ErrorKind::InvalidUtf8))?;
                self.pos += offset + 1;
                self.col += offset + 1;

                // Unescape the value
                match unescape(value) {
                    Ok(unescaped) => Ok(unescaped),
                    Err(e) => Err(Error::invalid_escape(e.entity)),
                }
            }
            None => Err(Error::syntax("unterminated attribute value").with_position(self.position())),
        }
    }

    /// Expects a specific character.
    fn expect_char(&mut self, expected: u8) -> Result<()> {
        if self.pos >= self.input.len() {
            return Err(Error::unexpected_eof().with_position(self.position()));
        }

        if self.input[self.pos] != expected {
            return Err(Error::syntax(format!(
                "expected '{}', found '{}'",
                expected as char,
                self.input[self.pos] as char
            ))
            .with_position(self.position()));
        }

        self.pos += 1;
        self.col += 1;
        Ok(())
    }
}

/// Checks if a byte is a valid XML name start character.
#[inline]
fn is_name_start_char(b: u8) -> bool {
    matches!(b, b'A'..=b'Z' | b'a'..=b'z' | b'_' | b':')
        || b >= 0x80 // Allow UTF-8 continuation bytes (simplified check)
}

/// Checks if a byte is a valid XML name character.
#[inline]
fn is_name_char(b: u8) -> bool {
    is_name_start_char(b) || matches!(b, b'0'..=b'9' | b'-' | b'.')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_element() {
        let mut reader = XmlReader::from_str("<root></root>");

        match reader.next_event().unwrap() {
            XmlEvent::StartElement { name, attributes } => {
                assert_eq!(name, "root");
                assert!(attributes.is_empty());
            }
            _ => panic!("expected StartElement"),
        }

        match reader.next_event().unwrap() {
            XmlEvent::EndElement { name } => {
                assert_eq!(name, "root");
            }
            _ => panic!("expected EndElement"),
        }

        assert!(matches!(reader.next_event().unwrap(), XmlEvent::Eof));
    }

    #[test]
    fn test_empty_element() {
        let mut reader = XmlReader::from_str("<root/>");

        match reader.next_event().unwrap() {
            XmlEvent::EmptyElement { name, attributes } => {
                assert_eq!(name, "root");
                assert!(attributes.is_empty());
            }
            _ => panic!("expected EmptyElement"),
        }

        assert!(matches!(reader.next_event().unwrap(), XmlEvent::Eof));
    }

    #[test]
    fn test_attributes() {
        let mut reader = XmlReader::from_str(r#"<root id="1" name="test"/>"#);

        match reader.next_event().unwrap() {
            XmlEvent::EmptyElement { name, attributes } => {
                assert_eq!(name, "root");
                assert_eq!(attributes.len(), 2);
                assert_eq!(attributes[0].name, "id");
                assert_eq!(attributes[0].value, "1");
                assert_eq!(attributes[1].name, "name");
                assert_eq!(attributes[1].value, "test");
            }
            _ => panic!("expected EmptyElement"),
        }
    }

    #[test]
    fn test_text_content() {
        let mut reader = XmlReader::from_str("<root>Hello, World!</root>");

        reader.next_event().unwrap(); // StartElement

        match reader.next_event().unwrap() {
            XmlEvent::Text(text) => {
                assert_eq!(text, "Hello, World!");
            }
            _ => panic!("expected Text"),
        }
    }

    #[test]
    fn test_escaped_text() {
        let mut reader = XmlReader::from_str("<root>&lt;Hello&gt;</root>");

        reader.next_event().unwrap(); // StartElement

        match reader.next_event().unwrap() {
            XmlEvent::Text(text) => {
                assert_eq!(text, "<Hello>");
            }
            _ => panic!("expected Text"),
        }
    }

    #[test]
    fn test_xml_declaration() {
        let mut reader = XmlReader::from_str(r#"<?xml version="1.0" encoding="UTF-8"?><root/>"#);

        match reader.next_event().unwrap() {
            XmlEvent::XmlDecl { version, encoding, standalone } => {
                assert_eq!(version, "1.0");
                assert_eq!(encoding.as_deref(), Some("UTF-8"));
                assert_eq!(standalone, None);
            }
            _ => panic!("expected XmlDecl"),
        }
    }

    #[test]
    fn test_comment() {
        let mut reader = XmlReader::from_str("<!-- This is a comment --><root/>");

        match reader.next_event().unwrap() {
            XmlEvent::Comment(comment) => {
                assert_eq!(comment, "This is a comment");
            }
            _ => panic!("expected Comment"),
        }
    }

    #[test]
    fn test_cdata() {
        let mut reader = XmlReader::from_str("<root><![CDATA[<special>content</special>]]></root>");

        reader.next_event().unwrap(); // StartElement

        match reader.next_event().unwrap() {
            XmlEvent::CData(data) => {
                assert_eq!(data, "<special>content</special>");
            }
            _ => panic!("expected CData"),
        }
    }

    #[test]
    fn test_nested_elements() {
        let xml = r#"<root><child1><grandchild/></child1><child2/></root>"#;
        let mut reader = XmlReader::from_str(xml);

        let events: Vec<_> = std::iter::from_fn(|| {
            match reader.next_event() {
                Ok(XmlEvent::Eof) => None,
                Ok(event) => Some(event),
                Err(_) => None,
            }
        }).collect();

        assert_eq!(events.len(), 6);
    }

    #[test]
    fn test_mismatched_tags() {
        let mut reader = XmlReader::from_str("<root></wrong>");
        reader.next_event().unwrap(); // StartElement
        assert!(reader.next_event().is_err());
    }

    #[test]
    fn test_unclosed_tag() {
        let mut reader = XmlReader::from_str("<root>");
        reader.next_event().unwrap(); // StartElement
        assert!(reader.next_event().is_err());
    }

    #[test]
    fn test_processing_instruction() {
        let mut reader = XmlReader::from_str("<?target data?><root/>");

        match reader.next_event().unwrap() {
            XmlEvent::ProcessingInstruction { target, data } => {
                assert_eq!(target, "target");
                assert_eq!(data.as_deref(), Some("data"));
            }
            _ => panic!("expected ProcessingInstruction"),
        }
    }

    #[test]
    fn test_attribute_with_single_quotes() {
        let mut reader = XmlReader::from_str("<root attr='value'/>");

        match reader.next_event().unwrap() {
            XmlEvent::EmptyElement { attributes, .. } => {
                assert_eq!(attributes[0].value, "value");
            }
            _ => panic!("expected EmptyElement"),
        }
    }

    #[test]
    fn test_position_tracking() {
        let xml = "<root>\n  <child/>\n</root>";
        let mut reader = XmlReader::from_str(xml);

        reader.next_event().unwrap(); // <root>
        reader.next_event().unwrap(); // <child/>

        let pos = reader.position();
        assert!(pos.line >= 2);
    }

    #[test]
    fn test_depth_tracking() {
        let mut reader = XmlReader::from_str("<a><b><c></c></b></a>");

        assert_eq!(reader.depth(), 0);
        reader.next_event().unwrap(); // <a>
        assert_eq!(reader.depth(), 1);
        reader.next_event().unwrap(); // <b>
        assert_eq!(reader.depth(), 2);
        reader.next_event().unwrap(); // <c>
        assert_eq!(reader.depth(), 3);
        reader.next_event().unwrap(); // </c>
        assert_eq!(reader.depth(), 2);
    }
}
