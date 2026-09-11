//! Shared fixture for the adversarial suite.
#![allow(dead_code)]

pub use linq_rs_sql::prelude::*;
pub use linq_rs_sql::{not, BoolOps, DynPred, SqlValue};

linq_rs_sql::table! {
    employees (id) {
        id     -> Integer,
        dept   -> Text,
        name   -> Text,
        salary -> Integer,
        active -> Boolean,
        score  -> Float,
    }
}

#[derive(Debug, Clone)]
pub struct Employee {
    pub id: i64,
    pub dept: String,
    pub name: String,
    pub salary: i64,
    pub active: bool,
    pub score: f64,
}

linq_rs_sql::entity! {
    Employee => employees {
        id: Integer = id,
        dept: Text = dept,
        name: Text = name,
        salary: Integer = salary,
        active: Boolean = active,
        score: Float = score,
    }
}

// A second table, to probe cross-table misuse.
linq_rs_sql::table! {
    depts (id) {
        id   -> Integer,
        name -> Text,
    }
}

pub struct Dept {
    pub id: i64,
    pub name: String,
}

linq_rs_sql::entity! {
    Dept => depts {
        id: Integer = id,
        name: Text = name,
    }
}

pub fn e(id: i64, dept: &str, name: &str, salary: i64, active: bool, score: f64) -> Employee {
    Employee {
        id,
        dept: dept.to_string(),
        name: name.to_string(),
        salary,
        active,
        score,
    }
}

/// The corpus. Deliberately has ties on `dept` and on `salary` so ORDER BY
/// stability is observable, and a couple of rows that sit exactly on
/// comparison boundaries.
pub fn corpus() -> Vec<Employee> {
    vec![
        e(1, "eng", "Alice", 180_000, true, 4.5),
        e(2, "sales", "Bob", 150_000, true, 3.25),
        e(3, "eng", "Ann", 100_000, false, 4.5),
        e(4, "eng", "Zed", 200_000, true, 1.0),
        e(5, "hr", "Carol", 90_000, false, 2.75),
        e(6, "sales", "Dave", 150_000, false, 3.25),
        e(7, "eng", "Amir", 100_000, true, 0.5),
        e(8, "hr", "Eve", 210_000, true, 5.0),
    ]
}

pub fn ids<'a, I: Iterator<Item = &'a Employee>>(it: I) -> Vec<i64> {
    it.map(|r| r.id).collect()
}
