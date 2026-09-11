//! The provider against a real in-memory SQLite. Nothing here is mocked: every
//! assertion runs SQL that `linq_rs_sql` generated, through rusqlite, and checks
//! what comes back.

use linq_rs_sql::prelude::*;
use linq_rs_sql::RowError;
use linq_rs_sqlite::{Error, Sqlite};

table! {
    emp (id) {
        id     -> Integer,
        name   -> Text,
        dept   -> Text,
        salary -> Integer,
        nick   -> Nullable<Text>,
        active -> Boolean,
    }
}

#[derive(Debug, PartialEq)]
pub struct Emp {
    pub id: i64,
    pub name: String,
    pub dept: String,
    pub salary: i64,
    pub nick: Option<String>,
    pub active: bool,
}

entity! {
    Emp => emp {
        id: Integer = id, name: Text = name, dept: Text = dept,
        salary: Integer = salary, nick: Nullable<Text> = nick, active: Boolean = active,
    }
}

/// A database whose PHYSICAL column order deliberately differs from the entity's
/// declared order, so every test here also exercises by-name resolution.
fn db() -> rusqlite::Connection {
    let c = rusqlite::Connection::open_in_memory().unwrap();
    c.execute_batch(
        "CREATE TABLE emp(dept TEXT, active INTEGER NOT NULL, id INTEGER,
                          nick TEXT, salary INTEGER, name TEXT);
         INSERT INTO emp VALUES
           ('eng',   1, 1, 'A',  180000, 'Ada'),
           ('sales', 0, 2, NULL,  90000, 'Bo'),
           ('eng',   1, 3, 'C',   95000, 'Cy'),
           ('eng',   0, 4, NULL, 210000, 'Di');",
    )
    .unwrap();
    c
}

#[test]
fn a_filtered_ordered_query_round_trips() {
    let c = db();
    let db = Sqlite::new(&c);
    let q = query::<Emp>()
        .filter(pred!(emp, |e| e.salary > 100_000i64 && e.dept == "eng"))
        .order_by_desc(emp::salary);

    let got: Vec<Emp> = db.fetch(&q.to_sql()).unwrap();
    assert_eq!(
        got.iter().map(|e| e.id).collect::<Vec<_>>(),
        [4, 1],
        "Di then Ada"
    );
    assert_eq!(got[0].name, "Di");
    assert_eq!(got[0].nick, None);
    assert_eq!(got[1].nick.as_deref(), Some("A"));
}

/// The seam's whole claim: one query value, two interpreters, one answer. Here
/// one side is a real database.
#[test]
fn the_database_and_the_in_memory_interpreter_agree() {
    let c = db();
    let db = Sqlite::new(&c);
    let q = query::<Emp>().filter(pred!(emp, |e| e.salary > 94_000i64 && e.active == true));

    let from_db: Vec<i64> = db
        .fetch::<Emp>(&q.to_sql())
        .unwrap()
        .iter()
        .map(|e| e.id)
        .collect();

    // The same query value, evaluated over the rows the database just gave us.
    let rows: Vec<Emp> = db.fetch(&query::<Emp>().to_sql()).unwrap();
    let in_memory: Vec<i64> = q.to_memory(&rows).map(|e| e.id).collect();

    assert_eq!(from_db, in_memory, "the two interpreters must agree");
    assert_eq!(from_db, [1, 3]);
}

/// D-026 through a real database: NULL is not a value that compares.
#[test]
fn three_valued_logic_matches_the_database() {
    let c = db();
    let db = Sqlite::new(&c);

    let nulls: Vec<i64> = db
        .fetch::<Emp>(&query::<Emp>().filter(emp::nick.is_null()).to_sql())
        .unwrap()
        .iter()
        .map(|e| e.id)
        .collect();
    assert_eq!(nulls, [2, 4]);

    // `NOT (nick = 'A')` is NULL for rows 2 and 4, so they do not survive.
    let not_a: Vec<i64> = db
        .fetch::<Emp>(
            &query::<Emp>()
                .filter(linq_rs_sql::expr::not(emp::nick.eq("A")))
                .to_sql(),
        )
        .unwrap()
        .iter()
        .map(|e| e.id)
        .collect();
    assert_eq!(not_a, [3], "only Cy: rows 2 and 4 are NULL, not false");
}

/// D-027 through a real database.
#[test]
fn a_conditionally_built_query_runs() {
    let c = db();
    let db = Sqlite::new(&c);
    for (min, dept, want) in [
        (None, None, vec![1, 2, 3, 4]),
        (Some(100_000i64), None, vec![1, 4]),
        (Some(100_000i64), Some("eng"), vec![1, 4]),
        (None, Some("sales"), vec![2]),
    ] {
        let mut q = boxed_query::<Emp>();
        if let Some(m) = min {
            q = q.filter(emp::salary.gt(m));
        }
        if let Some(d) = dept {
            q = q.filter(emp::dept.eq(d));
        }
        let got: Vec<i64> = db
            .fetch::<Emp>(&q.to_sql())
            .unwrap()
            .iter()
            .map(|e| e.id)
            .collect();
        assert_eq!(got, want, "min={min:?} dept={dept:?}");
    }
}

#[test]
fn fetch_one_returns_none_on_an_empty_result_set() {
    let c = db();
    let db = Sqlite::new(&c);
    assert!(db
        .fetch_one::<Emp>(&query::<Emp>().filter(emp::id.gt(999i64)).to_sql())
        .unwrap()
        .is_none());
    assert_eq!(
        db.fetch_one::<Emp>(&query::<Emp>().filter(emp::id.eq(3i64)).to_sql())
            .unwrap()
            .unwrap()
            .name,
        "Cy"
    );
}

/// `count` wraps the query rather than rewriting it, so a `LIMIT` survives —
/// a naive `SELECT COUNT(*)` rewrite would silently drop it and over-report.
#[test]
fn count_respects_limit() {
    let c = db();
    let db = Sqlite::new(&c);
    let all = query::<Emp>().filter(emp::salary.gt(1i64));
    assert_eq!(db.count(&all.to_sql()).unwrap(), 4);
    let capped = query::<Emp>().filter(emp::salary.gt(1i64)).limit(2);
    assert_eq!(
        db.count(&capped.to_sql()).unwrap(),
        2,
        "a LIMIT must not be dropped by the count wrapper"
    );
}

/// Hostile input reaches the database as a bound parameter, never as SQL.
#[test]
fn a_hostile_value_cannot_reach_the_sql_text() {
    let c = db();
    let db = Sqlite::new(&c);
    let hostile = "eng'); DROP TABLE emp;--";
    let q = query::<Emp>().filter(emp::dept.eq(hostile));
    assert!(
        !q.to_sql().sql.contains("DROP"),
        "the payload must not be in the SQL"
    );
    assert!(db.fetch::<Emp>(&q.to_sql()).unwrap().is_empty());
    // And the table is still there.
    assert_eq!(db.count(&query::<Emp>().to_sql()).unwrap(), 4);
}

/// D-029 against a real driver: a NULL in a non-nullable column names the column
/// and the row rather than defaulting.
#[test]
fn a_null_in_a_non_nullable_column_is_a_named_error() {
    let c = db();
    c.execute("UPDATE emp SET name = NULL WHERE id = 3", [])
        .unwrap();
    let db = Sqlite::new(&c);
    let e = db.fetch::<Emp>(&query::<Emp>().to_sql()).unwrap_err();
    assert!(
        matches!(
            &e,
            Error::Row(RowError::UnexpectedNull {
                column: "name",
                row: Some(_)
            })
        ),
        "got {e}"
    );
    assert!(e.to_string().contains("column `name` is NULL"), "{e}");
}

/// D-029's bool rule against a real driver. SQLite stores 2 happily; the SQL
/// comparison keeps only 1, so materializing 2 as `true` would make the two
/// interpreters disagree. It errors instead.
#[test]
fn an_integer_that_is_not_zero_or_one_is_refused_not_coerced() {
    let c = db();
    c.execute("UPDATE emp SET active = 2 WHERE id = 3", [])
        .unwrap();
    let db = Sqlite::new(&c);

    // What the database itself thinks `active = true` means.
    let from_sql: Vec<i64> = db
        .fetch::<Emp>(&query::<Emp>().filter(emp::active.eq(true)).to_sql())
        .unwrap()
        .iter()
        .map(|e| e.id)
        .collect();
    assert_eq!(from_sql, [1], "SQLite compares against 1, so only Ada");

    // And row 3 cannot be materialized at all, rather than becoming `true`.
    let e = db.fetch::<Emp>(&query::<Emp>().to_sql()).unwrap_err();
    assert!(
        matches!(
            &e,
            Error::Row(RowError::OutOfRange {
                column: "active",
                ..
            })
        ),
        "got {e}"
    );
}

/// A schema that does not have what the entity declares fails before any row is
/// read, and says which column — the same verdict whether or not rows exist.
/// This is the drift gap: the crate cannot catch it at compile time, but it does
/// not guess either.
#[test]
fn a_missing_column_fails_identically_on_empty_and_populated_data() {
    let c = rusqlite::Connection::open_in_memory().unwrap();
    c.execute_batch("CREATE TABLE emp(id INTEGER, name TEXT); INSERT INTO emp VALUES(1,'x');")
        .unwrap();
    let db = Sqlite::new(&c);
    let populated = db
        .fetch::<Emp>(&linq_rs_sql::QueryOutput {
            sql: "SELECT id, name FROM emp".into(),
            params: vec![],
        })
        .unwrap_err()
        .to_string();
    let empty = db
        .fetch::<Emp>(&linq_rs_sql::QueryOutput {
            sql: "SELECT id, name FROM emp WHERE 1 = 0".into(),
            params: vec![],
        })
        .unwrap_err()
        .to_string();
    assert_eq!(populated, empty);
    assert!(populated.contains("no column `dept`"), "{populated}");
}

/// A join gives two `id` columns. Resolving to the first would make the other
/// table's data silently unreachable.
#[test]
fn a_join_with_duplicate_column_names_is_ambiguous() {
    let c = db();
    c.execute_batch(
        "CREATE TABLE d(id INTEGER, name TEXT); INSERT INTO d VALUES(9,'engineering');",
    )
    .unwrap();
    let db = Sqlite::new(&c);
    let e = db
        .fetch::<Emp>(&linq_rs_sql::QueryOutput {
            sql: "SELECT * FROM emp JOIN d ON 1 = 1".into(),
            params: vec![],
        })
        .unwrap_err();
    assert!(
        matches!(
            &e,
            Error::Row(RowError::AmbiguousColumn { column: "id", .. })
        ),
        "got {e}"
    );
}

/// SQLite's own error surfaces intact rather than being flattened.
#[test]
fn a_sqlite_error_keeps_its_source() {
    let c = rusqlite::Connection::open_in_memory().unwrap();
    let db = Sqlite::new(&c);
    let e = db
        .fetch::<Emp>(&linq_rs_sql::QueryOutput {
            sql: "SELECT * FROM does_not_exist".into(),
            params: vec![],
        })
        .unwrap_err();
    assert!(matches!(e, Error::Sqlite(_)), "got {e}");
    assert!(std::error::Error::source(&e).is_some(), "source preserved");
}
