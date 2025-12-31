//! Example demonstrating nested structures.

use serde::{Deserialize, Serialize};
use serde_xml::{from_str, to_string};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Address {
    street: String,
    city: String,
    state: String,
    zip: String,
    country: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct ContactInfo {
    email: String,
    phone: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Employee {
    id: u32,
    first_name: String,
    last_name: String,
    department: String,
    address: Address,
    contact: ContactInfo,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Company {
    name: String,
    founded: u32,
    headquarters: Address,
    employee: Vec<Employee>,
}

fn main() {
    // Create a company with nested data
    let company = Company {
        name: "TechCorp".to_string(),
        founded: 2010,
        headquarters: Address {
            street: "123 Innovation Way".to_string(),
            city: "San Francisco".to_string(),
            state: "CA".to_string(),
            zip: "94105".to_string(),
            country: "USA".to_string(),
        },
        employee: vec![
            Employee {
                id: 1,
                first_name: "Alice".to_string(),
                last_name: "Smith".to_string(),
                department: "Engineering".to_string(),
                address: Address {
                    street: "456 Elm St".to_string(),
                    city: "Oakland".to_string(),
                    state: "CA".to_string(),
                    zip: "94601".to_string(),
                    country: "USA".to_string(),
                },
                contact: ContactInfo {
                    email: "alice@techcorp.com".to_string(),
                    phone: Some("555-1234".to_string()),
                },
            },
            Employee {
                id: 2,
                first_name: "Bob".to_string(),
                last_name: "Johnson".to_string(),
                department: "Marketing".to_string(),
                address: Address {
                    street: "789 Oak Ave".to_string(),
                    city: "Berkeley".to_string(),
                    state: "CA".to_string(),
                    zip: "94702".to_string(),
                    country: "USA".to_string(),
                },
                contact: ContactInfo {
                    email: "bob@techcorp.com".to_string(),
                    phone: None,
                },
            },
        ],
    };

    // Serialize
    let xml = to_string(&company).expect("Failed to serialize");
    println!("Serialized XML:");
    println!("{}", xml);
    println!();

    // Deserialize
    let parsed: Company = from_str(&xml).expect("Failed to deserialize");
    
    println!("Company: {}", parsed.name);
    println!("Founded: {}", parsed.founded);
    println!("Headquarters: {}, {}", parsed.headquarters.city, parsed.headquarters.state);
    println!();
    
    println!("Employees:");
    for emp in &parsed.employee {
        println!("  {} {} - {} ({}, {})", 
            emp.first_name, 
            emp.last_name, 
            emp.department,
            emp.address.city,
            emp.contact.phone.as_deref().unwrap_or("No phone")
        );
    }

    // Verify roundtrip
    assert_eq!(company, parsed);
    println!("\nRoundtrip verification passed!");
}
