//! Low-level XML writer.
//!
//! This module provides a fast XML writer that produces well-formed XML output.

use crate::escape::escape_to;
use std::io::{self, Write};

/// An XML writer that produces well-formed XML output.
pub struct XmlWriter<W: Write> {
    writer: W,
    /// Stack of open element names.
    element_stack: Vec<String>,
    /// Whether we're currently in an element tag (before the closing >).
    in_tag: bool,
    /// Indentation settings.
    indent: Option<IndentConfig>,
    /// Current indentation level.
    level: usize,
    /// Whether the last write was a start element (for formatting).
    last_was_start: bool,
}

/// Indentation configuration.
#[derive(Clone)]
pub struct IndentConfig {
    /// Characters to use for each level of indentation.
    pub indent_str: String,
    /// Whether to add a newline before each element.
    pub newlines: bool,
}

impl Default for IndentConfig {
    fn default() -> Self {
        Self {
            indent_str: "  ".to_string(),
            newlines: true,
        }
    }
}

impl<W: Write> XmlWriter<W> {
    /// Creates a new XML writer.
    #[inline]
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            element_stack: Vec::new(),
            in_tag: false,
            indent: None,
            level: 0,
            last_was_start: false,
        }
    }

    /// Creates a new XML writer with indentation.
    #[inline]
    pub fn with_indent(writer: W, indent: IndentConfig) -> Self {
        Self {
            writer,
            element_stack: Vec::new(),
            in_tag: false,
            indent: Some(indent),
            level: 0,
            last_was_start: false,
        }
    }

    /// Returns the inner writer.
    #[inline]
    pub fn into_inner(self) -> W {
        self.writer
    }

    /// Returns the current nesting depth.
    #[inline]
    pub fn depth(&self) -> usize {
        self.element_stack.len()
    }

    /// Writes the XML declaration.
    pub fn write_declaration(&mut self, version: &str, encoding: Option<&str>) -> io::Result<()> {
        self.close_tag_if_open()?;
        write!(self.writer, "<?xml version=\"{}\"", version)?;
        if let Some(enc) = encoding {
            write!(self.writer, " encoding=\"{}\"", enc)?;
        }
        self.writer.write_all(b"?>")
    }

    /// Starts an element.
    pub fn start_element(&mut self, name: &str) -> io::Result<()> {
        self.close_tag_if_open()?;
        self.write_indent()?;
        write!(self.writer, "<{}", name)?;
        self.element_stack.push(name.to_string());
        self.in_tag = true;
        self.last_was_start = true;
        self.level += 1;
        Ok(())
    }

    /// Writes an attribute for the current element.
    pub fn write_attribute(&mut self, name: &str, value: &str) -> io::Result<()> {
        if !self.in_tag {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "cannot write attribute outside of element tag",
            ));
        }
        write!(self.writer, " {}=\"", name)?;
        self.write_escaped(value)?;
        self.writer.write_all(b"\"")
    }

    /// Ends the current element.
    pub fn end_element(&mut self) -> io::Result<()> {
        self.level = self.level.saturating_sub(1);

        if let Some(name) = self.element_stack.pop() {
            if self.in_tag {
                // Self-closing tag
                self.writer.write_all(b"/>")?;
                self.in_tag = false;
            } else {
                if !self.last_was_start {
                    self.write_indent()?;
                }
                write!(self.writer, "</{}>", name)?;
            }
            self.last_was_start = false;
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "no element to close",
            ))
        }
    }

    /// Writes text content.
    pub fn write_text(&mut self, text: &str) -> io::Result<()> {
        self.close_tag_if_open()?;
        self.write_escaped(text)?;
        self.last_was_start = false;
        Ok(())
    }

    /// Writes a CDATA section.
    pub fn write_cdata(&mut self, data: &str) -> io::Result<()> {
        self.close_tag_if_open()?;
        write!(self.writer, "<![CDATA[{}]]>", data)
    }

    /// Writes a comment.
    pub fn write_comment(&mut self, comment: &str) -> io::Result<()> {
        self.close_tag_if_open()?;
        self.write_indent()?;
        write!(self.writer, "<!-- {} -->", comment)
    }

    /// Writes a processing instruction.
    pub fn write_pi(&mut self, target: &str, data: Option<&str>) -> io::Result<()> {
        self.close_tag_if_open()?;
        self.write_indent()?;
        write!(self.writer, "<?{}", target)?;
        if let Some(d) = data {
            write!(self.writer, " {}", d)?;
        }
        self.writer.write_all(b"?>")
    }

    /// Writes a complete element with text content.
    pub fn write_element(&mut self, name: &str, content: &str) -> io::Result<()> {
        self.start_element(name)?;
        self.write_text(content)?;
        self.end_element()
    }

    /// Writes an empty element.
    pub fn write_empty_element(&mut self, name: &str) -> io::Result<()> {
        self.close_tag_if_open()?;
        self.write_indent()?;
        write!(self.writer, "<{}/>", name)?;
        self.last_was_start = false;
        Ok(())
    }

    /// Closes the opening tag if one is open.
    fn close_tag_if_open(&mut self) -> io::Result<()> {
        if self.in_tag {
            self.writer.write_all(b">")?;
            self.in_tag = false;
        }
        Ok(())
    }

    /// Writes indentation if configured.
    fn write_indent(&mut self) -> io::Result<()> {
        if let Some(ref indent) = self.indent {
            if indent.newlines && self.level > 0 {
                self.writer.write_all(b"\n")?;
            }
            for _ in 0..self.level.saturating_sub(1) {
                self.writer.write_all(indent.indent_str.as_bytes())?;
            }
        }
        Ok(())
    }

    /// Writes escaped text.
    fn write_escaped(&mut self, s: &str) -> io::Result<()> {
        let mut escaped = String::with_capacity(s.len());
        escape_to(s, &mut escaped);
        self.writer.write_all(escaped.as_bytes())
    }

    /// Flushes the writer.
    pub fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

/// A string-based XML writer for convenience.
pub struct StringXmlWriter {
    writer: XmlWriter<Vec<u8>>,
}

impl StringXmlWriter {
    /// Creates a new string-based XML writer.
    pub fn new() -> Self {
        Self {
            writer: XmlWriter::new(Vec::new()),
        }
    }

    /// Creates a new string-based XML writer with indentation.
    pub fn with_indent(indent: IndentConfig) -> Self {
        Self {
            writer: XmlWriter::with_indent(Vec::new(), indent),
        }
    }

    /// Consumes the writer and returns the XML string.
    pub fn into_string(self) -> String {
        String::from_utf8(self.writer.into_inner()).unwrap_or_default()
    }
}

impl Default for StringXmlWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl std::ops::Deref for StringXmlWriter {
    type Target = XmlWriter<Vec<u8>>;

    fn deref(&self) -> &Self::Target {
        &self.writer
    }
}

impl std::ops::DerefMut for StringXmlWriter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.writer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_to_string<F>(f: F) -> String
    where
        F: FnOnce(&mut XmlWriter<Vec<u8>>) -> io::Result<()>,
    {
        let mut writer = XmlWriter::new(Vec::new());
        f(&mut writer).unwrap();
        String::from_utf8(writer.into_inner()).unwrap()
    }

    #[test]
    fn test_simple_element() {
        let result = write_to_string(|w| {
            w.start_element("root")?;
            w.end_element()
        });
        assert_eq!(result, "<root/>");
    }

    #[test]
    fn test_element_with_text() {
        let result = write_to_string(|w| {
            w.start_element("root")?;
            w.write_text("Hello")?;
            w.end_element()
        });
        assert_eq!(result, "<root>Hello</root>");
    }

    #[test]
    fn test_element_with_attributes() {
        let result = write_to_string(|w| {
            w.start_element("root")?;
            w.write_attribute("id", "1")?;
            w.write_attribute("name", "test")?;
            w.end_element()
        });
        assert_eq!(result, r#"<root id="1" name="test"/>"#);
    }

    #[test]
    fn test_nested_elements() {
        let result = write_to_string(|w| {
            w.start_element("root")?;
            w.start_element("child")?;
            w.write_text("content")?;
            w.end_element()?;
            w.end_element()
        });
        assert_eq!(result, "<root><child>content</child></root>");
    }

    #[test]
    fn test_escaped_content() {
        let result = write_to_string(|w| {
            w.start_element("root")?;
            w.write_text("<>&\"\'")?;
            w.end_element()
        });
        assert_eq!(result, "<root>&lt;&gt;&amp;&quot;&apos;</root>");
    }

    #[test]
    fn test_escaped_attribute() {
        let result = write_to_string(|w| {
            w.start_element("root")?;
            w.write_attribute("attr", "value with \"quotes\"")?;
            w.end_element()
        });
        assert_eq!(result, r#"<root attr="value with &quot;quotes&quot;"/>"#);
    }

    #[test]
    fn test_xml_declaration() {
        let result = write_to_string(|w| {
            w.write_declaration("1.0", Some("UTF-8"))?;
            w.start_element("root")?;
            w.end_element()
        });
        assert_eq!(result, r#"<?xml version="1.0" encoding="UTF-8"?><root/>"#);
    }

    #[test]
    fn test_comment() {
        let result = write_to_string(|w| {
            w.start_element("root")?;
            w.write_comment("This is a comment")?;
            w.end_element()
        });
        assert!(result.contains("<!-- This is a comment -->"));
    }

    #[test]
    fn test_cdata() {
        let result = write_to_string(|w| {
            w.start_element("root")?;
            w.write_cdata("<special>content</special>")?;
            w.end_element()
        });
        assert_eq!(result, "<root><![CDATA[<special>content</special>]]></root>");
    }

    #[test]
    fn test_empty_element() {
        let result = write_to_string(|w| {
            w.write_empty_element("br")
        });
        assert_eq!(result, "<br/>");
    }

    #[test]
    fn test_write_element_shorthand() {
        let result = write_to_string(|w| {
            w.write_element("name", "John")
        });
        assert_eq!(result, "<name>John</name>");
    }

    #[test]
    fn test_depth() {
        let mut writer = XmlWriter::new(Vec::new());
        assert_eq!(writer.depth(), 0);

        writer.start_element("a").unwrap();
        assert_eq!(writer.depth(), 1);

        writer.start_element("b").unwrap();
        assert_eq!(writer.depth(), 2);

        writer.end_element().unwrap();
        assert_eq!(writer.depth(), 1);

        writer.end_element().unwrap();
        assert_eq!(writer.depth(), 0);
    }

    #[test]
    fn test_processing_instruction() {
        let result = write_to_string(|w| {
            w.write_pi("xml-stylesheet", Some("type=\"text/xsl\" href=\"style.xsl\""))
        });
        assert_eq!(result, r#"<?xml-stylesheet type="text/xsl" href="style.xsl"?>"#);
    }

    #[test]
    fn test_indented_output() {
        let mut writer = XmlWriter::with_indent(Vec::new(), IndentConfig::default());
        writer.start_element("root").unwrap();
        writer.start_element("child").unwrap();
        writer.write_text("text").unwrap();
        writer.end_element().unwrap();
        writer.end_element().unwrap();

        let result = String::from_utf8(writer.into_inner()).unwrap();
        assert!(result.contains("\n"));
    }
}
