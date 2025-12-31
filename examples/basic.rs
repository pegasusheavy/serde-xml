//! Basic example demonstrating simple serialization and deserialization.

use serde::{Deserialize, Serialize};
use serde_xml::{from_str, to_string};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Person {
    name: String,
    age: u32,
    email: String,
}

fn main() {
    // Create a person
    let person = Person {
        name: "Alice Smith".to_string(),
        age: 30,
        email: "alice@example.com".to_string(),
    };

    // Serialize to XML
    let xml = to_string(&person).expect("Failed to serialize");
    println!("Serialized XML:");
    println!("{}", xml);
    println!();

    // Deserialize from XML
    let xml_input = r#"
        <Person>
            <name>Bob Johnson</name>
            <age>25</age>
            <email>bob@example.com</email>
        </Person>
    "#;

    let parsed: Person = from_str(xml_input).expect("Failed to deserialize");
    println!("Deserialized person:");
    println!("  Name: {}", parsed.name);
    println!("  Age: {}", parsed.age);
    println!("  Email: {}", parsed.email);
    println!();

    // Roundtrip
    let roundtrip_xml = to_string(&parsed).expect("Failed to serialize");
    let roundtrip: Person = from_str(&roundtrip_xml).expect("Failed to deserialize");
    assert_eq!(parsed, roundtrip);
    println!("Roundtrip successful!");
}
