//! Example demonstrating HTML-like parsing with serde-xml.
//!
//! This library can parse well-formed HTML/XHTML documents.
//! Note: For malformed HTML, consider using a dedicated HTML parser.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use serde_xml::{from_str, to_string};

// ============================================================================
// Basic HTML Elements
// ============================================================================

/// A simple anchor/link element.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Anchor {
    #[serde(rename = "@href")]
    href: String,
    #[serde(rename = "@target", default)]
    target: Option<String>,
    #[serde(rename = "@class", default)]
    class: Option<String>,
    #[serde(rename = "$value")]
    text: String,
}

/// An image element (self-closing).
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Img {
    #[serde(rename = "@src")]
    src: String,
    #[serde(rename = "@alt", default)]
    alt: Option<String>,
    #[serde(rename = "@width", default)]
    width: Option<u32>,
    #[serde(rename = "@height", default)]
    height: Option<u32>,
}

/// A div element with common attributes.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Div {
    #[serde(rename = "@id", default)]
    id: Option<String>,
    #[serde(rename = "@class", default)]
    class: Option<String>,
    #[serde(rename = "@style", default)]
    style: Option<String>,
    #[serde(rename = "$value", default)]
    content: Option<String>,
}

/// A span element.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Span {
    #[serde(rename = "@class", default)]
    class: Option<String>,
    #[serde(rename = "$value")]
    text: String,
}

// ============================================================================
// Form Elements
// ============================================================================

/// An input element with various types.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Input {
    #[serde(rename = "@type")]
    input_type: String,
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "@id", default)]
    id: Option<String>,
    #[serde(rename = "@value", default)]
    value: Option<String>,
    #[serde(rename = "@placeholder", default)]
    placeholder: Option<String>,
    #[serde(rename = "@required", default)]
    required: Option<String>,
    #[serde(rename = "@class", default)]
    class: Option<String>,
}

/// A label element.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Label {
    #[serde(rename = "@for")]
    for_id: String,
    #[serde(rename = "$value")]
    text: String,
}

/// A button element.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Button {
    #[serde(rename = "@type", default)]
    button_type: Option<String>,
    #[serde(rename = "@class", default)]
    class: Option<String>,
    #[serde(rename = "@disabled", default)]
    disabled: Option<String>,
    #[serde(rename = "$value")]
    text: String,
}

/// A select/dropdown element.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Select {
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "@id", default)]
    id: Option<String>,
    option: Vec<SelectOption>,
}

/// An option within a select.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct SelectOption {
    #[serde(rename = "@value")]
    value: String,
    #[serde(rename = "@selected", default)]
    selected: Option<String>,
    #[serde(rename = "$value")]
    text: String,
}

/// A complete form.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Form {
    #[serde(rename = "@action")]
    action: String,
    #[serde(rename = "@method", default)]
    method: Option<String>,
    #[serde(rename = "@class", default)]
    class: Option<String>,
    #[serde(default)]
    input: Vec<Input>,
    #[serde(default)]
    button: Vec<Button>,
}

// ============================================================================
// Table Elements
// ============================================================================

/// A table cell.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Td {
    #[serde(rename = "@colspan", default)]
    colspan: Option<u32>,
    #[serde(rename = "@class", default)]
    class: Option<String>,
    #[serde(rename = "$value")]
    content: String,
}

/// A table header cell.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Th {
    #[serde(rename = "@scope", default)]
    scope: Option<String>,
    #[serde(rename = "$value")]
    content: String,
}

/// A table row.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Tr {
    #[serde(rename = "@class", default)]
    class: Option<String>,
    #[serde(default)]
    th: Vec<Th>,
    #[serde(default)]
    td: Vec<Td>,
}

/// A complete table.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Table {
    #[serde(rename = "@class", default)]
    class: Option<String>,
    #[serde(rename = "@id", default)]
    id: Option<String>,
    tr: Vec<Tr>,
}

// ============================================================================
// Meta Elements
// ============================================================================

/// A meta tag.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Meta {
    #[serde(rename = "@name", default)]
    name: Option<String>,
    #[serde(rename = "@content", default)]
    content: Option<String>,
    #[serde(rename = "@charset", default)]
    charset: Option<String>,
    #[serde(rename = "@property", default)]
    property: Option<String>,
}

/// A link tag (for stylesheets, etc.).
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Link {
    #[serde(rename = "@rel")]
    rel: String,
    #[serde(rename = "@href")]
    href: String,
    #[serde(rename = "@type", default)]
    link_type: Option<String>,
}

/// A script tag.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Script {
    #[serde(rename = "@src", default)]
    src: Option<String>,
    #[serde(rename = "@type", default)]
    script_type: Option<String>,
    #[serde(rename = "@defer", default)]
    defer: Option<String>,
    #[serde(rename = "@async", default)]
    async_attr: Option<String>,
}

// ============================================================================
// Complex HTML Document Structure
// ============================================================================

/// A navigation menu.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Nav {
    #[serde(rename = "@class", default)]
    class: Option<String>,
    #[serde(default)]
    a: Vec<Anchor>,
}

/// A card component (common in modern HTML).
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Card {
    #[serde(rename = "@class", default)]
    class: Option<String>,
    #[serde(default)]
    img: Option<Img>,
    title: String,
    description: String,
    #[serde(default)]
    link: Option<Anchor>,
}

/// A list item.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Li {
    #[serde(rename = "@class", default)]
    class: Option<String>,
    #[serde(rename = "$value")]
    content: String,
}

/// An unordered list.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Ul {
    #[serde(rename = "@class", default)]
    class: Option<String>,
    li: Vec<Li>,
}

fn main() {
    println!("=== HTML Parsing Examples ===\n");

    // ========================================================================
    // Example 1: Parsing Links
    // ========================================================================
    println!("--- Parsing Anchor Tags ---\n");

    let html = r#"<a href="https://example.com" target="_blank" class="btn btn-primary">Click Here</a>"#;
    let anchor: Anchor = from_str(html).unwrap();
    println!("Parsed anchor:");
    println!("  href: {}", anchor.href);
    println!("  target: {:?}", anchor.target);
    println!("  class: {:?}", anchor.class);
    println!("  text: {}", anchor.text);

    // Roundtrip
    let serialized = to_string(&anchor).unwrap();
    println!("  Serialized: {}\n", serialized);

    // ========================================================================
    // Example 2: Parsing Forms
    // ========================================================================
    println!("--- Parsing Form Elements ---\n");

    let form_html = r#"
        <Form action="/login" method="POST" class="login-form">
            <input type="text" name="username" id="username" placeholder="Enter username" required="required"/>
            <input type="password" name="password" id="password" placeholder="Enter password" required="required"/>
            <input type="checkbox" name="remember" id="remember" value="1"/>
            <button type="submit" class="btn btn-success">Login</button>
        </Form>
    "#;

    let form: Form = from_str(form_html).unwrap();
    println!("Parsed form:");
    println!("  action: {}", form.action);
    println!("  method: {:?}", form.method);
    println!("  Inputs:");
    for input in &form.input {
        println!("    - {} (type: {}, placeholder: {:?})",
            input.name, input.input_type, input.placeholder);
    }
    println!("  Buttons:");
    for button in &form.button {
        println!("    - {} (type: {:?})", button.text, button.button_type);
    }
    println!();

    // ========================================================================
    // Example 3: Parsing Tables
    // ========================================================================
    println!("--- Parsing Table Elements ---\n");

    let table_html = r#"
        <Table class="data-table" id="users">
            <tr class="header">
                <th scope="col">Name</th>
                <th scope="col">Email</th>
                <th scope="col">Role</th>
            </tr>
            <tr>
                <td>Alice</td>
                <td>alice@example.com</td>
                <td>Admin</td>
            </tr>
            <tr>
                <td>Bob</td>
                <td>bob@example.com</td>
                <td>User</td>
            </tr>
        </Table>
    "#;

    let table: Table = from_str(table_html).unwrap();
    println!("Parsed table (id: {:?}, class: {:?}):", table.id, table.class);
    for (i, row) in table.tr.iter().enumerate() {
        if !row.th.is_empty() {
            let headers: Vec<_> = row.th.iter().map(|h| h.content.as_str()).collect();
            println!("  Header: {:?}", headers);
        } else {
            let cells: Vec<_> = row.td.iter().map(|c| c.content.as_str()).collect();
            println!("  Row {}: {:?}", i, cells);
        }
    }
    println!();

    // ========================================================================
    // Example 4: Parsing Navigation
    // ========================================================================
    println!("--- Parsing Navigation ---\n");

    let nav_html = r#"
        <Nav class="main-nav">
            <a href="/">Home</a>
            <a href="/about">About</a>
            <a href="/products">Products</a>
            <a href="/contact">Contact</a>
        </Nav>
    "#;

    let nav: Nav = from_str(nav_html).unwrap();
    println!("Navigation menu ({:?}):", nav.class);
    for link in &nav.a {
        println!("  - {} -> {}", link.text, link.href);
    }
    println!();

    // ========================================================================
    // Example 5: Parsing Select/Dropdown
    // ========================================================================
    println!("--- Parsing Select Elements ---\n");

    let select_html = r#"
        <Select name="country" id="country-select">
            <option value="">Select a country</option>
            <option value="us" selected="selected">United States</option>
            <option value="uk">United Kingdom</option>
            <option value="ca">Canada</option>
            <option value="au">Australia</option>
        </Select>
    "#;

    let select: Select = from_str(select_html).unwrap();
    println!("Select dropdown (name: {}):", select.name);
    for opt in &select.option {
        let selected = if opt.selected.is_some() { " [SELECTED]" } else { "" };
        println!("  - {} = {}{}", opt.value, opt.text, selected);
    }
    println!();

    // ========================================================================
    // Example 6: Parsing Card Components
    // ========================================================================
    println!("--- Parsing Card Components ---\n");

    let card_html = r#"
        <Card class="product-card">
            <img src="/images/product.jpg" alt="Product Image" width="300" height="200"/>
            <title>Amazing Product</title>
            <description>This is an amazing product that you need!</description>
            <link href="/products/1" class="btn">View Details</link>
        </Card>
    "#;

    let card: Card = from_str(card_html).unwrap();
    println!("Parsed card:");
    println!("  Title: {}", card.title);
    println!("  Description: {}", card.description);
    if let Some(img) = &card.img {
        println!("  Image: {} ({}x{:?})", img.src, img.width.unwrap_or(0), img.height);
    }
    if let Some(link) = &card.link {
        println!("  Link: {} -> {}", link.text, link.href);
    }
    println!();

    // ========================================================================
    // Example 7: Parsing Lists
    // ========================================================================
    println!("--- Parsing Lists ---\n");

    let list_html = r#"
        <Ul class="features">
            <li>Fast and efficient</li>
            <li>Easy to use</li>
            <li class="highlight">100% Serde compatible</li>
            <li>Well documented</li>
        </Ul>
    "#;

    let list: Ul = from_str(list_html).unwrap();
    println!("Feature list:");
    for item in &list.li {
        let marker = if item.class.is_some() { "★" } else { "•" };
        println!("  {} {}", marker, item.content);
    }
    println!();

    // ========================================================================
    // Example 8: Generating HTML
    // ========================================================================
    println!("--- Generating HTML ---\n");

    let new_form = Form {
        action: "/register".to_string(),
        method: Some("POST".to_string()),
        class: Some("registration-form".to_string()),
        input: vec![
            Input {
                input_type: "email".to_string(),
                name: "email".to_string(),
                id: Some("email".to_string()),
                value: None,
                placeholder: Some("your@email.com".to_string()),
                required: Some("required".to_string()),
                class: Some("form-control".to_string()),
            },
            Input {
                input_type: "password".to_string(),
                name: "password".to_string(),
                id: Some("password".to_string()),
                value: None,
                placeholder: Some("Choose a password".to_string()),
                required: Some("required".to_string()),
                class: Some("form-control".to_string()),
            },
        ],
        button: vec![
            Button {
                button_type: Some("submit".to_string()),
                class: Some("btn btn-primary".to_string()),
                disabled: None,
                text: "Register".to_string(),
            },
        ],
    };

    let generated_html = to_string(&new_form).unwrap();
    println!("Generated form HTML:");
    println!("{}\n", generated_html);

    // ========================================================================
    // Example 9: Meta Tags
    // ========================================================================
    println!("--- Parsing Meta Tags ---\n");

    let meta_html = r#"<Meta name="description" content="A fast XML parser for Rust"/>"#;
    let meta: Meta = from_str(meta_html).unwrap();
    println!("Meta tag: {}={:?}", meta.name.unwrap_or_default(), meta.content);

    let charset_html = r#"<Meta charset="UTF-8"/>"#;
    let charset: Meta = from_str(charset_html).unwrap();
    println!("Charset: {:?}", charset.charset);

    println!("\n=== All HTML parsing examples completed! ===");
}
