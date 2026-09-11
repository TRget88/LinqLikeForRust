//! Adversarial suite for `into_boxed()`. Written against the prototype, not
//! by it.

mod common;
use common::*;
use linq_rs_sql::{Boxed, BoxedRows};

// ───────────────────────────────────────────────────────────────────────────
// 1. More than two clauses
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn six_clauses_byte_identical_and_agree() {
    let rows = corpus();

    let typed = query::<Employee>()
        .filter(employees::dept.eq("eng"))
        .filter(employees::salary.gte(100_000i64))
        .filter(employees::active.eq(true))
        .filter(employees::name.like("A%"))
        .filter(employees::id.lt(100i64))
        .filter(employees::score.gt(0.25f64));

    let erased = query::<Employee>()
        .into_boxed()
        .filter(employees::dept.eq("eng"))
        .filter(employees::salary.gte(100_000i64))
        .filter(employees::active.eq(true))
        .filter(employees::name.like("A%"))
        .filter(employees::id.lt(100i64))
        .filter(employees::score.gt(0.25f64));

    assert_eq!(typed.to_sql(), erased.to_sql(), "SQL diverged at 6 clauses");
    assert_eq!(ids(typed.to_memory(&rows)), ids(erased.to_memory(&rows)));
    assert_eq!(ids(erased.to_memory(&rows)), [1, 7]);
}

/// Erase a query that ALREADY carried typed clauses, then keep adding. The
/// regrouping `And(And(a,b), c)` vs `[And(a,b), c]` must still render the same.
#[test]
fn partial_erasure_regrouping_is_still_byte_identical() {
    for split in 0..=6usize {
        let typed = query::<Employee>()
            .filter(employees::id.gte(1i64))
            .filter(employees::id.lte(8i64))
            .filter(employees::salary.gt(1i64))
            .filter(employees::score.gte(0.0f64))
            .filter(employees::name.ne("nobody"))
            .filter(employees::dept.ne("void"))
            .to_sql();

        // Same six clauses, but `into_boxed()` happens after `split` of them.
        let erased = match split {
            0 => query::<Employee>()
                .into_boxed()
                .filter(employees::id.gte(1i64))
                .filter(employees::id.lte(8i64))
                .filter(employees::salary.gt(1i64))
                .filter(employees::score.gte(0.0f64))
                .filter(employees::name.ne("nobody"))
                .filter(employees::dept.ne("void")),
            1 => query::<Employee>()
                .filter(employees::id.gte(1i64))
                .into_boxed()
                .filter(employees::id.lte(8i64))
                .filter(employees::salary.gt(1i64))
                .filter(employees::score.gte(0.0f64))
                .filter(employees::name.ne("nobody"))
                .filter(employees::dept.ne("void")),
            2 => query::<Employee>()
                .filter(employees::id.gte(1i64))
                .filter(employees::id.lte(8i64))
                .into_boxed()
                .filter(employees::salary.gt(1i64))
                .filter(employees::score.gte(0.0f64))
                .filter(employees::name.ne("nobody"))
                .filter(employees::dept.ne("void")),
            3 => query::<Employee>()
                .filter(employees::id.gte(1i64))
                .filter(employees::id.lte(8i64))
                .filter(employees::salary.gt(1i64))
                .into_boxed()
                .filter(employees::score.gte(0.0f64))
                .filter(employees::name.ne("nobody"))
                .filter(employees::dept.ne("void")),
            4 => query::<Employee>()
                .filter(employees::id.gte(1i64))
                .filter(employees::id.lte(8i64))
                .filter(employees::salary.gt(1i64))
                .filter(employees::score.gte(0.0f64))
                .into_boxed()
                .filter(employees::name.ne("nobody"))
                .filter(employees::dept.ne("void")),
            5 => query::<Employee>()
                .filter(employees::id.gte(1i64))
                .filter(employees::id.lte(8i64))
                .filter(employees::salary.gt(1i64))
                .filter(employees::score.gte(0.0f64))
                .filter(employees::name.ne("nobody"))
                .into_boxed()
                .filter(employees::dept.ne("void")),
            _ => query::<Employee>()
                .filter(employees::id.gte(1i64))
                .filter(employees::id.lte(8i64))
                .filter(employees::salary.gt(1i64))
                .filter(employees::score.gte(0.0f64))
                .filter(employees::name.ne("nobody"))
                .filter(employees::dept.ne("void"))
                .into_boxed(),
        };

        assert_eq!(typed, erased.to_sql(), "split at {split}");
    }
}

// ───────────────────────────────────────────────────────────────────────────
// 2. Deeply nested AND / OR
// ───────────────────────────────────────────────────────────────────────────

/// Build a right-nested `a AND (b OR <rest>)` chain, one node pair per `x`.
macro_rules! deep {
    () => { employees::id.gte(0i64) };
    (x $($rest:tt)*) => {
        BoolOps::and(
            employees::id.gte(0i64),
            BoolOps::or(employees::dept.eq("eng"), deep!($($rest)*)),
        )
    };
}

#[test]
fn deeply_nested_and_or_survives_erasure() {
    let rows = corpus();
    // 24 levels => 48 logical nodes + 49 comparison nodes in one clause.
    let d = deep!(x x x x x x x x x x x x x x x x x x x x x x x x);
    let typed = query::<Employee>().filter(d);
    let sql_typed = typed.to_sql();
    let mem_typed = ids(typed.to_memory(&rows));

    let d = deep!(x x x x x x x x x x x x x x x x x x x x x x x x);
    let erased = query::<Employee>().into_boxed().filter(d);

    assert_eq!(sql_typed, erased.to_sql());
    assert_eq!(mem_typed, ids(erased.to_memory(&rows)));
    assert_eq!(mem_typed, [1, 2, 3, 4, 5, 6, 7, 8]);
}

#[test]
fn nested_or_at_top_of_a_flat_clause_list() {
    let rows = corpus();
    let typed = query::<Employee>()
        .filter(employees::dept.eq("eng").or(employees::dept.eq("hr")))
        .filter(not(employees::active.eq(false)))
        .filter(
            employees::salary
                .gt(150_000i64)
                .or(employees::name.like("A_i%")),
        );
    let erased = query::<Employee>()
        .into_boxed()
        .filter(employees::dept.eq("eng").or(employees::dept.eq("hr")))
        .filter(not(employees::active.eq(false)))
        .filter(
            employees::salary
                .gt(150_000i64)
                .or(employees::name.like("A_i%")),
        );
    assert_eq!(typed.to_sql(), erased.to_sql());
    assert_eq!(ids(typed.to_memory(&rows)), ids(erased.to_memory(&rows)));
    assert_eq!(ids(erased.to_memory(&rows)), [1, 4, 7, 8]);
}

// ───────────────────────────────────────────────────────────────────────────
// 3. Empty predicate set
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn empty_predicate_set() {
    let rows = corpus();
    let q: Boxed<Employee> = query::<Employee>().into_boxed();
    assert_eq!(q.to_sql().sql, "SELECT * FROM employees");
    assert!(q.to_sql().params.is_empty());
    assert_eq!(ids(q.to_memory(&rows)).len(), 8);
    assert!(q.to_memory(&rows).is_streaming());
}

#[test]
fn empty_predicate_set_with_order_limit_offset() {
    let rows = corpus();
    let typed = query::<Employee>()
        .order_by_desc(employees::salary)
        .limit(3)
        .offset(1);
    let erased = query::<Employee>()
        .into_boxed()
        .order_by_desc(employees::salary)
        .limit(3)
        .offset(1);
    assert_eq!(typed.to_sql(), erased.to_sql());
    assert_eq!(
        erased.to_sql().sql,
        "SELECT * FROM employees ORDER BY salary DESC LIMIT 3 OFFSET 1"
    );
    assert_eq!(
        ids(typed.to_memory_sorted(&rows)),
        ids(erased.to_memory(&rows))
    );
    assert!(!erased.to_memory(&rows).is_streaming());
}

#[test]
fn empty_source_and_empty_predicates() {
    let rows: Vec<Employee> = Vec::new();
    let q: Boxed<Employee> = query::<Employee>().into_boxed();
    assert_eq!(ids(q.to_memory(&rows)), Vec::<i64>::new());
    let q = query::<Employee>().into_boxed().order_by(employees::id);
    assert_eq!(ids(q.to_memory(&rows)), Vec::<i64>::new());
}

// ───────────────────────────────────────────────────────────────────────────
// 4. Mixing the erased and typed forms in one program
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn typed_and_erased_side_by_side() {
    let rows = corpus();

    // Typed: streaming, monomorphised.
    let t = query::<Employee>()
        .filter(employees::dept.eq("eng"))
        .filter(employees::salary.gt(100_000i64));
    assert_eq!(
        t.to_sql().sql,
        "SELECT * FROM employees WHERE ((dept = ?) AND (salary > ?))"
    );
    assert_eq!(ids(t.to_memory(&rows)), [1, 4]);

    // Typed type-state still forces the sorted method.
    let t2 = query::<Employee>()
        .filter(employees::active.eq(true))
        .order_by_desc(employees::salary)
        .limit(2);
    assert_eq!(ids(t2.to_memory_sorted(&rows)), [8, 4]);

    // pred! still works on the typed path.
    let t3 = query::<Employee>()
        .filter(linq_rs_sql::pred!(employees, |e| e.salary > 100_000i64 && e.dept == "eng"));
    assert_eq!(ids(t3.to_memory(&rows)), [1, 4]);

    // ... and pred! survives erasure.
    let b3 = query::<Employee>()
        .into_boxed()
        .filter(linq_rs_sql::pred!(employees, |e| e.salary > 100_000i64 && e.dept == "eng"));
    assert_eq!(ids(b3.to_memory(&rows)), [1, 4]);
    assert_eq!(
        b3.to_sql().sql,
        "SELECT * FROM employees WHERE ((salary > ?) AND (dept = ?))"
    );
}

/// A `Box<dyn DynPred>` claims to be an ordinary node in the expression
/// language. Feed one back into `.and()` / `not()` and into the TYPED path.
#[test]
fn boxed_predicate_reenters_the_expression_language() {
    let rows = corpus();
    let b: Box<dyn DynPred<Employee>> = Box::new(employees::dept.eq("eng"));
    let combined = BoolOps::and(b, employees::salary.gt(150_000i64));

    let typed = query::<Employee>().filter(combined);
    // NOTE: `typed.to_sql()` does NOT compile here -- `Rows::to_sql` requires
    // `P: Clone` and `Box<dyn DynPred>` is not `Clone`. Interpreter two works,
    // interpreter one does not. See tests/negatives/boxed_pred_no_to_sql.rs.
    assert_eq!(ids(typed.to_memory(&rows)), [1, 4]);

    // The erased path can hold it, because BoxedRows never clones.
    let b: Box<dyn DynPred<Employee>> = Box::new(employees::dept.eq("eng"));
    let erased = query::<Employee>()
        .into_boxed()
        .filter(BoolOps::and(b, employees::salary.gt(150_000i64)));
    assert_eq!(
        erased.to_sql().sql,
        "SELECT * FROM employees WHERE ((dept = ?) AND (salary > ?))"
    );
    assert_eq!(ids(erased.to_memory(&rows)), [1, 4]);
}

// ───────────────────────────────────────────────────────────────────────────
// 5. Storage, reuse, returning from functions, collections
// ───────────────────────────────────────────────────────────────────────────

struct Report {
    title: &'static str,
    query: Boxed<Employee>,
}

fn recent_hires(min_id: i64, dept: Option<&str>) -> Boxed<Employee> {
    let mut q = query::<Employee>()
        .into_boxed()
        .filter(employees::id.gte(min_id));
    if let Some(d) = dept {
        q = q.filter(employees::dept.eq(d.to_string()));
    }
    q
}

#[test]
fn stored_query_runs_more_than_once() {
    let rows = corpus();
    let r = Report {
        title: "well-paid engineers",
        query: query::<Employee>()
            .into_boxed()
            .filter(employees::dept.eq("eng"))
            .filter(employees::salary.gt(100_000i64))
            .order_by(employees::id),
    };
    assert_eq!(r.title, "well-paid engineers");
    assert_eq!(ids(r.query.to_memory(&rows)), [1, 4]);
    assert_eq!(ids(r.query.to_memory(&rows)), [1, 4]);
    assert_eq!(r.query.to_sql(), r.query.to_sql());
    assert_eq!(
        r.query.to_sql().sql,
        "SELECT * FROM employees WHERE ((dept = ?) AND (salary > ?)) ORDER BY id"
    );
}

#[test]
fn vec_of_queries_through_both_interpreters() {
    let rows = corpus();
    let queries: Vec<Boxed<Employee>> = vec![
        query::<Employee>().into_boxed(),
        query::<Employee>()
            .into_boxed()
            .filter(employees::dept.eq("eng")),
        query::<Employee>()
            .into_boxed()
            .filter(employees::salary.gt(150_000i64))
            .order_by_desc(employees::salary),
        recent_hires(7, None),
    ];
    let mem: Vec<Vec<i64>> = queries.iter().map(|q| ids(q.to_memory(&rows))).collect();
    assert_eq!(
        mem,
        vec![
            vec![1, 2, 3, 4, 5, 6, 7, 8],
            vec![1, 3, 4, 7],
            vec![8, 4, 1],
            vec![7, 8]
        ]
    );
    let sql: Vec<String> = queries.iter().map(|q| q.to_sql().sql).collect();
    assert_eq!(
        sql[2],
        "SELECT * FROM employees WHERE (salary > ?) ORDER BY salary DESC"
    );
}

#[test]
fn conditional_filters_in_a_loop() {
    struct Search {
        dept: Option<&'static str>,
        min_salary: Option<i64>,
        active_only: bool,
        name_prefix: Option<String>,
    }
    fn build(s: &Search) -> Boxed<Employee> {
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
            q = q.filter(employees::name.like(format!("{}%", p)));
        }
        q
    }
    let rows = corpus();

    let all = build(&Search {
        dept: Some("eng"),
        min_salary: Some(100_000),
        active_only: true,
        name_prefix: Some("A".into()),
    });
    assert_eq!(
        all.to_sql().sql,
        "SELECT * FROM employees WHERE ((((dept = ?) AND (salary >= ?)) AND (active = ?)) AND (name LIKE ?))"
    );
    assert_eq!(
        all.to_sql().params,
        vec![
            SqlValue::Text("eng".into()),
            SqlValue::Integer(100_000),
            SqlValue::Boolean(true),
            SqlValue::Text("A%".into())
        ]
    );
    assert_eq!(ids(all.to_memory(&rows)), [1, 7]);

    let none = build(&Search {
        dept: None,
        min_salary: None,
        active_only: false,
        name_prefix: None,
    });
    assert_eq!(none.to_sql().sql, "SELECT * FROM employees");
    assert_eq!(ids(none.to_memory(&rows)).len(), 8);
}

// ───────────────────────────────────────────────────────────────────────────
// 6. Ordering / limit / offset alongside the erased form
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn multi_key_order_matches_typed() {
    let rows = corpus();
    let typed = query::<Employee>()
        .order_by(employees::dept)
        .order_by_desc(employees::salary)
        .order_by(employees::id);
    let erased = query::<Employee>()
        .into_boxed()
        .order_by(employees::dept)
        .order_by_desc(employees::salary)
        .order_by(employees::id);
    assert_eq!(typed.to_sql(), erased.to_sql());
    assert_eq!(
        erased.to_sql().sql,
        "SELECT * FROM employees ORDER BY dept, salary DESC, id"
    );
    assert_eq!(
        ids(typed.to_memory_sorted(&rows)),
        ids(erased.to_memory(&rows))
    );
}

#[test]
fn limit_offset_edges() {
    let rows = corpus();
    for (lim, off) in [
        (None, None),
        (Some(0), None),
        (None, Some(0)),
        (Some(3), Some(2)),
        (Some(100), Some(0)),
        (Some(1), Some(7)),
        (Some(1), Some(8)),
        (Some(2), Some(99)),
        (None, Some(6)),
    ] {
        let mut t = query::<Employee>().filter(employees::id.gte(1i64));
        let mut b = query::<Employee>()
            .into_boxed()
            .filter(employees::id.gte(1i64));
        if let Some(l) = lim {
            t = t.limit(l);
            b = b.limit(l);
        }
        if let Some(o) = off {
            t = t.offset(o);
            b = b.offset(o);
        }
        assert_eq!(t.to_sql(), b.to_sql(), "sql for lim={lim:?} off={off:?}");
        assert_eq!(
            ids(t.to_memory(&rows)),
            ids(b.to_memory(&rows)),
            "mem for lim={lim:?} off={off:?}"
        );

        // and the same again with ORDER BY (materialising path)
        let mut t = query::<Employee>()
            .filter(employees::id.gte(1i64))
            .order_by_desc(employees::id);
        let mut b = query::<Employee>()
            .into_boxed()
            .filter(employees::id.gte(1i64))
            .order_by_desc(employees::id);
        if let Some(l) = lim {
            t = t.limit(l);
            b = b.limit(l);
        }
        if let Some(o) = off {
            t = t.offset(o);
            b = b.offset(o);
        }
        assert_eq!(
            t.to_sql(),
            b.to_sql(),
            "sorted sql for lim={lim:?} off={off:?}"
        );
        assert_eq!(
            ids(t.to_memory_sorted(&rows)),
            ids(b.to_memory(&rows)),
            "sorted mem for lim={lim:?} off={off:?}"
        );
    }
}

#[test]
fn is_streaming_tracks_order_by() {
    let rows = corpus();
    for sort in [false, true] {
        let mut q = query::<Employee>()
            .into_boxed()
            .filter(employees::active.eq(true));
        if sort {
            q = q.order_by_desc(employees::salary);
        }
        assert_eq!(q.to_memory(&rows).is_streaming(), !sort);
        assert_eq!(q.to_sql().sql.contains("ORDER BY salary DESC"), sort);
    }
}

// ───────────────────────────────────────────────────────────────────────────
// 7. Lifetimes
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn query_outlives_the_data() {
    let q = query::<Employee>()
        .into_boxed()
        .filter(employees::dept.eq("eng"));
    {
        let rows = corpus();
        assert_eq!(ids(q.to_memory(&rows)), [1, 3, 4, 7]);
    }
    // q is still usable after the data is gone
    let rows2 = corpus();
    assert_eq!(ids(q.to_memory(&rows2)), [1, 3, 4, 7]);
}

#[test]
fn data_outlives_the_query() {
    let rows = corpus();
    let out = {
        let q = query::<Employee>()
            .into_boxed()
            .filter(employees::dept.eq("hr"));
        // the ITERATOR borrows q, so it must be drained inside the scope
        ids(q.to_memory(&rows))
    };
    assert_eq!(out, [5, 8]);
}

/// The rows the iterator yields must be able to outlive the query value.
#[test]
fn yielded_rows_outlive_the_query() {
    let rows = corpus();
    let kept: Vec<&Employee> = {
        let q = query::<Employee>()
            .into_boxed()
            .filter(employees::dept.eq("hr"));
        q.to_memory(&rows).collect()
    };
    assert_eq!(kept.iter().map(|r| r.id).collect::<Vec<_>>(), [5, 8]);
}

#[test]
fn owned_string_predicate_via_to_string() {
    let rows = corpus();
    let owned = String::from("eng");
    fn in_dept(dept: &str) -> Boxed<Employee> {
        query::<Employee>()
            .into_boxed()
            .filter(employees::dept.eq(dept.to_string()))
    }
    assert_eq!(ids(in_dept(&owned).to_memory(&rows)), [1, 3, 4, 7]);
}

// A row type whose columns BORROW. This is the "borrowed &str columns" case.
linq_rs_sql::table! {
    staff (id) {
        id   -> Integer,
        name -> Text,
    }
}

pub struct BorrowedStaff<'d> {
    pub id: i64,
    pub name: &'d str,
}

impl<'d> linq_rs_sql::rows::Entity for BorrowedStaff<'d> {
    type Table = staff::Marker;
}
impl<'r, 'd> linq_rs_sql::rows::Eval<'r, BorrowedStaff<'d>> for staff::id {
    fn eval(&'r self, row: &'r BorrowedStaff<'d>) -> i64 {
        row.id
    }
}
impl<'r, 'd> linq_rs_sql::rows::Eval<'r, BorrowedStaff<'d>> for staff::name {
    fn eval(&'r self, row: &'r BorrowedStaff<'d>) -> &'r str {
        row.name
    }
}

#[test]
fn row_type_with_borrowed_columns_typed_path() {
    let backing: Vec<String> = vec!["Ann".into(), "Bob".into(), "Amir".into()];
    let rows: Vec<BorrowedStaff<'_>> = backing
        .iter()
        .enumerate()
        .map(|(i, s)| BorrowedStaff {
            id: i as i64 + 1,
            name: s.as_str(),
        })
        .collect();
    let q = query::<BorrowedStaff>().filter(staff::name.like("A%"));
    let got: Vec<i64> = q.to_memory(&rows).map(|r| r.id).collect();
    assert_eq!(got, [1, 3]);
}

#[test]
fn row_type_with_borrowed_columns_erased_path() {
    let backing: Vec<String> = vec!["Ann".into(), "Bob".into(), "Amir".into()];
    let rows: Vec<BorrowedStaff<'_>> = backing
        .iter()
        .enumerate()
        .map(|(i, s)| BorrowedStaff {
            id: i as i64 + 1,
            name: s.as_str(),
        })
        .collect();
    let q: BoxedRows<'_, BorrowedStaff<'_>> = query::<BorrowedStaff>()
        .into_boxed()
        .filter(staff::name.like("A%"));
    let got: Vec<i64> = q.to_memory(&rows).map(|r| r.id).collect();
    assert_eq!(got, [1, 3]);
    assert_eq!(q.to_sql().sql, "SELECT * FROM staff WHERE (name LIKE ?)");
}
