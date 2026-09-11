//! The five call sites the design has to support, plus the two properties
//! that make it a *seam* rather than an escape hatch: the erased form renders
//! byte-identical SQL, and the erased form still evaluates in memory.

use linq_rs_sql::prelude::*;
use linq_rs_sql::{BoxedRows, SqlValue};

table! {
    employees (id) {
        id     -> Integer,
        name   -> Text,
        dept   -> Text,
        salary -> Integer,
        active -> Boolean,
    }
}

#[derive(Debug)]
pub struct Employee {
    pub id: i64,
    pub name: String,
    pub dept: String,
    pub salary: i64,
    pub active: bool,
}

entity! {
    Employee => employees {
        id: Integer = id,
        name: Text = name,
        dept: Text = dept,
        salary: Integer = salary,
        active: Boolean = active,
    }
}

fn staff() -> Vec<Employee> {
    vec![
        emp(1, "Ada", "eng", 180_000, true),
        emp(2, "Bo", "sales", 190_000, true),
        emp(3, "Cy", "eng", 90_000, false),
        emp(4, "Di", "eng", 210_000, true),
        emp(5, "Ed", "ops", 120_000, true),
    ]
}

fn emp(id: i64, name: &str, dept: &str, salary: i64, active: bool) -> Employee {
    Employee {
        id,
        name: name.into(),
        dept: dept.into(),
        salary,
        active,
    }
}

fn ids<'a, I: Iterator<Item = &'a Employee>>(it: I) -> Vec<i64> {
    it.map(|e| e.id).collect()
}

// ═══════════════════════════════════════════════════════════════════════════
// CALL SITE 1 — conditional filters in a loop
// ═══════════════════════════════════════════════════════════════════════════

/// The exact shape from the brief: `if flag { q = q.filter(..) }`, repeated.
#[test]
fn call_site_conditional_filters_in_a_loop() {
    struct Search {
        dept: Option<&'static str>,
        min_salary: Option<i64>,
        active_only: bool,
        name_prefix: Option<String>,
    }

    fn build(s: &Search) -> BoxedRows<'static, Employee> {
        let mut q = query::<Employee>().into_boxed();
        if let Some(d) = s.dept {
            q = q.filter(employees::dept.eq(d));
        }
        if let Some(m) = s.min_salary {
            q = q.filter(employees::salary.gte(m));
        }
        if s.active_only {
            q = q.filter(employees::active.eq(true));
        }
        if let Some(p) = &s.name_prefix {
            q = q.filter(employees::name.like(format!("{p}%")));
        }
        q
    }

    let none = Search {
        dept: None,
        min_salary: None,
        active_only: false,
        name_prefix: None,
    };
    let all = Search {
        dept: Some("eng"),
        min_salary: Some(100_000),
        active_only: true,
        name_prefix: Some("A".into()),
    };
    let rows = staff();

    // No filters selected: no WHERE clause at all, every row kept.
    let q = build(&none);
    assert_eq!(q.to_sql().sql, "SELECT * FROM employees");
    assert_eq!(ids(q.to_memory(&rows)), [1, 2, 3, 4, 5]);

    // All four selected: AND-chained in the order they were added.
    let q = build(&all);
    assert_eq!(
        q.to_sql().sql,
        "SELECT * FROM employees WHERE ((((dept = ?) AND (salary >= ?)) AND (active = ?)) AND (name LIKE ?))"
    );
    assert_eq!(
        q.to_sql().params,
        vec![
            SqlValue::Text("eng".into()),
            SqlValue::Integer(100_000),
            SqlValue::Boolean(true),
            SqlValue::Text("A%".into()),
        ]
    );
    assert_eq!(ids(q.to_memory(&rows)), [1]);

    // A middle subset — the combinatorial point of dynamic composition.
    let some = Search {
        dept: Some("eng"),
        min_salary: None,
        active_only: true,
        name_prefix: None,
    };
    let q = build(&some);
    assert_eq!(ids(q.to_memory(&rows)), [1, 4]);
}

/// `order_by` is conditional too, which the typed path cannot express:
/// there it moves `Unordered` -> `Ordered`, so the two branches have
/// different types.
#[test]
fn call_site_conditional_order_by() {
    let rows = staff();
    for (sort, expected) in [(false, vec![1, 2, 4, 5]), (true, vec![4, 2, 1, 5])] {
        let mut q = query::<Employee>()
            .into_boxed()
            .filter(employees::active.eq(true));
        if sort {
            q = q.order_by_desc(employees::salary);
        }
        assert_eq!(ids(q.to_memory(&rows)), expected);
        // ... and the cost of that choice is only visible at runtime:
        assert_eq!(q.to_memory(&rows).is_streaming(), !sort);
        assert_eq!(q.to_sql().sql.contains("ORDER BY salary DESC"), sort);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// CALL SITE 2 — a query in a struct field
// ═══════════════════════════════════════════════════════════════════════════

struct Report {
    title: &'static str,
    query: BoxedRows<'static, Employee>,
}

#[test]
fn call_site_query_in_a_struct_field() {
    let r = Report {
        title: "well-paid engineers",
        query: query::<Employee>()
            .into_boxed()
            .filter(employees::dept.eq("eng"))
            .filter(employees::salary.gt(100_000i64))
            .order_by(employees::id),
    };
    let rows = staff();

    assert_eq!(r.title, "well-paid engineers");
    // `to_memory` takes `&self`, so a stored query runs more than once.
    assert_eq!(ids(r.query.to_memory(&rows)), [1, 4]);
    assert_eq!(ids(r.query.to_memory(&rows)), [1, 4]);
    assert_eq!(
        r.query.to_sql().sql,
        "SELECT * FROM employees WHERE ((dept = ?) AND (salary > ?)) ORDER BY id"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// CALL SITE 3 — return a query from a function
// ═══════════════════════════════════════════════════════════════════════════

/// A named return type, not `impl Trait`: it can be stored in a field, named
/// in a `Vec`, and returned from either arm of an `if`.
fn recent_hires(min_id: i64, dept: Option<&str>) -> BoxedRows<'static, Employee> {
    let mut q = query::<Employee>()
        .into_boxed()
        .filter(employees::id.gte(min_id));
    if let Some(d) = dept {
        // `.to_string()` because this returns `'static`; see the borrowed test.
        q = q.filter(employees::dept.eq(d.to_string()));
    }
    q
}

#[test]
fn call_site_return_a_query_from_a_function() {
    let rows = staff();
    assert_eq!(ids(recent_hires(3, None).to_memory(&rows)), [3, 4, 5]);
    assert_eq!(ids(recent_hires(3, Some("eng")).to_memory(&rows)), [3, 4]);
}

/// MEASURED LIMITATION, and it is **not** caused by boxing.
///
/// `BoxedRows` carries `'a` like Diesel's `BoxedSelectStatement`, but in Phase 1
/// `'a` can only ever be `'static`, so `Boxed<Row>` is the alias worth using.
/// The cause is upstream and predates this design: `to_memory` binds
/// `P: for<'x> Eval<'x, Row>`, and the literal impl is
/// `impl<'r, 'a, Row> Eval<'r, Row> for &'a str where 'a: 'r`. Quantifying `'x`
/// over *every* lifetime therefore demands `'a: 'static`.
///
/// The identical error appears on the un-boxed typed path against the
/// unmodified published crate:
///
/// ```text
/// error[E0597]: `owned` does not live long enough
///   --> tests/borrowed.rs:12:60
///    | let q = query::<Employee>().filter(employees::dept.eq(&owned[..]));
///    |   argument requires that `owned` is borrowed for `'static`
/// ```
///
/// And through `into_boxed`, the same cause with a different code:
///
/// ```text
/// error[E0521]: borrowed data escapes outside of function
///   --> tests/boxed.rs:225:45
///    | .filter(employees::dept.eq(dept))
///    |   argument requires that `'a` must outlive `'static`
/// ```
///
/// The workaround is the one `Query` already documents for its own `'static`
/// WHERE storage: own the string.
#[test]
fn call_site_borrowed_predicate_needs_to_string() {
    fn in_dept(dept: &str) -> Boxed<Employee> {
        query::<Employee>()
            .into_boxed()
            .filter(employees::dept.eq(dept.to_string()))
    }
    let rows = staff();
    let owned = String::from("eng");
    let q = in_dept(&owned);
    assert_eq!(ids(q.to_memory(&rows)), [1, 3, 4]);
    assert_eq!(q.to_sql().sql, "SELECT * FROM employees WHERE (dept = ?)");
}

// ═══════════════════════════════════════════════════════════════════════════
// CALL SITE 4 — several queries in a Vec
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn call_site_several_queries_in_a_vec() {
    let queries: Vec<BoxedRows<'static, Employee>> = vec![
        query::<Employee>().into_boxed(),
        query::<Employee>()
            .into_boxed()
            .filter(employees::dept.eq("eng")),
        query::<Employee>()
            .into_boxed()
            .filter(employees::salary.gt(150_000i64))
            .order_by_desc(employees::salary),
        recent_hires(4, None),
    ];
    let rows = staff();

    let results: Vec<Vec<i64>> = queries.iter().map(|q| ids(q.to_memory(&rows))).collect();
    assert_eq!(
        results,
        vec![
            vec![1, 2, 3, 4, 5],
            vec![1, 3, 4],
            vec![4, 2, 1],
            vec![4, 5]
        ]
    );

    let sql: Vec<String> = queries.iter().map(|q| q.to_sql().sql).collect();
    assert_eq!(sql[0], "SELECT * FROM employees");
    assert_eq!(
        sql[2],
        "SELECT * FROM employees WHERE (salary > ?) ORDER BY salary DESC"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// CALL SITE 5 — the typed path, unchanged
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn call_site_typed_path_unchanged() {
    let rows = staff();

    // Streaming, monomorphised, no `into_boxed` anywhere.
    let q = query::<Employee>()
        .filter(employees::dept.eq("eng"))
        .filter(employees::salary.gt(100_000i64));
    assert_eq!(
        q.to_sql().sql,
        "SELECT * FROM employees WHERE ((dept = ?) AND (salary > ?))"
    );
    assert_eq!(ids(q.to_memory(&rows)), [1, 4]);

    // The `Ordered` type-state still forces `to_memory_sorted`.
    let q = query::<Employee>()
        .filter(employees::active.eq(true))
        .order_by_desc(employees::salary)
        .limit(2);
    assert_eq!(ids(q.to_memory_sorted(&rows)), [4, 2]);

    // `pred!` is untouched.
    let q =
        query::<Employee>().filter(pred!(employees, |e| e.salary > 100_000i64 && e.dept == "eng"));
    assert_eq!(
        q.to_sql().sql,
        "SELECT * FROM employees WHERE ((salary > ?) AND (dept = ?))"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// The seam properties
// ═══════════════════════════════════════════════════════════════════════════

/// Erasure must not change the SQL by one byte, or the two forms are two
/// languages.
#[test]
fn erased_sql_is_byte_identical_to_typed_sql() {
    let typed = query::<Employee>()
        .filter(employees::dept.eq("eng"))
        .filter(employees::salary.gt(100_000i64))
        .filter(employees::name.like("A%"))
        .order_by_desc(employees::salary)
        .limit(10)
        .offset(2);

    let erased = query::<Employee>()
        .into_boxed()
        .filter(employees::dept.eq("eng"))
        .filter(employees::salary.gt(100_000i64))
        .filter(employees::name.like("A%"))
        .order_by_desc(employees::salary)
        .limit(10)
        .offset(2);

    assert_eq!(typed.to_sql(), erased.to_sql());
    assert_eq!(
        erased.to_sql().sql,
        "SELECT * FROM employees WHERE (((dept = ?) AND (salary > ?)) AND (name LIKE ?)) \
         ORDER BY salary DESC LIMIT 10 OFFSET 2"
    );
}

/// And the same erased value must agree with the typed value row for row.
#[test]
fn erased_memory_agrees_with_typed_memory() {
    let rows = staff();
    let typed = query::<Employee>()
        .filter(employees::dept.eq("eng"))
        .filter(employees::salary.gt(100_000i64));
    let erased = query::<Employee>()
        .into_boxed()
        .filter(employees::dept.eq("eng"))
        .filter(employees::salary.gt(100_000i64));
    assert_eq!(ids(typed.to_memory(&rows)), ids(erased.to_memory(&rows)));
}

/// Half-erased: box a query that already had typed filters, then add more.
#[test]
fn into_boxed_preserves_already_accumulated_clauses() {
    let rows = staff();
    let q = query::<Employee>()
        .filter(employees::dept.eq("eng"))
        .limit(2)
        .into_boxed()
        .filter(employees::active.eq(true));
    assert_eq!(
        q.to_sql().sql,
        "SELECT * FROM employees WHERE ((dept = ?) AND (active = ?)) LIMIT 2"
    );
    assert_eq!(ids(q.to_memory(&rows)), [1, 4]);
}

/// `.or()` still composes inside a boxed filter, and `not(..)` too.
#[test]
fn erased_keeps_the_whole_expression_language() {
    let rows = staff();
    let q = query::<Employee>().into_boxed().filter(
        employees::dept
            .eq("eng")
            .or(employees::salary.gt(150_000i64)),
    );
    assert_eq!(ids(q.to_memory(&rows)), [1, 2, 3, 4]);
    assert_eq!(
        q.to_sql().sql,
        "SELECT * FROM employees WHERE ((dept = ?) OR (salary > ?))"
    );

    let q = query::<Employee>()
        .into_boxed()
        .filter(linq_rs_sql::not(employees::dept.eq("eng")));
    assert_eq!(ids(q.to_memory(&rows)), [2, 5]);
}

/// Streaming is still streaming on the erased, unordered path: `to_memory` on
/// an infinite source with a `LIMIT` terminates.
#[test]
fn erased_unordered_path_still_streams() {
    let rows = staff();
    let q = query::<Employee>()
        .into_boxed()
        .filter(employees::active.eq(true))
        .limit(2);
    let mut it = q.to_memory(rows.iter().cycle());
    assert!(it.is_streaming());
    assert_eq!(it.next().map(|e| e.id), Some(1));
    assert_eq!(it.next().map(|e| e.id), Some(2));
    assert_eq!(it.next().map(|e| e.id), None);
}

/// `LIMIT`/`OFFSET` semantics carry over unchanged.
#[test]
fn erased_limit_and_offset_match_typed() {
    let rows = staff();
    let typed = query::<Employee>()
        .filter(employees::active.eq(true))
        .offset(1)
        .limit(2);
    let erased = query::<Employee>()
        .into_boxed()
        .filter(employees::active.eq(true))
        .offset(1)
        .limit(2);
    assert_eq!(ids(typed.to_memory(&rows)), [2, 4]);
    assert_eq!(ids(erased.to_memory(&rows)), [2, 4]);
}

/// `boxed_query()` is the same thing without the two-step.
#[test]
fn boxed_query_shorthand() {
    let rows = staff();
    let q = boxed_query::<Employee>().filter(employees::salary.gt(150_000i64));
    assert_eq!(ids(q.to_memory(&rows)), [1, 2, 4]);
}
