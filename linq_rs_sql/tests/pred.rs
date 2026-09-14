//! `pred!` — the closure-shaped front end. It must produce exactly what the
//! builder form produces, in both interpreters, and it must refuse anything it
//! cannot translate.

use linq_rs_sql::prelude::*;

table! { employees (id) { id -> Integer, name -> Text, dept -> Text, salary -> Integer } }

#[derive(Debug, PartialEq)]
pub struct Employee {
    pub id: i64,
    pub name: String,
    pub dept: String,
    pub salary: i64,
}

entity! {
    Employee => employees {
        id: Integer = id, name: Text = name, dept: Text = dept, salary: Integer = salary,
    }
}

fn staff() -> Vec<Employee> {
    fn e(id: i64, name: &str, dept: &str, salary: i64) -> Employee {
        Employee {
            id,
            name: name.into(),
            dept: dept.into(),
            salary,
        }
    }
    vec![
        e(1, "Ada", "eng", 180_000),
        e(2, "Brent", "sales", 90_000),
        e(3, "Cora", "eng", 95_000),
        e(4, "Dev", "ops", 210_000),
    ]
}

#[test]
fn expands_to_exactly_the_builder_form() {
    // The macro is a front end and nothing more: same SQL, same params.
    let via_macro = query::<Employee>()
        .filter(pred!(employees, |e| e.salary > 100_000i64 && e.dept == "eng"))
        .to_sql();
    let via_builder = query::<Employee>()
        .filter(
            employees::salary
                .gt(100_000i64)
                .and(employees::dept.eq("eng")),
        )
        .to_sql();
    assert_eq!(via_macro.sql, via_builder.sql);
    assert_eq!(via_macro.params, via_builder.params);
}

#[test]
fn and_binds_tighter_than_or() {
    // `a || b && c` must be `a OR (b AND c)`, as in Rust and in SQL.
    let q = query::<Employee>()
        .filter(pred!(employees, |e| e.salary > 200_000i64
            || e.dept == "eng" && e.salary >= 90_000i64))
        .to_sql();
    assert_eq!(
        q.sql,
        "SELECT id, name, dept, salary FROM employees WHERE ((salary > ?) OR ((dept = ?) AND (salary >= ?)))"
    );
}

#[test]
fn parentheses_override_precedence() {
    let q = query::<Employee>()
        .filter(pred!(employees, |e| (e.dept == "eng" || e.dept == "ops")
            && e.salary > 150_000i64))
        .to_sql();
    assert_eq!(
        q.sql,
        "SELECT id, name, dept, salary FROM employees WHERE (((dept = ?) OR (dept = ?)) AND (salary > ?))"
    );
}

#[test]
fn every_comparison_operator_translates() {
    let cases: Vec<(String, &str)> = vec![
        (
            query::<Employee>()
                .filter(pred!(employees, |e| e.salary > 1i64))
                .to_sql()
                .sql,
            ">",
        ),
        (
            query::<Employee>()
                .filter(pred!(employees, |e| e.salary >= 1i64))
                .to_sql()
                .sql,
            ">=",
        ),
        (
            query::<Employee>()
                .filter(pred!(employees, |e| e.salary < 1i64))
                .to_sql()
                .sql,
            "<",
        ),
        (
            query::<Employee>()
                .filter(pred!(employees, |e| e.salary <= 1i64))
                .to_sql()
                .sql,
            "<=",
        ),
        (
            query::<Employee>()
                .filter(pred!(employees, |e| e.dept == "x"))
                .to_sql()
                .sql,
            "=",
        ),
        (
            query::<Employee>()
                .filter(pred!(employees, |e| e.dept != "x"))
                .to_sql()
                .sql,
            // The crate emits `!=`. Every major database accepts it, but the
            // SQL standard operator is `<>` — a dialect question for whenever
            // this grows a renderer per backend, not a defect today.
            "!=",
        ),
    ];
    for (sql, op) in cases {
        assert!(sql.contains(op), "{sql} should contain {op}");
    }
}

#[test]
fn the_same_macro_built_value_evaluates_in_memory() {
    // The whole point: one value, two interpreters.
    let people = staff();
    let q =
        query::<Employee>().filter(pred!(employees, |e| e.salary > 100_000i64 && e.dept == "eng"));

    assert_eq!(
        q.to_sql().sql,
        "SELECT id, name, dept, salary FROM employees WHERE ((salary > ?) AND (dept = ?))"
    );
    let names: Vec<&str> = q.to_memory(&people).map(|e| e.name.as_str()).collect();
    assert_eq!(names, ["Ada"]);
}

#[test]
fn it_still_streams() {
    // A front end must not cost the laziness the seam was built to keep.
    let people = staff();
    let first = query::<Employee>()
        .filter(pred!(employees, |e| e.salary > 1i64))
        .to_memory(&people)
        .next();
    assert_eq!(first.map(|e| e.id), Some(1));
}
