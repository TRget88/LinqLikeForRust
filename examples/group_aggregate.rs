//! Group and aggregate — typical "salary summary by department" scenario.
//!
//! Run with: `cargo run --example group_aggregate`

use linq_rs::{LinqExt, ThenBy};

#[derive(Debug, Clone)]
struct Employee {
    name: &'static str,
    dept: &'static str,
    salary: u32,
}

fn main() {
    let employees = vec![
        Employee {
            name: "Alice",
            dept: "Eng",
            salary: 120_000,
        },
        Employee {
            name: "Bob",
            dept: "Eng",
            salary: 95_000,
        },
        Employee {
            name: "Carol",
            dept: "Sales",
            salary: 80_000,
        },
        Employee {
            name: "Dave",
            dept: "Sales",
            salary: 75_000,
        },
        Employee {
            name: "Eve",
            dept: "Eng",
            salary: 130_000,
        },
        Employee {
            name: "Frank",
            dept: "HR",
            salary: 70_000,
        },
    ];

    // 1. Total salary per department, sorted by department.
    println!("Total salary per dept:");
    let mut totals: Vec<_> = employees
        .iter()
        .cloned()
        .aggregate_by_hashed(|e| e.dept, |_| 0u32, |acc, e| acc + e.salary)
        .collect();
    totals.sort_by_key(|(d, _)| *d);
    for (dept, total) in &totals {
        println!("  {dept}: ${total}");
    }

    // 2. Headcount per department.
    println!("\nHeadcount per dept:");
    let mut counts: Vec<_> = employees
        .iter()
        .cloned()
        .count_by_hashed(|e| e.dept)
        .collect();
    counts.sort_by_key(|(d, _)| *d);
    for (dept, n) in &counts {
        println!("  {dept}: {n}");
    }

    // 3. Top earner per department, sorted by department then salary descending.
    println!("\nAll employees, sorted by dept then salary desc:");
    let sorted: Vec<_> = employees
        .into_iter()
        .order_by(|e| e.dept)
        .then_by_descending(|e| e.salary)
        .into_iter()
        .collect();
    for e in sorted {
        println!("  {} ({}) — ${}", e.name, e.dept, e.salary);
    }
}
