//! # serde_xml
//!
//! A fast, 100% Serde-compatible XML serialization and deserialization library.
//!
//! ## Features
//!
//! - Full Serde compatibility for serialization and deserialization
//! - Zero-copy parsing where possible
//! - Fast XML tokenization using SIMD-accelerated string searching
//! - Support for attributes, namespaces, CDATA, comments, and processing instructions
//! - Comprehensive error reporting with line/column positions
//! - No unsafe code in the public API
//!
//! ## Quick Start
//!
//! ```rust
//! use serde::{Deserialize, Serialize};
//! use serde_xml::{from_str, to_string};
//!
//! #[derive(Debug, Serialize, Deserialize, PartialEq)]
//! struct Person {
//!     name: String,
//!     age: u32,
//! }
//!
//! // Serialize to XML
//! let person = Person {
//!     name: "Alice".to_string(),
//!     age: 30,
//! };
//! let xml = to_string(&person).unwrap();
//!
//! // Deserialize from XML
//! let xml = "<Person><name>Alice</name><age>30</age></Person>";
//! let person: Person = from_str(xml).unwrap();
//! assert_eq!(person.name, "Alice");
//! assert_eq!(person.age, 30);
//! ```
//!
//! ## Nested Structures
//!
//! ```rust
//! use serde::{Deserialize, Serialize};
//! use serde_xml::from_str;
//!
//! #[derive(Debug, Deserialize)]
//! struct Address {
//!     city: String,
//!     country: String,
//! }
//!
//! #[derive(Debug, Deserialize)]
//! struct Person {
//!     name: String,
//!     address: Address,
//! }
//!
//! let xml = r#"
//!     <Person>
//!         <name>Bob</name>
//!         <address>
//!             <city>New York</city>
//!             <country>USA</country>
//!         </address>
//!     </Person>
//! "#;
//!
//! let person: Person = from_str(xml).unwrap();
//! assert_eq!(person.address.city, "New York");
//! ```
//!
//! ## Collections
//!
//! ```rust
//! use serde::{Deserialize, Serialize};
//! use serde_xml::{from_str, to_string};
//!
//! #[derive(Debug, Serialize, Deserialize)]
//! struct Library {
//!     books: Vec<String>,
//! }
//!
//! let library = Library {
//!     books: vec![
//!         "The Rust Programming Language".to_string(),
//!         "Programming Rust".to_string(),
//!     ],
//! };
//!
//! let xml = to_string(&library).unwrap();
//! ```
//!
//! ## Optional Fields
//!
//! ```rust
//! use serde::{Deserialize, Serialize};
//! use serde_xml::from_str;
//!
//! #[derive(Debug, Deserialize)]
//! struct Config {
//!     name: String,
//!     description: Option<String>,
//! }
//!
//! let xml = "<Config><name>test</name></Config>";
//! let config: Config = from_str(xml).unwrap();
//! assert_eq!(config.description, None);
//! ```

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]
#![deny(unsafe_op_in_unsafe_fn)]

pub mod de;
pub mod error;
pub mod escape;
pub mod reader;
pub mod ser;
pub mod writer;

// Re-export main types and functions
pub use de::{from_bytes, from_str, Deserializer};
pub use error::{Error, ErrorKind, Position, Result};
pub use escape::{escape, unescape};
pub use reader::{Attribute, XmlEvent, XmlReader};
pub use ser::{to_string, to_string_with_root, to_vec, to_writer, Serializer};
pub use writer::{IndentConfig, XmlWriter};

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[test]
    fn test_roundtrip_simple() {
        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        struct Person {
            name: String,
            age: u32,
        }

        let original = Person {
            name: "Alice".to_string(),
            age: 30,
        };

        let xml = to_string(&original).unwrap();
        let parsed: Person = from_str(&xml).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn test_roundtrip_nested() {
        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        struct Address {
            city: String,
            country: String,
        }

        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        struct Person {
            name: String,
            address: Address,
        }

        let original = Person {
            name: "Bob".to_string(),
            address: Address {
                city: "New York".to_string(),
                country: "USA".to_string(),
            },
        };

        let xml = to_string(&original).unwrap();
        let parsed: Person = from_str(&xml).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn test_roundtrip_vector() {
        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        struct Items {
            item: Vec<String>,
        }

        let original = Items {
            item: vec!["one".to_string(), "two".to_string(), "three".to_string()],
        };

        let xml = to_string(&original).unwrap();
        let parsed: Items = from_str(&xml).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn test_roundtrip_optional() {
        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        struct Config {
            name: String,
            value: Option<String>,
        }

        let with_value = Config {
            name: "test".to_string(),
            value: Some("val".to_string()),
        };

        let xml = to_string(&with_value).unwrap();
        let parsed: Config = from_str(&xml).unwrap();
        assert_eq!(with_value, parsed);
    }

    #[test]
    fn test_roundtrip_escaped() {
        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        struct Data {
            content: String,
        }

        let original = Data {
            content: "<hello> & \"world\"".to_string(),
        };

        let xml = to_string(&original).unwrap();
        let parsed: Data = from_str(&xml).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn test_xml_reader_basic() {
        let mut reader = XmlReader::from_str("<root><child>text</child></root>");

        match reader.next_event().unwrap() {
            XmlEvent::StartElement { name, .. } => assert_eq!(name, "root"),
            _ => panic!("expected StartElement"),
        }

        match reader.next_event().unwrap() {
            XmlEvent::StartElement { name, .. } => assert_eq!(name, "child"),
            _ => panic!("expected StartElement"),
        }

        match reader.next_event().unwrap() {
            XmlEvent::Text(text) => assert_eq!(text, "text"),
            _ => panic!("expected Text"),
        }
    }

    #[test]
    fn test_xml_writer_basic() {

        let mut buffer = Vec::new();
        {
            let mut writer = XmlWriter::new(&mut buffer);
            writer.start_element("root").unwrap();
            writer.start_element("child").unwrap();
            writer.write_text("text").unwrap();
            writer.end_element().unwrap();
            writer.end_element().unwrap();
        }

        let xml = String::from_utf8(buffer).unwrap();
        assert!(xml.contains("<root>"));
        assert!(xml.contains("<child>text</child>"));
    }

    #[test]
    fn test_escape_unescape() {
        let original = "<hello> & \"world\"";
        let escaped = escape(original);
        let unescaped = unescape(&escaped).unwrap();
        assert_eq!(unescaped, original);
    }

    #[test]
    fn test_error_reporting() {
        // Test mismatched tags error
        #[derive(Debug, Deserialize)]
        struct Item {
            name: String,
        }

        let result: Result<Item> = from_str("<Item><name>test</wrong></Item>");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("mismatched") || err.to_string().contains("wrong"));
    }

    #[test]
    fn test_complex_xml() {
        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        struct Book {
            title: String,
            author: String,
            year: u32,
        }

        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        struct Library {
            name: String,
            book: Vec<Book>,
        }

        let original = Library {
            name: "My Library".to_string(),
            book: vec![
                Book {
                    title: "The Rust Programming Language".to_string(),
                    author: "Steve Klabnik".to_string(),
                    year: 2018,
                },
                Book {
                    title: "Programming Rust".to_string(),
                    author: "Jim Blandy".to_string(),
                    year: 2021,
                },
            ],
        };

        let xml = to_string(&original).unwrap();
        let parsed: Library = from_str(&xml).unwrap();
        assert_eq!(original, parsed);
    }
}
