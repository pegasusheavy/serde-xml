//! Serde serializer for XML.
//!
//! This module provides a full-featured Serde serializer that converts
//! Rust data structures into XML documents.
//!
//! ## Attribute Serialization
//!
//! Fields can be serialized as XML attributes by using the `@` prefix:
//!
//! ```rust
//! use serde::Serialize;
//! use serde_xml::to_string;
//!
//! #[derive(Serialize)]
//! struct Element {
//!     #[serde(rename = "@id")]
//!     id: String,
//!     #[serde(rename = "@class")]
//!     class: String,
//!     content: String,
//! }
//!
//! let elem = Element {
//!     id: "main".to_string(),
//!     class: "container".to_string(),
//!     content: "Hello".to_string(),
//! };
//!
//! let xml = to_string(&elem).unwrap();
//! // Output: <Element id="main" class="container"><content>Hello</content></Element>
//! ```

use crate::error::{Error, Result};
use crate::escape::escape;
use serde::ser::{self, Serialize};
use std::io::Write;

/// Serializes a value to an XML string.
///
/// # Example
///
/// ```
/// use serde::Serialize;
/// use serde_xml::to_string;
///
/// #[derive(Serialize)]
/// struct Person {
///     name: String,
///     age: u32,
/// }
///
/// let person = Person {
///     name: "Alice".to_string(),
///     age: 30,
/// };
///
/// let xml = to_string(&person).unwrap();
/// assert!(xml.contains("<name>Alice</name>"));
/// ```
pub fn to_string<T>(value: &T) -> Result<String>
where
    T: Serialize + ?Sized,
{
    let mut serializer = Serializer::new();
    value.serialize(&mut serializer)?;
    Ok(serializer.into_string())
}

/// Serializes a value to an XML string with a root element name.
pub fn to_string_with_root<T>(value: &T, root: &str) -> Result<String>
where
    T: Serialize + ?Sized,
{
    let mut serializer = Serializer::with_root(root);
    value.serialize(&mut serializer)?;
    Ok(serializer.into_string())
}

/// Serializes a value to XML bytes.
pub fn to_vec<T>(value: &T) -> Result<Vec<u8>>
where
    T: Serialize + ?Sized,
{
    Ok(to_string(value)?.into_bytes())
}

/// Serializes a value to a writer.
pub fn to_writer<W, T>(writer: W, value: &T) -> Result<()>
where
    W: Write,
    T: Serialize + ?Sized,
{
    let xml = to_string(value)?;
    let mut writer = writer;
    writer.write_all(xml.as_bytes())?;
    Ok(())
}

/// The XML serializer.
pub struct Serializer {
    output: String,
    /// Root element name (for when we don't have type name info).
    root: Option<String>,
    /// Current element name.
    current_element: Option<String>,
    /// Stack of element names for nested structures.
    element_stack: Vec<String>,
    /// Whether we're serializing a map key.
    is_key: bool,
    /// Current key for map entries.
    current_key: Option<String>,
    /// Whether to include XML declaration.
    include_declaration: bool,
    /// Indentation level.
    indent_level: usize,
    /// Indentation string.
    indent_str: Option<String>,
}

impl Serializer {
    /// Creates a new serializer.
    pub fn new() -> Self {
        Self {
            output: String::new(),
            root: None,
            current_element: None,
            element_stack: Vec::new(),
            is_key: false,
            current_key: None,
            include_declaration: false,
            indent_level: 0,
            indent_str: None,
        }
    }

    /// Creates a new serializer with a root element name.
    pub fn with_root(root: &str) -> Self {
        Self {
            root: Some(root.to_string()),
            ..Self::new()
        }
    }

    /// Enables pretty-printing with the given indentation.
    pub fn with_indent(mut self, indent: &str) -> Self {
        self.indent_str = Some(indent.to_string());
        self
    }

    /// Includes XML declaration in the output.
    pub fn with_declaration(mut self) -> Self {
        self.include_declaration = true;
        self
    }

    /// Returns the serialized XML string.
    pub fn into_string(self) -> String {
        self.output
    }

    /// Writes an opening tag.
    fn write_start_tag(&mut self, name: &str) {
        self.write_indent();
        self.output.push('<');
        self.output.push_str(name);
        self.output.push('>');
        self.element_stack.push(name.to_string());
        self.indent_level += 1;
    }

    /// Writes an opening tag with attributes.
    fn write_start_tag_with_attrs(&mut self, name: &str, attrs: &[(String, String)]) {
        self.write_indent();
        self.output.push('<');
        self.output.push_str(name);
        for (attr_name, attr_value) in attrs {
            self.output.push(' ');
            self.output.push_str(attr_name);
            self.output.push_str("=\"");
            self.output.push_str(&escape(attr_value));
            self.output.push('"');
        }
        self.output.push('>');
        self.element_stack.push(name.to_string());
        self.indent_level += 1;
    }

    /// Writes a closing tag.
    fn write_end_tag(&mut self) {
        self.indent_level = self.indent_level.saturating_sub(1);

        if let Some(name) = self.element_stack.pop() {
            self.write_indent();
            self.output.push_str("</");
            self.output.push_str(&name);
            self.output.push('>');
        }
    }

    /// Writes an empty element.
    fn write_empty_element(&mut self, name: &str) {
        self.write_indent();
        self.output.push('<');
        self.output.push_str(name);
        self.output.push_str("/>");
    }

    /// Writes an empty element with attributes.
    fn write_empty_element_with_attrs(&mut self, name: &str, attrs: &[(String, String)]) {
        self.write_indent();
        self.output.push('<');
        self.output.push_str(name);
        for (attr_name, attr_value) in attrs {
            self.output.push(' ');
            self.output.push_str(attr_name);
            self.output.push_str("=\"");
            self.output.push_str(&escape(attr_value));
            self.output.push('"');
        }
        self.output.push_str("/>");
    }

    /// Writes a complete element with text content.
    fn write_element(&mut self, name: &str, content: &str) {
        self.write_indent();
        self.output.push('<');
        self.output.push_str(name);
        self.output.push('>');
        self.output.push_str(&escape(content));
        self.output.push_str("</");
        self.output.push_str(name);
        self.output.push('>');
    }

    /// Writes text content.
    fn write_text(&mut self, content: &str) {
        self.output.push_str(&escape(content));
    }

    /// Writes indentation if configured.
    fn write_indent(&mut self) {
        if let Some(ref indent) = self.indent_str {
            if !self.output.is_empty() && !self.output.ends_with('\n') {
                self.output.push('\n');
            }
            for _ in 0..self.indent_level.saturating_sub(1) {
                self.output.push_str(indent);
            }
        }
    }

    /// Gets the current element name.
    fn get_element_name(&self, fallback: &str) -> String {
        self.current_key
            .clone()
            .or_else(|| self.current_element.clone())
            .or_else(|| self.root.clone())
            .unwrap_or_else(|| fallback.to_string())
    }
}

impl Default for Serializer {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> ser::Serializer for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = SeqSerializer<'a>;
    type SerializeTuple = SeqSerializer<'a>;
    type SerializeTupleStruct = SeqSerializer<'a>;
    type SerializeTupleVariant = SeqSerializer<'a>;
    type SerializeMap = MapSerializer<'a>;
    type SerializeStruct = StructSerializer<'a>;
    type SerializeStructVariant = StructSerializer<'a>;

    fn serialize_bool(self, v: bool) -> Result<()> {
        let text = if v { "true" } else { "false" };
        if let Some(ref key) = self.current_key.take() {
            self.write_element(key, text);
        } else {
            self.write_text(text);
        }
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<()> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i32(self, v: i32) -> Result<()> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i64(self, v: i64) -> Result<()> {
        let mut buffer = itoa::Buffer::new();
        let text = buffer.format(v);
        if let Some(ref key) = self.current_key.take() {
            self.write_element(key, text);
        } else {
            self.write_text(text);
        }
        Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        self.serialize_u64(v as u64)
    }

    fn serialize_u16(self, v: u16) -> Result<()> {
        self.serialize_u64(v as u64)
    }

    fn serialize_u32(self, v: u32) -> Result<()> {
        self.serialize_u64(v as u64)
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        let mut buffer = itoa::Buffer::new();
        let text = buffer.format(v);
        if let Some(ref key) = self.current_key.take() {
            self.write_element(key, text);
        } else {
            self.write_text(text);
        }
        Ok(())
    }

    fn serialize_f32(self, v: f32) -> Result<()> {
        self.serialize_f64(v as f64)
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        let mut buffer = ryu::Buffer::new();
        let text = buffer.format(v);
        if let Some(ref key) = self.current_key.take() {
            self.write_element(key, text);
        } else {
            self.write_text(text);
        }
        Ok(())
    }

    fn serialize_char(self, v: char) -> Result<()> {
        let mut buf = [0u8; 4];
        let text = v.encode_utf8(&mut buf);
        if let Some(ref key) = self.current_key.take() {
            self.write_element(key, text);
        } else {
            self.write_text(text);
        }
        Ok(())
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        if self.is_key {
            self.current_key = Some(v.to_string());
            self.is_key = false;
        } else if let Some(ref key) = self.current_key.take() {
            self.write_element(key, v);
        } else {
            self.write_text(v);
        }
        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        // Hex encode bytes
        use std::fmt::Write;
        let mut encoded = String::new();
        for byte in v {
            write!(&mut encoded, "{:02x}", byte).unwrap();
        }
        if let Some(ref key) = self.current_key.take() {
            self.write_element(key, &encoded);
        } else {
            self.write_text(&encoded);
        }
        Ok(())
    }

    fn serialize_none(self) -> Result<()> {
        // Don't output anything for None
        self.current_key = None;
        Ok(())
    }

    fn serialize_some<T>(self, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<()> {
        if let Some(ref key) = self.current_key.take() {
            self.write_empty_element(key);
        }
        Ok(())
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<()> {
        let elem_name = self.get_element_name(name);
        self.write_empty_element(&elem_name);
        Ok(())
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<()> {
        if let Some(ref key) = self.current_key.take() {
            self.write_element(key, variant);
        } else {
            self.write_empty_element(variant);
        }
        Ok(())
    }

    fn serialize_newtype_struct<T>(self, name: &'static str, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        self.current_element = Some(name.to_string());
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        self.write_start_tag(variant);
        value.serialize(&mut *self)?;
        self.write_end_tag();
        Ok(())
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        let element_name = self.current_key.take().unwrap_or_else(|| "item".to_string());
        Ok(SeqSerializer {
            ser: self,
            element_name,
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        self.write_start_tag(name);
        Ok(SeqSerializer {
            ser: self,
            element_name: "item".to_string(),
        })
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        self.write_start_tag(variant);
        Ok(SeqSerializer {
            ser: self,
            element_name: "item".to_string(),
        })
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        let name = self.current_key.take()
            .or_else(|| self.root.clone())
            .unwrap_or_else(|| "map".to_string());
        self.write_start_tag(&name);
        Ok(MapSerializer { ser: self })
    }

    fn serialize_struct(self, name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        let elem_name = self.current_key.take().unwrap_or_else(|| name.to_string());
        // Don't write start tag yet - collect attributes first
        Ok(StructSerializer {
            ser: self,
            elem_name,
            attrs: Vec::new(),
            children: Vec::new(),
            text_content: None,
            started: false,
        })
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Ok(StructSerializer {
            ser: self,
            elem_name: variant.to_string(),
            attrs: Vec::new(),
            children: Vec::new(),
            text_content: None,
            started: false,
        })
    }
}

/// Simple serializer for attribute values (no XML escaping - escaping done at output).
struct AttrValueSerializer {
    output: String,
}

impl AttrValueSerializer {
    fn new() -> Self {
        Self { output: String::new() }
    }

    fn into_string(self) -> String {
        self.output
    }
}

impl ser::Serializer for &mut AttrValueSerializer {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = ser::Impossible<(), Error>;
    type SerializeTuple = ser::Impossible<(), Error>;
    type SerializeTupleStruct = ser::Impossible<(), Error>;
    type SerializeTupleVariant = ser::Impossible<(), Error>;
    type SerializeMap = ser::Impossible<(), Error>;
    type SerializeStruct = ser::Impossible<(), Error>;
    type SerializeStructVariant = ser::Impossible<(), Error>;

    fn serialize_bool(self, v: bool) -> Result<()> {
        self.output.push_str(if v { "true" } else { "false" });
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<()> { self.serialize_i64(v as i64) }
    fn serialize_i16(self, v: i16) -> Result<()> { self.serialize_i64(v as i64) }
    fn serialize_i32(self, v: i32) -> Result<()> { self.serialize_i64(v as i64) }
    fn serialize_i64(self, v: i64) -> Result<()> {
        let mut buffer = itoa::Buffer::new();
        self.output.push_str(buffer.format(v));
        Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<()> { self.serialize_u64(v as u64) }
    fn serialize_u16(self, v: u16) -> Result<()> { self.serialize_u64(v as u64) }
    fn serialize_u32(self, v: u32) -> Result<()> { self.serialize_u64(v as u64) }
    fn serialize_u64(self, v: u64) -> Result<()> {
        let mut buffer = itoa::Buffer::new();
        self.output.push_str(buffer.format(v));
        Ok(())
    }

    fn serialize_f32(self, v: f32) -> Result<()> { self.serialize_f64(v as f64) }
    fn serialize_f64(self, v: f64) -> Result<()> {
        let mut buffer = ryu::Buffer::new();
        self.output.push_str(buffer.format(v));
        Ok(())
    }

    fn serialize_char(self, v: char) -> Result<()> {
        self.output.push(v);
        Ok(())
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        // No escaping here - escaping happens when writing the attribute
        self.output.push_str(v);
        Ok(())
    }

    fn serialize_bytes(self, _v: &[u8]) -> Result<()> {
        Err(Error::unsupported("bytes in attribute"))
    }

    fn serialize_none(self) -> Result<()> { Ok(()) }
    fn serialize_some<T: ?Sized + Serialize>(self, v: &T) -> Result<()> { v.serialize(self) }
    fn serialize_unit(self) -> Result<()> { Ok(()) }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> { Ok(()) }
    fn serialize_unit_variant(self, _name: &'static str, _idx: u32, variant: &'static str) -> Result<()> {
        self.output.push_str(variant);
        Ok(())
    }
    fn serialize_newtype_struct<T: ?Sized + Serialize>(self, _name: &'static str, v: &T) -> Result<()> {
        v.serialize(self)
    }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(self, _name: &'static str, _idx: u32, _variant: &'static str, v: &T) -> Result<()> {
        v.serialize(self)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Err(Error::unsupported("sequence in attribute"))
    }
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Err(Error::unsupported("tuple in attribute"))
    }
    fn serialize_tuple_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeTupleStruct> {
        Err(Error::unsupported("tuple struct in attribute"))
    }
    fn serialize_tuple_variant(self, _name: &'static str, _idx: u32, _variant: &'static str, _len: usize) -> Result<Self::SerializeTupleVariant> {
        Err(Error::unsupported("tuple variant in attribute"))
    }
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Err(Error::unsupported("map in attribute"))
    }
    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        Err(Error::unsupported("struct in attribute"))
    }
    fn serialize_struct_variant(self, _name: &'static str, _idx: u32, _variant: &'static str, _len: usize) -> Result<Self::SerializeStructVariant> {
        Err(Error::unsupported("struct variant in attribute"))
    }
}

/// Sequence serializer.
pub struct SeqSerializer<'a> {
    ser: &'a mut Serializer,
    element_name: String,
}

impl<'a> ser::SerializeSeq for SeqSerializer<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        self.ser.current_key = Some(self.element_name.clone());
        value.serialize(&mut *self.ser)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a> ser::SerializeTuple for SeqSerializer<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<()> {
        ser::SerializeSeq::end(self)
    }
}

impl<'a> ser::SerializeTupleStruct for SeqSerializer<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        self.ser.current_key = Some(self.element_name.clone());
        value.serialize(&mut *self.ser)
    }

    fn end(self) -> Result<()> {
        self.ser.write_end_tag();
        Ok(())
    }
}

impl<'a> ser::SerializeTupleVariant for SeqSerializer<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        self.ser.current_key = Some(self.element_name.clone());
        value.serialize(&mut *self.ser)
    }

    fn end(self) -> Result<()> {
        self.ser.write_end_tag();
        Ok(())
    }
}

/// Map serializer.
pub struct MapSerializer<'a> {
    ser: &'a mut Serializer,
}

impl<'a> ser::SerializeMap for MapSerializer<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        self.ser.is_key = true;
        key.serialize(&mut *self.ser)
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(&mut *self.ser)
    }

    fn end(self) -> Result<()> {
        self.ser.write_end_tag();
        Ok(())
    }
}

/// Struct serializer with attribute support.
pub struct StructSerializer<'a> {
    ser: &'a mut Serializer,
    elem_name: String,
    attrs: Vec<(String, String)>,
    children: Vec<String>,
    text_content: Option<String>,
    started: bool,
}

impl<'a> StructSerializer<'a> {
    fn ensure_started(&mut self) {
        if !self.started {
            self.ser.write_start_tag_with_attrs(&self.elem_name, &self.attrs);
            // Write any buffered children
            for child in &self.children {
                self.ser.output.push_str(child);
            }
            self.children.clear();
            self.started = true;
        }
    }
}

impl<'a> ser::SerializeStruct for StructSerializer<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        // Check if this is an attribute (starts with @)
        if let Some(attr_name) = key.strip_prefix('@') {
            // Serialize value to string - use a special mode that doesn't escape
            let mut attr_ser = AttrValueSerializer::new();
            value.serialize(&mut attr_ser)?;
            let attr_value = attr_ser.into_string();
            self.attrs.push((attr_name.to_string(), attr_value));
            return Ok(());
        }

        // Check if this is text content ($value or $text)
        if key == "$value" || key == "$text" {
            // Serialize value to string
            let mut text_ser = Serializer::new();
            value.serialize(&mut text_ser)?;
            self.text_content = Some(text_ser.into_string());
            return Ok(());
        }

        // Regular field - ensure element started
        self.ensure_started();
        self.ser.current_key = Some(key.to_string());
        value.serialize(&mut *self.ser)
    }

    fn end(self) -> Result<()> {
        if self.started {
            // Write text content if any
            if let Some(text) = self.text_content {
                self.ser.output.push_str(&text);
            }
            self.ser.write_end_tag();
        } else if self.attrs.is_empty() && self.text_content.is_none() {
            // Empty element with no attributes
            self.ser.write_empty_element(&self.elem_name);
        } else if let Some(text) = self.text_content {
            // Element with just text content and possibly attributes
            self.ser.write_start_tag_with_attrs(&self.elem_name, &self.attrs);
            self.ser.output.push_str(&text);
            self.ser.write_end_tag();
        } else {
            // Element with only attributes
            self.ser.write_empty_element_with_attrs(&self.elem_name, &self.attrs);
        }
        Ok(())
    }
}

impl<'a> ser::SerializeStructVariant for StructSerializer<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        ser::SerializeStruct::serialize_field(self, key, value)
    }

    fn end(self) -> Result<()> {
        ser::SerializeStruct::end(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[test]
    fn test_serialize_simple_struct() {
        #[derive(Serialize)]
        struct Person {
            name: String,
            age: u32,
        }

        let person = Person {
            name: "Alice".to_string(),
            age: 30,
        };

        let xml = to_string(&person).unwrap();
        assert!(xml.contains("<Person>"));
        assert!(xml.contains("<name>Alice</name>"));
        assert!(xml.contains("<age>30</age>"));
        assert!(xml.contains("</Person>"));
    }

    #[test]
    fn test_serialize_with_attributes() {
        #[derive(Serialize)]
        struct Element {
            #[serde(rename = "@id")]
            id: String,
            #[serde(rename = "@class")]
            class: String,
            content: String,
        }

        let elem = Element {
            id: "main".to_string(),
            class: "container".to_string(),
            content: "Hello".to_string(),
        };

        let xml = to_string(&elem).unwrap();
        assert!(xml.contains(r#"id="main""#));
        assert!(xml.contains(r#"class="container""#));
        assert!(xml.contains("<content>Hello</content>"));
    }

    #[test]
    fn test_serialize_attributes_only() {
        #[derive(Serialize)]
        struct EmptyElement {
            #[serde(rename = "@id")]
            id: String,
            #[serde(rename = "@disabled")]
            disabled: bool,
        }

        let elem = EmptyElement {
            id: "btn".to_string(),
            disabled: true,
        };

        let xml = to_string(&elem).unwrap();
        assert!(xml.contains(r#"id="btn""#));
        assert!(xml.contains(r#"disabled="true""#));
        assert!(xml.contains("/>") || xml.contains("</EmptyElement>"));
    }

    #[test]
    fn test_serialize_text_content() {
        #[derive(Serialize)]
        struct TextElement {
            #[serde(rename = "@id")]
            id: String,
            #[serde(rename = "$value")]
            text: String,
        }

        let elem = TextElement {
            id: "para".to_string(),
            text: "Hello World".to_string(),
        };

        let xml = to_string(&elem).unwrap();
        assert!(xml.contains(r#"id="para""#));
        assert!(xml.contains("Hello World"));
    }

    #[test]
    fn test_serialize_nested_struct() {
        #[derive(Serialize)]
        struct Address {
            city: String,
            country: String,
        }

        #[derive(Serialize)]
        struct Person {
            name: String,
            address: Address,
        }

        let person = Person {
            name: "Bob".to_string(),
            address: Address {
                city: "New York".to_string(),
                country: "USA".to_string(),
            },
        };

        let xml = to_string(&person).unwrap();
        assert!(xml.contains("<address>"));
        assert!(xml.contains("<city>New York</city>"));
        assert!(xml.contains("</address>"));
    }

    #[test]
    fn test_serialize_optional() {
        #[derive(Serialize)]
        struct Config {
            name: String,
            value: Option<String>,
        }

        let with_value = Config {
            name: "test".to_string(),
            value: Some("val".to_string()),
        };
        let xml = to_string(&with_value).unwrap();
        assert!(xml.contains("<value>val</value>"));

        let without_value = Config {
            name: "test".to_string(),
            value: None,
        };
        let xml = to_string(&without_value).unwrap();
        assert!(!xml.contains("<value>"));
    }

    #[test]
    fn test_serialize_vector() {
        #[derive(Serialize)]
        struct Items {
            items: Vec<String>,
        }

        let items = Items {
            items: vec!["one".to_string(), "two".to_string(), "three".to_string()],
        };

        let xml = to_string(&items).unwrap();
        assert!(xml.contains("<items>one</items>"));
        assert!(xml.contains("<items>two</items>"));
        assert!(xml.contains("<items>three</items>"));
    }

    #[test]
    fn test_serialize_escaped_content() {
        #[derive(Serialize)]
        struct Data {
            content: String,
        }

        let data = Data {
            content: "<hello> & \"world\"".to_string(),
        };

        let xml = to_string(&data).unwrap();
        assert!(xml.contains("&lt;hello&gt;"));
        assert!(xml.contains("&amp;"));
        assert!(xml.contains("&quot;"));
    }

    #[test]
    fn test_serialize_escaped_attribute() {
        #[derive(Serialize)]
        struct Element {
            #[serde(rename = "@title")]
            title: String,
        }

        let elem = Element {
            title: "Hello \"World\" & <Friends>".to_string(),
        };

        let xml = to_string(&elem).unwrap();
        assert!(xml.contains("&quot;"));
        assert!(xml.contains("&amp;"));
        assert!(xml.contains("&lt;"));
    }

    #[test]
    fn test_serialize_bool() {
        #[derive(Serialize)]
        struct Flags {
            enabled: bool,
            active: bool,
        }

        let flags = Flags {
            enabled: true,
            active: false,
        };

        let xml = to_string(&flags).unwrap();
        assert!(xml.contains("<enabled>true</enabled>"));
        assert!(xml.contains("<active>false</active>"));
    }

    #[test]
    fn test_serialize_numbers() {
        #[derive(Serialize)]
        struct Numbers {
            i: i32,
            u: u64,
            f: f64,
        }

        let nums = Numbers {
            i: -42,
            u: 100,
            f: 1.234,
        };

        let xml = to_string(&nums).unwrap();
        assert!(xml.contains("<i>-42</i>"));
        assert!(xml.contains("<u>100</u>"));
        assert!(xml.contains("<f>1.234</f>"));
    }

    #[test]
    fn test_serialize_enum() {
        #[derive(Serialize)]
        enum Status {
            Active,
            #[allow(dead_code)]
            Inactive,
            #[allow(dead_code)]
            Pending,
        }

        #[derive(Serialize)]
        struct Item {
            status: Status,
        }

        let item = Item {
            status: Status::Active,
        };

        let xml = to_string(&item).unwrap();
        assert!(xml.contains("<status>Active</status>") || xml.contains("<Active/>"));
    }

    #[test]
    fn test_serialize_unit_struct() {
        #[derive(Serialize)]
        struct Empty;

        let xml = to_string(&Empty).unwrap();
        assert!(xml.contains("<Empty/>"));
    }

    #[test]
    fn test_serialize_char() {
        #[derive(Serialize)]
        struct Data {
            c: char,
        }

        let data = Data { c: 'A' };
        let xml = to_string(&data).unwrap();
        assert!(xml.contains("<c>A</c>"));
    }

    #[test]
    fn test_to_vec() {
        #[derive(Serialize)]
        struct Data {
            value: String,
        }

        let data = Data {
            value: "test".to_string(),
        };

        let bytes = to_vec(&data).unwrap();
        let xml = String::from_utf8(bytes).unwrap();
        assert!(xml.contains("<value>test</value>"));
    }

    #[test]
    fn test_to_writer() {
        #[derive(Serialize)]
        struct Data {
            value: String,
        }

        let data = Data {
            value: "test".to_string(),
        };

        let mut buffer = Vec::new();
        to_writer(&mut buffer, &data).unwrap();
        let xml = String::from_utf8(buffer).unwrap();
        assert!(xml.contains("<value>test</value>"));
    }

    #[test]
    fn test_with_root() {
        #[derive(Serialize)]
        struct Data {
            value: String,
        }

        let data = Data {
            value: "test".to_string(),
        };

        let xml = to_string_with_root(&data, "root").unwrap();
        // The struct name takes precedence, but root is used for maps
        assert!(xml.contains("<value>test</value>"));
    }

    #[test]
    fn test_complex_with_attributes() {
        #[derive(Serialize)]
        struct Item {
            #[serde(rename = "@id")]
            id: u32,
            #[serde(rename = "@class")]
            class: String,
            name: String,
            value: i32,
        }

        #[derive(Serialize)]
        struct Container {
            #[serde(rename = "@version")]
            version: String,
            item: Vec<Item>,
        }

        let container = Container {
            version: "1.0".to_string(),
            item: vec![
                Item {
                    id: 1,
                    class: "primary".to_string(),
                    name: "First".to_string(),
                    value: 100,
                },
                Item {
                    id: 2,
                    class: "secondary".to_string(),
                    name: "Second".to_string(),
                    value: 200,
                },
            ],
        };

        let xml = to_string(&container).unwrap();
        assert!(xml.contains(r#"version="1.0""#));
        assert!(xml.contains(r#"id="1""#));
        assert!(xml.contains(r#"class="primary""#));
        assert!(xml.contains("<name>First</name>"));
    }
}
