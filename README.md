# serde-xml-fast

[![Crates.io](https://img.shields.io/crates/v/serde-xml-fast.svg)](https://crates.io/crates/serde-xml-fast)
[![Documentation](https://docs.rs/serde-xml-fast/badge.svg)](https://docs.rs/serde-xml-fast)
[![License](https://img.shields.io/crates/l/serde-xml-fast.svg)](https://github.com/pegasusheavy/serde-xml#license)

A fast, 100% Serde-compatible XML serialization and deserialization library for Rust.

## Features

- **Full Serde compatibility** - Works seamlessly with `#[derive(Serialize, Deserialize)]`
- **High performance** - Zero-copy parsing, SIMD-accelerated string operations via `memchr`
- **XML Attributes** - First-class support using the `@` prefix convention
- **Rich XML support** - CDATA, comments, processing instructions, and more
- **Comprehensive error reporting** - Line/column positions for all errors
- **Minimal dependencies** - Only `serde`, `memchr`, `itoa`, and `ryu`

## Performance

| Operation | Throughput |
|-----------|------------|
| Serialization (simple) | ~5.8M structs/sec |
| Serialization (complex) | ~256K structs/sec |
| Deserialization | 190-200 MiB/s |
| XML Parsing | 580+ MiB/s |
| Roundtrip (simple) | ~1.85M ops/sec |

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
serde-xml-fast = "0.1"
serde = { version = "1.0", features = ["derive"] }
```

## Quick Start

```rust
use serde::{Deserialize, Serialize};
use serde_xml::{from_str, to_string};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Person {
    name: String,
    age: u32,
}

fn main() {
    // Serialize to XML
    let person = Person {
        name: "Alice".to_string(),
        age: 30,
    };
    let xml = to_string(&person).unwrap();
    println!("{}", xml);
    // Output: <Person><name>Alice</name><age>30</age></Person>

    // Deserialize from XML
    let xml = "<Person><name>Bob</name><age>25</age></Person>";
    let person: Person = from_str(xml).unwrap();
    assert_eq!(person.name, "Bob");
}
```

## Examples

Run any example with:

```bash
cargo run --example <name>
```

Available examples:
- `basic` - Simple serialization and deserialization
- `nested` - Nested struct handling
- `collections` - Vectors and collections
- `attributes` - XML attribute support with `@` prefix
- `html_parsing` - Parsing HTML-like structures

### Nested Structures

```rust
use serde::{Deserialize, Serialize};
use serde_xml::from_str;

#[derive(Debug, Serialize, Deserialize)]
struct Address {
    city: String,
    country: String,
}

#[derive(Debug, Serialize, Deserialize)]
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
assert_eq!(person.address.city, "New York");
```

### Collections

```rust
use serde::{Deserialize, Serialize};
use serde_xml::to_string;

#[derive(Debug, Serialize, Deserialize)]
struct Library {
    book: Vec<String>,
}

let library = Library {
    book: vec!["Book 1".to_string(), "Book 2".to_string()],
};

let xml = to_string(&library).unwrap();
// <Library><book>Book 1</book><book>Book 2</book></Library>
```

### XML Attributes

Use the `@` prefix to serialize/deserialize XML attributes:

```rust
use serde::{Deserialize, Serialize};
use serde_xml::{from_str, to_string};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Item {
    #[serde(rename = "@id")]
    id: String,          // Serializes as attribute: id="..."
    #[serde(rename = "@class")]
    class: String,       // Serializes as attribute: class="..."
    name: String,        // Serializes as child element: <name>...</name>
}

// Serialize with attributes
let item = Item {
    id: "123".to_string(),
    class: "product".to_string(),
    name: "Widget".to_string(),
};
let xml = to_string(&item).unwrap();
// Output: <Item id="123" class="product"><name>Widget</name></Item>

// Deserialize with attributes
let xml = r#"<Item id="456" class="sale"><name>Gadget</name></Item>"#;
let parsed: Item = from_str(xml).unwrap();
assert_eq!(parsed.id, "456");
```

### Text Content with Attributes

Use `$value` or `$text` to combine attributes with text content:

```rust
use serde::{Deserialize, Serialize};
use serde_xml::to_string;

#[derive(Serialize, Deserialize)]
struct Link {
    #[serde(rename = "@href")]
    href: String,
    #[serde(rename = "$value")]
    text: String,
}

let link = Link {
    href: "https://example.com".to_string(),
    text: "Click here".to_string(),
};
let xml = to_string(&link).unwrap();
// Output: <Link href="https://example.com">Click here</Link>
```

### HTML-like Parsing

The library can parse well-formed HTML/XHTML structures:

```rust
use serde::Deserialize;
use serde_xml::from_str;

#[derive(Deserialize)]
struct Form {
    #[serde(rename = "@action")]
    action: String,
    #[serde(rename = "@method")]
    method: Option<String>,
    #[serde(default)]
    input: Vec<Input>,
}

#[derive(Deserialize)]
struct Input {
    #[serde(rename = "@type")]
    input_type: String,
    #[serde(rename = "@name")]
    name: String,
}

let html = r#"
    <Form action="/login" method="POST">
        <input type="text" name="username"/>
        <input type="password" name="password"/>
    </Form>
"#;

let form: Form = from_str(html).unwrap();
assert_eq!(form.action, "/login");
assert_eq!(form.input.len(), 2);
```

### Optional Fields

```rust
use serde::{Deserialize, Serialize};
use serde_xml::from_str;

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    name: String,
    #[serde(default)]
    value: Option<String>,
}

let xml = "<Config><name>test</name></Config>";
let config: Config = from_str(xml).unwrap();
assert_eq!(config.value, None);
```

### Automatic Escaping

Special characters are automatically escaped in both content and attributes:

```rust
use serde::Serialize;
use serde_xml::to_string;

#[derive(Serialize)]
struct Element {
    #[serde(rename = "@title")]
    title: String,
    content: String,
}

let elem = Element {
    title: "Hello \"World\" & <Friends>".to_string(),
    content: "<script>alert('xss')</script>".to_string(),
};
let xml = to_string(&elem).unwrap();
// Attributes: title="Hello &quot;World&quot; &amp; &lt;Friends&gt;"
// Content: <content>&lt;script&gt;alert('xss')&lt;/script&gt;</content>
```

## Low-Level API

For more control, use the reader and writer directly:

```rust
use serde_xml::{XmlReader, XmlWriter, XmlEvent};

// Reading
let mut reader = XmlReader::from_str("<root>Hello</root>");
while let Ok(event) = reader.next_event() {
    match event {
        XmlEvent::StartElement { name, .. } => println!("Start: {}", name),
        XmlEvent::Text(text) => println!("Text: {}", text),
        XmlEvent::EndElement { name } => println!("End: {}", name),
        XmlEvent::Eof => break,
        _ => {}
    }
}

// Writing
let mut writer = XmlWriter::new(Vec::new());
writer.start_element("root").unwrap();
writer.write_text("Hello").unwrap();
writer.end_element().unwrap();
```

## Running Benchmarks

```bash
cargo bench
```

## Running Tests

```bash
cargo test
```

## License

Copyright 2025 Pegasus Heavy Industries LLC

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
