//! Serde deserializer for XML.
//!
//! This module provides a full-featured Serde deserializer that converts
//! XML documents into Rust data structures.

use crate::error::{Error, Result};
use crate::reader::{XmlEvent, XmlReader};
use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};

/// Deserializes a value from an XML string.
///
/// # Example
///
/// ```
/// use serde::Deserialize;
/// use serde_xml::from_str;
///
/// #[derive(Deserialize)]
/// struct Person {
///     name: String,
///     age: u32,
/// }
///
/// let xml = "<Person><name>Alice</name><age>30</age></Person>";
/// let person: Person = from_str(xml).unwrap();
/// assert_eq!(person.name, "Alice");
/// assert_eq!(person.age, 30);
/// ```
pub fn from_str<'de, T>(s: &'de str) -> Result<T>
where
    T: de::Deserialize<'de>,
{
    let mut de = Deserializer::from_str(s);
    T::deserialize(&mut de)
}

/// Deserializes a value from XML bytes.
pub fn from_bytes<'de, T>(bytes: &'de [u8]) -> Result<T>
where
    T: de::Deserialize<'de>,
{
    let s = std::str::from_utf8(bytes)
        .map_err(|_| Error::new(crate::error::ErrorKind::InvalidUtf8))?;
    from_str(s)
}

/// The XML deserializer.
pub struct Deserializer<'de> {
    reader: XmlReader<'de>,
    /// Peeked event for look-ahead.
    peeked: Option<XmlEvent<'de>>,
    /// Pending value to deserialize (for text content or attribute values).
    pending_value: Option<String>,
    /// Whether we already consumed the start element for the current struct.
    start_consumed: bool,
    /// Whether the current element is empty (<tag/>).
    is_empty_element: bool,
}

impl<'de> Deserializer<'de> {
    /// Creates a new deserializer from a string.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &'de str) -> Self {
        Self {
            reader: XmlReader::from_str(s),
            peeked: None,
            pending_value: None,
            start_consumed: false,
            is_empty_element: false,
        }
    }

    /// Peeks at the next event without consuming it.
    fn peek_event(&mut self) -> Result<&XmlEvent<'de>> {
        if self.peeked.is_none() {
            self.peeked = Some(self.reader.next_event()?);
        }
        Ok(self.peeked.as_ref().unwrap())
    }

    /// Consumes and returns the next event.
    fn next_event(&mut self) -> Result<XmlEvent<'de>> {
        if let Some(event) = self.peeked.take() {
            Ok(event)
        } else {
            self.reader.next_event()
        }
    }

    /// Reads text content until we hit an end tag or another element.
    fn read_text_content(&mut self) -> Result<String> {
        let mut content = String::new();

        loop {
            match self.peek_event()? {
                XmlEvent::Text(text) => {
                    content.push_str(text);
                    self.next_event()?;
                }
                XmlEvent::CData(data) => {
                    content.push_str(data);
                    self.next_event()?;
                }
                _ => break,
            }
        }

        Ok(content)
    }

    /// Reads element text and consumes the end tag.
    fn read_element_text(&mut self) -> Result<String> {
        if self.is_empty_element {
            self.is_empty_element = false;
            self.start_consumed = false;
            return Ok(String::new());
        }

        let content = self.read_text_content()?;

        // Consume end element if we're after a start element
        if self.start_consumed {
            self.start_consumed = false;
            if let XmlEvent::EndElement { .. } = self.peek_event()? {
                self.next_event()?;
            }
        }

        Ok(content)
    }

    /// Skips the current element and all its children.
    fn skip_element(&mut self) -> Result<()> {
        let mut depth = 1;
        while depth > 0 {
            match self.next_event()? {
                XmlEvent::StartElement { .. } => depth += 1,
                XmlEvent::EndElement { .. } => depth -= 1,
                XmlEvent::EmptyElement { .. } => {}
                XmlEvent::Eof => return Err(Error::unexpected_eof()),
                _ => {}
            }
        }
        Ok(())
    }

    /// Parses a value from a string.
    fn parse_value<T>(&self, s: &str) -> Result<T>
    where
        T: std::str::FromStr,
        T::Err: std::fmt::Display,
    {
        s.parse()
            .map_err(|e: T::Err| Error::invalid_value(e.to_string()))
    }

    /// Gets text for primitive deserialization.
    fn get_text(&mut self) -> Result<String> {
        if let Some(value) = self.pending_value.take() {
            return Ok(value);
        }

        // If start was already consumed
        if self.start_consumed {
            return self.read_element_text();
        }

        // Otherwise, we might need to consume a start element first
        match self.peek_event()? {
            XmlEvent::StartElement { .. } => {
                self.next_event()?;
                self.start_consumed = true;
                self.is_empty_element = false;
                self.read_element_text()
            }
            XmlEvent::EmptyElement { .. } => {
                self.next_event()?;
                Ok(String::new())
            }
            _ => self.read_text_content(),
        }
    }
}

impl<'de> de::Deserializer<'de> for &mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if let Some(value) = self.pending_value.take() {
            return visitor.visit_string(value);
        }

        match self.peek_event()? {
            XmlEvent::StartElement { .. } | XmlEvent::EmptyElement { .. } => {
                self.deserialize_map(visitor)
            }
            XmlEvent::Text(text) => {
                let text = text.clone().into_owned();
                self.next_event()?;
                visitor.visit_string(text)
            }
            XmlEvent::CData(data) => {
                let data = data.clone().into_owned();
                self.next_event()?;
                visitor.visit_string(data)
            }
            XmlEvent::EndElement { .. } => visitor.visit_unit(),
            XmlEvent::Eof => visitor.visit_unit(),
            _ => {
                self.next_event()?;
                self.deserialize_any(visitor)
            }
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let text = self.get_text()?;
        match text.as_str() {
            "true" | "1" | "yes" => visitor.visit_bool(true),
            "false" | "0" | "no" => visitor.visit_bool(false),
            _ => Err(Error::invalid_value(format!("expected boolean, got '{}'", text))),
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let text = self.get_text()?;
        visitor.visit_i8(self.parse_value(&text)?)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let text = self.get_text()?;
        visitor.visit_i16(self.parse_value(&text)?)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let text = self.get_text()?;
        visitor.visit_i32(self.parse_value(&text)?)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let text = self.get_text()?;
        visitor.visit_i64(self.parse_value(&text)?)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let text = self.get_text()?;
        visitor.visit_u8(self.parse_value(&text)?)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let text = self.get_text()?;
        visitor.visit_u16(self.parse_value(&text)?)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let text = self.get_text()?;
        visitor.visit_u32(self.parse_value(&text)?)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let text = self.get_text()?;
        visitor.visit_u64(self.parse_value(&text)?)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let text = self.get_text()?;
        visitor.visit_f32(self.parse_value(&text)?)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let text = self.get_text()?;
        visitor.visit_f64(self.parse_value(&text)?)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let text = self.get_text()?;
        let mut chars = text.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) => visitor.visit_char(c),
            _ => Err(Error::invalid_value("expected single character")),
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let text = self.get_text()?;
        visitor.visit_string(text)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let text = self.get_text()?;
        visitor.visit_bytes(text.as_bytes())
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.pending_value.is_some() || self.start_consumed {
            return visitor.visit_some(self);
        }

        match self.peek_event()? {
            XmlEvent::EndElement { .. } | XmlEvent::Eof => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.pending_value = None;
        if self.start_consumed && !self.is_empty_element {
            // Consume end element
            if let XmlEvent::EndElement { .. } = self.peek_event()? {
                self.next_event()?;
            }
        }
        self.start_consumed = false;
        self.is_empty_element = false;
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.start_consumed {
            if !self.is_empty_element {
                // Consume end element
                loop {
                    match self.next_event()? {
                        XmlEvent::EndElement { .. } => break,
                        XmlEvent::Eof => break,
                        _ => {}
                    }
                }
            }
            self.start_consumed = false;
            self.is_empty_element = false;
            return visitor.visit_unit();
        }

        match self.peek_event()? {
            XmlEvent::EmptyElement { .. } => {
                self.next_event()?;
            }
            XmlEvent::StartElement { .. } => {
                self.next_event()?;
                loop {
                    match self.next_event()? {
                        XmlEvent::EndElement { .. } => break,
                        XmlEvent::Eof => break,
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_seq(SeqDeserializer::new(self))
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // Check if start was already consumed (for nested structs)
        let (attrs, is_empty) = if self.start_consumed {
            self.start_consumed = false;
            let is_empty = self.is_empty_element;
            self.is_empty_element = false;
            (vec![], is_empty)
        } else {
            // Get attributes from the start element
            match self.next_event()? {
                XmlEvent::StartElement { attributes, .. } => {
                    let attrs: Vec<_> = attributes
                        .into_iter()
                        .map(|a| (a.name.into_owned(), a.value.into_owned()))
                        .collect();
                    (attrs, false)
                }
                XmlEvent::EmptyElement { attributes, .. } => {
                    let attrs: Vec<_> = attributes
                        .into_iter()
                        .map(|a| (a.name.into_owned(), a.value.into_owned()))
                        .collect();
                    (attrs, true)
                }
                XmlEvent::Eof => (vec![], true),
                _ => (vec![], false),
            }
        };

        let result = visitor.visit_map(MapDeserializer {
            de: self,
            attrs,
            attr_idx: 0,
            finished: is_empty,
        })?;

        // Consume remaining content until end element
        if !is_empty {
            loop {
                match self.peek_event()? {
                    XmlEvent::EndElement { .. } => {
                        self.next_event()?;
                        break;
                    }
                    XmlEvent::Eof => break,
                    _ => {
                        self.next_event()?;
                    }
                }
            }
        }

        Ok(result)
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_enum(EnumDeserializer::new(self))
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.pending_value = None;

        if self.start_consumed {
            if !self.is_empty_element {
                self.skip_element()?;
            }
            self.start_consumed = false;
            self.is_empty_element = false;
            return visitor.visit_unit();
        }

        match self.peek_event()? {
            XmlEvent::StartElement { .. } => {
                self.next_event()?;
                self.skip_element()?;
            }
            XmlEvent::EmptyElement { .. } => {
                self.next_event()?;
            }
            XmlEvent::Text(_) | XmlEvent::CData(_) => {
                self.next_event()?;
            }
            _ => {}
        }
        visitor.visit_unit()
    }
}

/// Sequence deserializer for arrays and vectors.
struct SeqDeserializer<'a, 'de> {
    de: &'a mut Deserializer<'de>,
    element_name: Option<String>,
}

impl<'a, 'de> SeqDeserializer<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        Self {
            de,
            element_name: None,
        }
    }
}

impl<'de, 'a> SeqAccess<'de> for SeqDeserializer<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        loop {
            match self.de.peek_event()? {
                XmlEvent::StartElement { name, .. } | XmlEvent::EmptyElement { name, .. } => {
                    let current_name = name.clone().into_owned();

                    if let Some(ref expected) = self.element_name {
                        if &current_name != expected {
                            return Ok(None);
                        }
                    } else {
                        self.element_name = Some(current_name);
                    }

                    return seed.deserialize(&mut *self.de).map(Some);
                }
                XmlEvent::EndElement { .. } | XmlEvent::Eof => return Ok(None),
                XmlEvent::Text(_) | XmlEvent::CData(_) => {
                    return seed.deserialize(&mut *self.de).map(Some);
                }
                _ => {
                    self.de.next_event()?;
                }
            }
        }
    }
}

/// Map deserializer for structs.
struct MapDeserializer<'a, 'de> {
    de: &'a mut Deserializer<'de>,
    attrs: Vec<(String, String)>,
    attr_idx: usize,
    finished: bool,
}

impl<'de, 'a> MapAccess<'de> for MapDeserializer<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        // First, return any remaining attributes (prefixed with @)
        if self.attr_idx < self.attrs.len() {
            let (name, _) = &self.attrs[self.attr_idx];
            // Prefix with @ to match serde rename convention for attributes
            self.de.pending_value = Some(format!("@{}", name));
            return seed.deserialize(&mut *self.de).map(Some);
        }

        if self.finished {
            return Ok(None);
        }

        // Then check for child elements
        loop {
            match self.de.peek_event()? {
                XmlEvent::StartElement { name, .. } | XmlEvent::EmptyElement { name, .. } => {
                    let name = name.clone().into_owned();
                    // Don't consume the element here - let the value deserializer do it
                    self.de.pending_value = Some(name);
                    return seed.deserialize(&mut *self.de).map(Some);
                }
                XmlEvent::EndElement { .. } | XmlEvent::Eof => {
                    self.finished = true;
                    return Ok(None);
                }
                XmlEvent::Text(_) | XmlEvent::CData(_) => {
                    self.de.pending_value = Some("$value".to_string());
                    return seed.deserialize(&mut *self.de).map(Some);
                }
                _ => {
                    self.de.next_event()?;
                }
            }
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        // Handle attribute values
        if self.attr_idx < self.attrs.len() {
            let (_, value) = &self.attrs[self.attr_idx];
            self.attr_idx += 1;
            self.de.pending_value = Some(value.clone());
            return seed.deserialize(&mut *self.de);
        }

        // Handle element values - element already consumed in next_key_seed
        seed.deserialize(&mut *self.de)
    }
}

/// Enum deserializer.
struct EnumDeserializer<'a, 'de> {
    de: &'a mut Deserializer<'de>,
}

impl<'a, 'de> EnumDeserializer<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        Self { de }
    }
}

impl<'de, 'a> de::EnumAccess<'de> for EnumDeserializer<'a, 'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        // Check for pending value (text-based enum)
        if self.de.pending_value.is_some() {
            let variant = seed.deserialize(&mut *self.de)?;
            return Ok((variant, self));
        }

        // The variant name is the element name
        match self.de.peek_event()? {
            XmlEvent::StartElement { name, .. } | XmlEvent::EmptyElement { name, .. } => {
                let name = name.clone().into_owned();
                self.de.pending_value = Some(name);
            }
            XmlEvent::Text(text) => {
                let text = text.clone().into_owned();
                self.de.pending_value = Some(text);
            }
            _ => {}
        }

        let variant = seed.deserialize(&mut *self.de)?;
        Ok((variant, self))
    }
}

impl<'de, 'a> de::VariantAccess<'de> for EnumDeserializer<'a, 'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        if self.de.start_consumed {
            if !self.de.is_empty_element {
                self.de.skip_element()?;
            }
            self.de.start_consumed = false;
            self.de.is_empty_element = false;
            return Ok(());
        }

        match self.de.peek_event()? {
            XmlEvent::EmptyElement { .. } => {
                self.de.next_event()?;
            }
            XmlEvent::StartElement { .. } => {
                self.de.next_event()?;
                self.de.skip_element()?;
            }
            _ => {}
        }
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.de)
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_seq(&mut *self.de, visitor)
    }

    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_map(&mut *self.de, visitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[test]
    fn test_deserialize_simple_struct() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Person {
            name: String,
            age: u32,
        }

        let xml = "<Person><name>Alice</name><age>30</age></Person>";
        let person: Person = from_str(xml).unwrap();
        assert_eq!(person.name, "Alice");
        assert_eq!(person.age, 30);
    }

    #[test]
    fn test_deserialize_with_attributes() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Item {
            #[serde(rename = "@id")]
            id: String,
            name: String,
        }

        let xml = r#"<Item id="123"><name>Widget</name></Item>"#;
        let item: Item = from_str(xml).unwrap();
        assert_eq!(item.id, "123");
        assert_eq!(item.name, "Widget");
    }

    #[test]
    fn test_deserialize_nested_struct() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Address {
            city: String,
            country: String,
        }

        #[derive(Debug, Deserialize, PartialEq)]
        struct Person {
            name: String,
            address: Address,
        }

        let xml = r#"
            <Person>
                <name>Bob</name>
                <address>
                    <city>New York</city>
                    <country>USA</country>
                </address>
            </Person>
        "#;
        let person: Person = from_str(xml).unwrap();
        assert_eq!(person.name, "Bob");
        assert_eq!(person.address.city, "New York");
    }

    #[test]
    fn test_deserialize_optional() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Config {
            name: String,
            #[serde(default)]
            value: Option<String>,
        }

        let xml1 = "<Config><name>test</name><value>val</value></Config>";
        let config1: Config = from_str(xml1).unwrap();
        assert_eq!(config1.value, Some("val".to_string()));

        let xml2 = "<Config><name>test</name></Config>";
        let config2: Config = from_str(xml2).unwrap();
        assert_eq!(config2.value, None);
    }

    #[test]
    fn test_deserialize_bool() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Flags {
            enabled: bool,
            active: bool,
        }

        let xml = "<Flags><enabled>true</enabled><active>false</active></Flags>";
        let flags: Flags = from_str(xml).unwrap();
        assert!(flags.enabled);
        assert!(!flags.active);
    }

    #[test]
    fn test_deserialize_numbers() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Numbers {
            i: i32,
            u: u64,
            f: f64,
        }

        let xml = "<Numbers><i>-42</i><u>100</u><f>1.234</f></Numbers>";
        let nums: Numbers = from_str(xml).unwrap();
        assert_eq!(nums.i, -42);
        assert_eq!(nums.u, 100);
        assert!((nums.f - 1.234).abs() < 0.001);
    }

    #[test]
    fn test_deserialize_vector() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Items {
            item: Vec<String>,
        }

        let xml = r#"<Items><item>one</item><item>two</item><item>three</item></Items>"#;
        let items: Items = from_str(xml).unwrap();
        assert_eq!(items.item, vec!["one", "two", "three"]);
    }

    #[test]
    fn test_deserialize_escaped_content() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Data {
            content: String,
        }

        let xml = "<Data><content>&lt;hello&gt; &amp; &quot;world&quot;</content></Data>";
        let data: Data = from_str(xml).unwrap();
        assert_eq!(data.content, "<hello> & \"world\"");
    }

    #[test]
    fn test_deserialize_empty_element() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Empty {
            #[serde(default)]
            value: String,
        }

        let xml = "<Empty><value></value></Empty>";
        let empty: Empty = from_str(xml).unwrap();
        assert_eq!(empty.value, "");
    }

    #[test]
    fn test_deserialize_char() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Data {
            c: char,
        }

        let xml = "<Data><c>A</c></Data>";
        let data: Data = from_str(xml).unwrap();
        assert_eq!(data.c, 'A');
    }

    #[test]
    fn test_deserialize_unit_struct() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Unit;

        let xml = "<Unit/>";
        let _unit: Unit = from_str(xml).unwrap();
    }

    #[test]
    fn test_from_bytes() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Data {
            value: String,
        }

        let bytes = b"<Data><value>test</value></Data>";
        let data: Data = from_bytes(bytes).unwrap();
        assert_eq!(data.value, "test");
    }

    #[test]
    fn test_deserialize_vector_of_structs() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Item {
            name: String,
            count: u32,
        }

        #[derive(Debug, Deserialize, PartialEq)]
        struct Items {
            item: Vec<Item>,
        }

        let xml = r#"<Items><item><name>A</name><count>1</count></item><item><name>B</name><count>2</count></item></Items>"#;
        let items: Items = from_str(xml).unwrap();
        assert_eq!(items.item.len(), 2);
        assert_eq!(items.item[0].name, "A");
        assert_eq!(items.item[1].name, "B");
    }

    #[test]
    fn test_deserialize_multiple_attributes() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Element {
            #[serde(rename = "@id")]
            id: String,
            #[serde(rename = "@class")]
            class: String,
            name: String,
        }

        let xml = r#"<Element id="main" class="container"><name>Test</name></Element>"#;
        let elem: Element = from_str(xml).unwrap();
        assert_eq!(elem.id, "main");
        assert_eq!(elem.class, "container");
        assert_eq!(elem.name, "Test");
    }

    #[test]
    fn test_deserialize_attributes_with_special_chars() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Element {
            #[serde(rename = "@title")]
            title: String,
        }

        let xml = r#"<Element title="Hello &amp; &quot;World&quot;"/>"#;
        let elem: Element = from_str(xml).unwrap();
        assert_eq!(elem.title, "Hello & \"World\"");
    }

    #[test]
    fn test_deserialize_numeric_attributes() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Item {
            #[serde(rename = "@id")]
            id: u32,
            #[serde(rename = "@count")]
            count: i32,
            #[serde(rename = "@price")]
            price: f64,
            #[serde(rename = "@active")]
            active: bool,
        }

        let xml = r#"<Item id="42" count="-10" price="19.99" active="true"/>"#;
        let item: Item = from_str(xml).unwrap();
        assert_eq!(item.id, 42);
        assert_eq!(item.count, -10);
        assert!((item.price - 19.99).abs() < 0.001);
        assert!(item.active);
    }

    #[test]
    fn test_deserialize_empty_element_with_attributes() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Empty {
            #[serde(rename = "@id")]
            id: String,
            #[serde(default)]
            value: String,
        }

        let xml = r#"<Empty id="test"/>"#;
        let elem: Empty = from_str(xml).unwrap();
        assert_eq!(elem.id, "test");
        assert_eq!(elem.value, "");
    }

    #[test]
    fn test_deserialize_nested_with_attributes() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Child {
            #[serde(rename = "@name")]
            name: String,
        }

        #[derive(Debug, Deserialize, PartialEq)]
        struct Parent {
            #[serde(rename = "@id")]
            id: String,
            child: Child,
        }

        let xml = r#"<Parent id="p1"><child name="c1"/></Parent>"#;
        let parent: Parent = from_str(xml).unwrap();
        assert_eq!(parent.id, "p1");
        assert_eq!(parent.child.name, "c1");
    }

    #[test]
    fn test_deserialize_vector_with_attributes() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Item {
            #[serde(rename = "@id")]
            id: u32,
            name: String,
        }

        #[derive(Debug, Deserialize, PartialEq)]
        struct List {
            #[serde(rename = "@version")]
            version: String,
            item: Vec<Item>,
        }

        let xml = r#"<List version="1.0"><item id="1"><name>A</name></item><item id="2"><name>B</name></item></List>"#;
        let list: List = from_str(xml).unwrap();
        assert_eq!(list.version, "1.0");
        assert_eq!(list.item.len(), 2);
        assert_eq!(list.item[0].id, 1);
        assert_eq!(list.item[1].id, 2);
    }
}
