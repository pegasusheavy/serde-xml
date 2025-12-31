//! Example demonstrating XML attributes handling.
//!
//! This example shows how to use the `@` prefix convention to serialize
//! and deserialize XML attributes.

use serde::{Deserialize, Serialize};
use serde_xml::{from_str, to_string};

/// Product with attributes and child elements.
/// Fields prefixed with `@` become XML attributes.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Product {
    #[serde(rename = "@id")]
    id: String,
    #[serde(rename = "@sku")]
    sku: String,
    name: String,
    price: f64,
}

/// Catalog with version attribute and product children.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Catalog {
    #[serde(rename = "@version")]
    version: String,
    product: Vec<Product>,
}

/// Element with text content and attributes.
/// Use `$value` to capture text content alongside attributes.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Link {
    #[serde(rename = "@href")]
    href: String,
    #[serde(rename = "@target")]
    target: String,
    #[serde(rename = "$value")]
    text: String,
}

fn main() {
    // ================================
    // DESERIALIZATION
    // ================================
    println!("=== Deserializing XML with Attributes ===\n");

    let xml = r#"
        <Catalog version="1.0">
            <product id="1" sku="WIDGET-001">
                <name>Super Widget</name>
                <price>29.99</price>
            </product>
            <product id="2" sku="GADGET-002">
                <name>Mega Gadget</name>
                <price>49.99</price>
            </product>
        </Catalog>
    "#;

    let catalog: Catalog = from_str(xml).expect("Failed to deserialize");

    println!("Catalog version: {}", catalog.version);
    for product in &catalog.product {
        println!("  Product {} ({}): {} @ ${:.2}",
            product.id, product.sku, product.name, product.price);
    }

    // ================================
    // SERIALIZATION
    // ================================
    println!("\n=== Serializing to XML with Attributes ===\n");

    let new_catalog = Catalog {
        version: "2.0".to_string(),
        product: vec![
            Product {
                id: "10".to_string(),
                sku: "NEW-001".to_string(),
                name: "New Product".to_string(),
                price: 99.99,
            },
            Product {
                id: "11".to_string(),
                sku: "NEW-002".to_string(),
                name: "Another Product".to_string(),
                price: 149.99,
            },
        ],
    };

    let xml = to_string(&new_catalog).unwrap();
    println!("Serialized XML:");
    println!("{}", xml);

    // ================================
    // TEXT CONTENT WITH ATTRIBUTES
    // ================================
    println!("\n=== Text Content with Attributes ===\n");

    let link = Link {
        href: "https://example.com".to_string(),
        target: "_blank".to_string(),
        text: "Click here".to_string(),
    };

    let xml = to_string(&link).unwrap();
    println!("Link serialized: {}", xml);

    // ================================
    // SPECIAL CHARACTER ESCAPING
    // ================================
    println!("\n=== Attribute Value Escaping ===\n");

    #[derive(Serialize)]
    struct Element {
        #[serde(rename = "@title")]
        title: String,
        #[serde(rename = "@data")]
        data: String,
    }

    let elem = Element {
        title: "Hello \"World\" & <Friends>".to_string(),
        data: "a=1&b=2".to_string(),
    };

    let xml = to_string(&elem).unwrap();
    println!("Element with special characters:");
    println!("{}", xml);

    // ================================
    // ROUNDTRIP
    // ================================
    println!("\n=== Roundtrip Test ===\n");

    let original = Product {
        id: "42".to_string(),
        sku: "TEST-001".to_string(),
        name: "Test Product".to_string(),
        price: 19.99,
    };

    let xml = to_string(&original).unwrap();
    println!("Serialized: {}", xml);

    let deserialized: Product = from_str(&xml).unwrap();
    println!("Deserialized: {:?}", deserialized);

    assert_eq!(original, deserialized);
    println!("Roundtrip successful!");
}
