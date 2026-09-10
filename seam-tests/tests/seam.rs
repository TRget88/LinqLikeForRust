use linq_rs::LinqExt;
use linq_rs_sql::rows::query;
use linq_rs_sql::{ExprExt, IntOps, TextOps};
use std::cell::Cell;

use linq_rs_sql::table;

table! {
    employees (id) {
        id     -> Integer,
        name   -> Text,
        dept   -> Text,
        salary -> Integer,
        remote -> Boolean,
    }
}

#[derive(Debug, PartialEq)]
pub struct Employee {
    pub id: i64,
    pub name: String,
    pub dept: String,
    pub salary: i64,
    pub remote: bool,
}

linq_rs_sql::entity! {
    Employee => employees {
        id:     Integer = id,
        name:   Text    = name,
        dept:   Text    = dept,
        salary: Integer = salary,
        remote: Boolean = remote,
    }
}

pub fn staff() -> Vec<Employee> {
    fn e(id: i64, name: &str, dept: &str, salary: i64, remote: bool) -> Employee {
        Employee {
            id,
            name: name.into(),
            dept: dept.into(),
            salary,
            remote,
        }
    }
    vec![
        e(1, "Ada", "eng", 180_000, true),
        e(2, "Brent", "sales", 90_000, false),
        e(3, "Cora", "eng", 150_000, false),
        e(4, "Dev", "eng", 210_000, true),
        e(5, "Eve", "sales", 120_000, true),
    ]
}

#[test]
fn one_value_two_interpreters() {
    let people = staff();
    let q = query::<Employee>()
        .filter(employees::salary.gt(100_000i64))
        .filter(employees::dept.eq("eng"))
        .order_by(employees::dept)
        .order_by_desc(employees::salary)
        .limit(2);

    assert_eq!(
        q.to_sql().sql,
        "SELECT * FROM employees WHERE ((salary > ?) AND (dept = ?)) \
         ORDER BY dept, salary DESC LIMIT 2"
    );
    let ids: Vec<i64> = q.to_memory_sorted(&people).map(|e| e.id).collect();
    assert_eq!(ids, [4, 1]);
}

/// Gap 1: the prototype's `run_in_memory` collected and cloned. This does not.
/// A source iterator that counts how many rows it yielded proves the pipeline
/// is pull-based: `LIMIT 1` over five rows must touch only the rows needed to
/// find the first match, not all five.
#[test]
fn to_memory_streams_and_stops_early() {
    let people = staff();
    let pulled = Cell::new(0usize);
    let counted = people.iter().inspect(|_| pulled.set(pulled.get() + 1));

    let got: Vec<i64> = query::<Employee>()
        .filter(employees::remote.eq(true))
        .limit(1)
        .to_memory(counted)
        .map(|e| e.id)
        .collect();

    assert_eq!(got, [1]); // Ada is row 1 and is remote
    assert_eq!(pulled.get(), 1); // ...so exactly one row was ever pulled
}

/// The same query with the limit lifted still touches only what it must.
#[test]
fn to_memory_is_lazy_per_element() {
    let people = staff();
    let pulled = Cell::new(0usize);
    let counted = people.iter().inspect(|_| pulled.set(pulled.get() + 1));

    let mut it = query::<Employee>()
        .filter(employees::dept.eq("sales"))
        .to_memory(counted);

    assert_eq!(pulled.get(), 0); // building the iterator pulls nothing
    assert_eq!(it.next().map(|e| e.id), Some(2));
    assert_eq!(pulled.get(), 2); // stopped as soon as Brent matched
}

/// D-102: after `.to_memory()` the full `LinqExt` surface is back.
#[test]
fn to_memory_restores_the_full_surface() {
    let people = staff();
    let out: Vec<String> = query::<Employee>()
        .filter(employees::salary.gte(120_000i64))
        .to_memory(&people)
        .group_by_key(|e| e.dept.clone()) // `terminal` — no SQL clause
        .map(|g| format!("{}={}", g.key(), g.elements().len()))
        .collect();
    let mut out = out;
    out.sort();
    assert_eq!(out, ["eng=3", "sales=1"]);
}

/// LIKE, and its D-103 stability class.
#[test]
fn like_translates_and_evaluates() {
    let people = staff();
    let q = query::<Employee>().filter(employees::name.like("%e%"));
    assert_eq!(
        q.to_sql().sql,
        "SELECT * FROM employees WHERE (name LIKE ?)"
    );
    let names: Vec<&str> = q.to_memory(&people).map(|e| e.name.as_str()).collect();
    assert_eq!(names, ["Brent", "Dev", "Eve"]);
}

/// Text comparison is zero-copy: the column evaluates to `&'r str`, borrowed
/// from the row, so no `String` is cloned per row.
#[test]
fn text_predicates_borrow_from_the_row() {
    let people = staff();
    let n = query::<Employee>()
        .filter(employees::dept.lt("f"))
        .to_memory(&people)
        .count();
    assert_eq!(n, 3); // "eng" < "f" for the three engineers
}

#[test]
fn offset_and_limit_agree_across_interpreters() {
    let people = staff();
    let q = query::<Employee>()
        .filter(employees::salary.gt(0i64))
        .offset(1)
        .limit(2);
    assert_eq!(
        q.to_sql().sql,
        "SELECT * FROM employees WHERE (salary > ?) LIMIT 2 OFFSET 1"
    );
    let ids: Vec<i64> = q.to_memory(&people).map(|e| e.id).collect();
    assert_eq!(ids, [2, 3]);
}
