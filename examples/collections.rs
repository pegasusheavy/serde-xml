//! Example demonstrating collections (vectors, maps, etc.).

use serde::{Deserialize, Serialize};
use serde_xml::{from_str, to_string};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Task {
    title: String,
    completed: bool,
    priority: u8,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct TodoList {
    name: String,
    task: Vec<Task>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Book {
    title: String,
    author: String,
    year: u32,
    genre: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Library {
    name: String,
    book: Vec<Book>,
}

fn main() {
    // Example 1: Simple vector of strings in a struct
    println!("=== Todo List Example ===");

    let todo_list = TodoList {
        name: "Weekend Tasks".to_string(),
        task: vec![
            Task {
                title: "Buy groceries".to_string(),
                completed: false,
                priority: 1,
            },
            Task {
                title: "Clean house".to_string(),
                completed: true,
                priority: 2,
            },
            Task {
                title: "Call mom".to_string(),
                completed: false,
                priority: 1,
            },
        ],
    };

    let xml = to_string(&todo_list).expect("Failed to serialize");
    println!("Serialized TodoList:");
    println!("{}", xml);
    println!();

    let parsed: TodoList = from_str(&xml).expect("Failed to deserialize");
    println!("Parsed {} tasks:", parsed.task.len());
    for task in &parsed.task {
        let status = if task.completed { "✓" } else { "○" };
        println!("  {} [P{}] {}", status, task.priority, task.title);
    }
    println!();

    // Example 2: Nested collections
    println!("=== Library Example ===");

    let library = Library {
        name: "City Library".to_string(),
        book: vec![
            Book {
                title: "The Rust Programming Language".to_string(),
                author: "Steve Klabnik".to_string(),
                year: 2018,
                genre: vec!["Programming".to_string(), "Technology".to_string()],
            },
            Book {
                title: "1984".to_string(),
                author: "George Orwell".to_string(),
                year: 1949,
                genre: vec!["Fiction".to_string(), "Dystopian".to_string(), "Political".to_string()],
            },
        ],
    };

    let xml = to_string(&library).expect("Failed to serialize");
    println!("Serialized Library:");
    println!("{}", xml);
    println!();

    let parsed: Library = from_str(&xml).expect("Failed to deserialize");
    println!("Library: {}", parsed.name);
    for book in &parsed.book {
        println!("  📚 {} by {} ({})", book.title, book.author, book.year);
        println!("     Genres: {}", book.genre.join(", "));
    }
    println!();

    // Verify roundtrips
    assert_eq!(todo_list, from_str::<TodoList>(&to_string(&todo_list).unwrap()).unwrap());
    assert_eq!(library, from_str::<Library>(&to_string(&library).unwrap()).unwrap());
    println!("All roundtrip verifications passed!");
}
