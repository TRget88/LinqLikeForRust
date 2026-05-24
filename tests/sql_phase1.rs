//! Integration tests for the Phase 1 SQL query builder.
//!
//! These verify the *string output* — there's no driver in Phase 1, so
//! correctness is "does it produce the SQL we expect, with the right
//! parameter binds?" Compile-time type-mismatch rejection is exercised
//! by the `compile_fail` doctests inside the module rustdoc.

use linq_rs::sql::*;

linq_rs::table! {
    users (id) {
        id     -> Integer,
        name   -> Text,
        age    -> Integer,
        score  -> Float,
        active -> Boolean,
    }
}

linq_rs::table! {
    orders (id) {
        id          -> Integer,
        customer_id -> Integer,
        product     -> Text,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Minimum viable: bare SELECT *
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn select_star() {
    let q = users::table().to_sql();
    assert_eq!(q.sql, "SELECT * FROM users");
    assert!(q.params.is_empty());
}

#[test]
fn select_single_column() {
    let q = users::table().select(users::name).to_sql();
    assert_eq!(q.sql, "SELECT name FROM users");
}

#[test]
fn select_tuple_of_columns() {
    let q = users::table()
        .select((users::id, users::name, users::age))
        .to_sql();
    assert_eq!(q.sql, "SELECT id, name, age FROM users");
}

// ─────────────────────────────────────────────────────────────────────────────
// WHERE — single predicates by SQL type
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn where_integer_gt() {
    let q = users::table().filter(users::age.gt(18)).to_sql();
    assert_eq!(q.sql, "SELECT * FROM users WHERE (age > ?)");
    assert_eq!(q.params, vec![SqlValue::Integer(18)]);
}

#[test]
fn where_integer_full_range_of_comparisons() {
    for (op, expected) in [
        ("eq", "(age = ?)"),
        ("ne", "(age != ?)"),
        ("lt", "(age < ?)"),
        ("lte", "(age <= ?)"),
        ("gt", "(age > ?)"),
        ("gte", "(age >= ?)"),
    ] {
        let q = match op {
            "eq" => users::table().filter(users::age.eq(18)).to_sql(),
            "ne" => users::table().filter(users::age.ne(18)).to_sql(),
            "lt" => users::table().filter(users::age.lt(18)).to_sql(),
            "lte" => users::table().filter(users::age.lte(18)).to_sql(),
            "gt" => users::table().filter(users::age.gt(18)).to_sql(),
            "gte" => users::table().filter(users::age.gte(18)).to_sql(),
            _ => unreachable!(),
        };
        assert_eq!(q.sql, format!("SELECT * FROM users WHERE {expected}"));
        assert_eq!(q.params, vec![SqlValue::Integer(18)]);
    }
}

#[test]
fn where_text_eq() {
    let q = users::table().filter(users::name.eq("Alice")).to_sql();
    assert_eq!(q.sql, "SELECT * FROM users WHERE (name = ?)");
    assert_eq!(q.params, vec![SqlValue::Text("Alice".to_string())]);
}

#[test]
fn where_text_like() {
    let q = users::table().filter(users::name.like("A%")).to_sql();
    assert_eq!(q.sql, "SELECT * FROM users WHERE (name LIKE ?)");
    assert_eq!(q.params, vec![SqlValue::Text("A%".to_string())]);
}

#[test]
fn where_bool_column_used_directly() {
    // A Boolean column IS an Expr<SqlType = Boolean>, so it can serve
    // as a predicate without an explicit .eq(true).
    let q = users::table().filter(users::active).to_sql();
    assert_eq!(q.sql, "SELECT * FROM users WHERE active");
}

#[test]
fn where_float_comparison() {
    let q = users::table().filter(users::score.gte(4.5f64)).to_sql();
    assert_eq!(q.sql, "SELECT * FROM users WHERE (score >= ?)");
    assert_eq!(q.params, vec![SqlValue::Float(4.5)]);
}

#[test]
fn where_is_null() {
    let q = users::table().filter(users::name.is_null()).to_sql();
    assert_eq!(q.sql, "SELECT * FROM users WHERE (name IS NULL)");
    assert!(q.params.is_empty());
}

// ─────────────────────────────────────────────────────────────────────────────
// Compound predicates
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn and_combinator() {
    let q = users::table()
        .filter(users::age.gt(18).and(users::age.lt(65)))
        .to_sql();
    assert_eq!(q.sql, "SELECT * FROM users WHERE ((age > ?) AND (age < ?))");
    assert_eq!(q.params, vec![SqlValue::Integer(18), SqlValue::Integer(65)]);
}

#[test]
fn or_combinator() {
    let q = users::table()
        .filter(users::name.eq("Alice").or(users::name.eq("Bob")))
        .to_sql();
    assert_eq!(
        q.sql,
        "SELECT * FROM users WHERE ((name = ?) OR (name = ?))"
    );
    assert_eq!(
        q.params,
        vec![
            SqlValue::Text("Alice".to_string()),
            SqlValue::Text("Bob".to_string()),
        ]
    );
}

#[test]
fn not_negation() {
    let q = users::table().filter(not(users::active)).to_sql();
    assert_eq!(q.sql, "SELECT * FROM users WHERE NOT (active)");
}

#[test]
fn multiple_filter_calls_anded_together() {
    let q = users::table()
        .filter(users::age.gt(18))
        .filter(users::active.eq(true))
        .filter(users::name.like("A%"))
        .to_sql();
    assert_eq!(
        q.sql,
        "SELECT * FROM users WHERE (age > ?) AND (active = ?) AND (name LIKE ?)"
    );
    assert_eq!(q.params.len(), 3);
}

// ─────────────────────────────────────────────────────────────────────────────
// ORDER BY / LIMIT / OFFSET
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn order_by_asc() {
    let q = users::table().order_by(users::age).to_sql();
    assert_eq!(q.sql, "SELECT * FROM users ORDER BY age");
}

#[test]
fn order_by_desc() {
    let q = users::table().order_by_desc(users::age).to_sql();
    assert_eq!(q.sql, "SELECT * FROM users ORDER BY age DESC");
}

#[test]
fn order_by_multiple_columns_preserves_call_order() {
    let q = users::table()
        .order_by(users::age)
        .order_by_desc(users::name)
        .to_sql();
    assert_eq!(q.sql, "SELECT * FROM users ORDER BY age, name DESC");
}

#[test]
fn limit_only() {
    let q = users::table().limit(10).to_sql();
    assert_eq!(q.sql, "SELECT * FROM users LIMIT 10");
}

#[test]
fn offset_only() {
    let q = users::table().offset(20).to_sql();
    assert_eq!(q.sql, "SELECT * FROM users OFFSET 20");
}

#[test]
fn limit_and_offset() {
    let q = users::table().limit(10).offset(20).to_sql();
    assert_eq!(q.sql, "SELECT * FROM users LIMIT 10 OFFSET 20");
}

// ─────────────────────────────────────────────────────────────────────────────
// Full kitchen sink — verifies clauses appear in canonical SQL order
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn full_query_clause_ordering() {
    let q = users::table()
        .filter(users::age.gt(18))
        .filter(users::active.eq(true))
        .select((users::id, users::name))
        .order_by_desc(users::age)
        .limit(10)
        .offset(5)
        .to_sql();
    assert_eq!(
        q.sql,
        "SELECT id, name FROM users \
         WHERE (age > ?) AND (active = ?) \
         ORDER BY age DESC \
         LIMIT 10 OFFSET 5"
    );
    assert_eq!(
        q.params,
        vec![SqlValue::Integer(18), SqlValue::Boolean(true)]
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Cross-table — verify tables are independent
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn distinct_tables_emit_distinct_sql() {
    let q1 = users::table().to_sql();
    let q2 = orders::table().to_sql();
    assert_eq!(q1.sql, "SELECT * FROM users");
    assert_eq!(q2.sql, "SELECT * FROM orders");
}

#[test]
fn orders_query_with_filter() {
    let q = orders::table()
        .filter(orders::customer_id.eq(42))
        .select((orders::id, orders::product))
        .to_sql();
    assert_eq!(
        q.sql,
        "SELECT id, product FROM orders WHERE (customer_id = ?)"
    );
    assert_eq!(q.params, vec![SqlValue::Integer(42)]);
}

// ─────────────────────────────────────────────────────────────────────────────
// Parameter binding — verify nothing user-supplied is inlined
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn sql_injection_attempt_goes_into_params_not_sql() {
    let nasty = "Alice'; DROP TABLE users; --";
    let q = users::table().filter(users::name.eq(nasty)).to_sql();
    assert_eq!(q.sql, "SELECT * FROM users WHERE (name = ?)");
    assert_eq!(q.params, vec![SqlValue::Text(nasty.to_string())]);
    // The dangerous text appears in params, never in the SQL string.
    assert!(!q.sql.contains("DROP"));
}

#[test]
fn string_owned_works_as_bind() {
    let name: String = "Bob".to_string();
    let q = users::table().filter(users::name.eq(name)).to_sql();
    assert_eq!(q.params, vec![SqlValue::Text("Bob".to_string())]);
}

#[test]
fn primary_key_constant_is_emitted_by_macro() {
    assert_eq!(users::PRIMARY_KEY, &["id"]);
    assert_eq!(orders::PRIMARY_KEY, &["id"]);
}

// ─────────────────────────────────────────────────────────────────────────────
// Lit<T> explicit wrapper
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn explicit_lit_int() {
    let q = users::table()
        .filter(users::age.eq(Lit::<Integer>::int(42)))
        .to_sql();
    assert_eq!(q.sql, "SELECT * FROM users WHERE (age = ?)");
    assert_eq!(q.params, vec![SqlValue::Integer(42)]);
}

#[test]
fn explicit_lit_text() {
    let q = users::table()
        .filter(users::name.eq(Lit::<Text>::text("Alice")))
        .to_sql();
    assert_eq!(q.params, vec![SqlValue::Text("Alice".to_string())]);
}
