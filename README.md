# serde-xml

A fast, 100% Serde-compatible XML serialization and deserialization library for Rust.

## Features

- **Full Serde compatibility** - Works with `#[derive(Serialize, Deserialize)]`
- **High performance** - Zero-copy parsing, SIMD-accelerated string operations
- **Rich XML support** - Attributes, namespaces, CDATA, comments, processing instructions
- **Comprehensive error reporting** - Line/column positions for all errors
- **Minimal dependencies** - Only `serde`, `memchr`, `itoa`, and `ryu`

## Performance

| Operation | Throughput |
|-----------|------------|
| Serialization (simple) | ~5M structs/sec |
| Deserialization | 180+ MiB/s |
| XML Parsing | 460+ MiB/s |

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

### Nested Structures

```rust
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

### With Attributes

XML attributes are handled using the `@` prefix for field names. Fields without the prefix become child elements:

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
assert_eq!(parsed.class, "sale");
assert_eq!(parsed.name, "Gadget");
```

### Text Content with Attributes

Use `$value` or `$text` to serialize both attributes and text content:

```rust
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

### Attribute Escaping

Special characters in attribute values are automatically escaped:

```rust
#[derive(Serialize)]
struct Element {
    #[serde(rename = "@title")]
    title: String,
}

let elem = Element {
    title: "Hello \"World\" & <Friends>".to_string(),
};
let xml = to_string(&elem).unwrap();
// Output: <Element title="Hello &quot;World&quot; &amp; &lt;Friends&gt;"/>
```

### Optional Fields

```rust
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

## Low-Level API

For more control, you can use the reader and writer directly:

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
