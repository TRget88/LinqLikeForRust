//! Build SQL strings with the typed query builder.
//!
//! Run with: `cargo run --example sql_query`

use linq_rs::sql::*;

linq_rs::table! {
    employees (id) {
        id     -> Integer,
        name   -> Text,
        dept   -> Text,
        salary -> Integer,
        active -> Boolean,
    }
}

fn main() {
    // 1. Bare SELECT *
    let q = employees::table().to_sql();
    println!("Query 1:\n  {}\n  params = {:?}\n", q.sql, q.params);

    // 2. WHERE with one predicate
    let q = employees::table()
        .filter(employees::salary.gt(100_000))
        .to_sql();
    println!("Query 2:\n  {}\n  params = {:?}\n", q.sql, q.params);

    // 3. Compound WHERE — AND + LIKE + boolean column used directly
    let q = employees::table()
        .filter(employees::salary.gt(100_000))
        .filter(employees::name.like("A%"))
        .filter(employees::active)
        .select((employees::id, employees::name, employees::salary))
        .order_by_desc(employees::salary)
        .limit(5)
        .to_sql();
    println!("Query 3:\n  {}\n  params = {:?}\n", q.sql, q.params);

    // 4. OR + NOT
    let q = employees::table()
        .filter(
            employees::dept
                .eq("Eng")
                .or(employees::dept.eq("Sales"))
                .and(not(employees::name.is_null())),
        )
        .to_sql();
    println!("Query 4:\n  {}\n  params = {:?}\n", q.sql, q.params);

    // 5. Parameter binding handles unsafe input.
    let nasty = "Alice'; DROP TABLE employees; --";
    let q = employees::table()
        .filter(employees::name.eq(nasty))
        .to_sql();
    println!(
        "Query 5 (injection-safe):\n  {}\n  params = {:?}\n",
        q.sql, q.params
    );
    assert!(!q.sql.contains("DROP"));
}
