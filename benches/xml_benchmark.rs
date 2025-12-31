//! Benchmarks for serde_xml performance.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use serde::{Deserialize, Serialize};
use serde_xml::{from_str, to_string};

// Simple struct for basic benchmarks
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
struct Simple {
    name: String,
    value: i64,
    active: bool,
}

// Medium complexity struct
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
struct Medium {
    id: u64,
    title: String,
    description: String,
    count: i32,
    ratio: f64,
    enabled: bool,
    tags: Vec<String>,
}

// Complex nested struct
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
struct Address {
    street: String,
    city: String,
    state: String,
    zip: String,
    country: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
struct Person {
    id: u64,
    first_name: String,
    last_name: String,
    email: String,
    age: u8,
    address: Address,
    phone_numbers: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
struct Company {
    name: String,
    founded: u32,
    employee_count: u32,
    headquarters: Address,
    employee: Vec<Person>,
}

// Test data generators
fn simple_data() -> Simple {
    Simple {
        name: "Test Item".to_string(),
        value: 42,
        active: true,
    }
}

fn medium_data() -> Medium {
    Medium {
        id: 12345,
        title: "Sample Document".to_string(),
        description: "This is a sample document with some description text.".to_string(),
        count: 100,
        ratio: 1.23456,
        enabled: true,
        tags: vec![
            "tag1".to_string(),
            "tag2".to_string(),
            "tag3".to_string(),
            "important".to_string(),
        ],
    }
}

fn complex_data() -> Company {
    Company {
        name: "TechCorp International".to_string(),
        founded: 2010,
        employee_count: 5000,
        headquarters: Address {
            street: "123 Innovation Boulevard".to_string(),
            city: "San Francisco".to_string(),
            state: "California".to_string(),
            zip: "94105".to_string(),
            country: "United States".to_string(),
        },
        employee: (0..10)
            .map(|i| Person {
                id: i as u64,
                first_name: format!("First{}", i),
                last_name: format!("Last{}", i),
                email: format!("person{}@techcorp.com", i),
                age: 25 + (i as u8 % 40),
                address: Address {
                    street: format!("{} Oak Street", 100 + i),
                    city: "Oakland".to_string(),
                    state: "California".to_string(),
                    zip: format!("9460{}", i % 10),
                    country: "United States".to_string(),
                },
                phone_numbers: vec![
                    format!("555-000{}", i),
                    format!("555-100{}", i),
                ],
            })
            .collect(),
    }
}

fn simple_xml() -> &'static str {
    "<Simple><name>Test Item</name><value>42</value><active>true</active></Simple>"
}

fn medium_xml() -> String {
    to_string(&medium_data()).unwrap()
}

fn complex_xml() -> String {
    to_string(&complex_data()).unwrap()
}

fn bench_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("Serialization");
    
    let simple = simple_data();
    let medium = medium_data();
    let complex = complex_data();

    group.bench_function("simple", |b| {
        b.iter(|| to_string(black_box(&simple)))
    });

    group.bench_function("medium", |b| {
        b.iter(|| to_string(black_box(&medium)))
    });

    group.bench_function("complex", |b| {
        b.iter(|| to_string(black_box(&complex)))
    });

    group.finish();
}

fn bench_deserialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("Deserialization");
    
    let simple_xml = simple_xml();
    let medium_xml = medium_xml();
    let complex_xml = complex_xml();

    group.throughput(Throughput::Bytes(simple_xml.len() as u64));
    group.bench_function("simple", |b| {
        b.iter(|| from_str::<Simple>(black_box(simple_xml)))
    });

    group.throughput(Throughput::Bytes(medium_xml.len() as u64));
    group.bench_function("medium", |b| {
        b.iter(|| from_str::<Medium>(black_box(&medium_xml)))
    });

    group.throughput(Throughput::Bytes(complex_xml.len() as u64));
    group.bench_function("complex", |b| {
        b.iter(|| from_str::<Company>(black_box(&complex_xml)))
    });

    group.finish();
}

fn bench_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("Roundtrip");
    
    let simple = simple_data();
    let medium = medium_data();
    let complex = complex_data();

    group.bench_function("simple", |b| {
        b.iter(|| {
            let xml = to_string(black_box(&simple)).unwrap();
            from_str::<Simple>(black_box(&xml)).unwrap()
        })
    });

    group.bench_function("medium", |b| {
        b.iter(|| {
            let xml = to_string(black_box(&medium)).unwrap();
            from_str::<Medium>(black_box(&xml)).unwrap()
        })
    });

    group.bench_function("complex", |b| {
        b.iter(|| {
            let xml = to_string(black_box(&complex)).unwrap();
            from_str::<Company>(black_box(&xml)).unwrap()
        })
    });

    group.finish();
}

fn bench_escape(c: &mut Criterion) {
    use serde_xml::escape;
    
    let mut group = c.benchmark_group("Escape");
    
    let no_escape = "This is a simple string with no special characters at all.";
    let some_escape = "This string has <angle brackets> & ampersands.";
    let heavy_escape = "<<<<>>>> &&&&& \"\"\"\" ''''";

    group.bench_function("no_escape", |b| {
        b.iter(|| escape(black_box(no_escape)))
    });

    group.bench_function("some_escape", |b| {
        b.iter(|| escape(black_box(some_escape)))
    });

    group.bench_function("heavy_escape", |b| {
        b.iter(|| escape(black_box(heavy_escape)))
    });

    group.finish();
}

fn bench_xml_reader(c: &mut Criterion) {
    use serde_xml::{XmlReader, XmlEvent};
    
    let mut group = c.benchmark_group("XmlReader");
    
    let xml = r#"<?xml version="1.0"?>
        <root>
            <child1 attr="value">Text content</child1>
            <child2>
                <nested>Deep text</nested>
            </child2>
            <child3/>
        </root>
    "#;

    group.throughput(Throughput::Bytes(xml.len() as u64));
    group.bench_function("parse_events", |b| {
        b.iter(|| {
            let mut reader = XmlReader::from_str(black_box(xml));
            let mut count = 0;
            loop {
                match reader.next_event() {
                    Ok(XmlEvent::Eof) => break,
                    Ok(_) => count += 1,
                    Err(_) => break,
                }
            }
            count
        })
    });

    group.finish();
}

fn bench_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("Scaling");
    
    for size in [1, 10, 100, 1000].iter() {
        let data: Vec<Simple> = (0..*size)
            .map(|i| Simple {
                name: format!("Item {}", i),
                value: i as i64,
                active: i % 2 == 0,
            })
            .collect();

        #[derive(Serialize, Deserialize)]
        struct Wrapper {
            item: Vec<Simple>,
        }

        let wrapper = Wrapper { item: data };
        let xml = to_string(&wrapper).unwrap();

        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(
            BenchmarkId::new("serialize", size),
            &wrapper,
            |b, data| b.iter(|| to_string(black_box(data))),
        );

        group.bench_with_input(
            BenchmarkId::new("deserialize", size),
            &xml,
            |b, xml| b.iter(|| from_str::<Wrapper>(black_box(xml))),
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_serialization,
    bench_deserialization,
    bench_roundtrip,
    bench_escape,
    bench_xml_reader,
    bench_scaling,
);

criterion_main!(benches);
