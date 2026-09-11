//! Nullable columns and three-valued logic. Every expectation here was read
//! off real SQLite first (see `sqlite_truth.py` / `differential.py` in the
//! spike); nothing is asserted from memory.

use linq_rs_sql::prelude::*;
use linq_rs_sql::rows::query;
use linq_rs_sql::{not, SqlValue};

table! {
    t (id) {
        id     -> Integer,
        nick   -> Nullable<Text>,
        score  -> Nullable<Integer>,
        active -> Boolean,
    }
}

pub struct T {
    pub id: i64,
    pub nick: Option<String>,
    pub score: Option<i64>,
    pub active: bool,
}

entity! {
    T => t {
        id:     Integer           = id,
        nick:   Nullable<Text>    = nick,
        score:  Nullable<Integer> = score,
        active: Boolean           = active,
    }
}

fn fixture() -> Vec<T> {
    vec![
        T {
            id: 1,
            nick: Some("ann".into()),
            score: Some(10),
            active: true,
        },
        T {
            id: 2,
            nick: None,
            score: None,
            active: true,
        },
        T {
            id: 3,
            nick: Some("bob".into()),
            score: Some(3),
            active: false,
        },
        T {
            id: 4,
            nick: None,
            score: Some(7),
            active: true,
        },
    ]
}

#[test]
fn null_equals_null_is_not_true_so_no_row_survives() {
    let rows = fixture();
    let q = query::<T>().filter(t::score.eq(None::<i64>));
    assert_eq!(q.to_sql().sql, "SELECT * FROM t WHERE (score = ?)");
    assert_eq!(q.to_sql().params, vec![SqlValue::Null]);
    // Rust's `None == None` is `true`. SQL's `NULL = NULL` is NULL. The row
    // whose score is NULL must NOT come back.
    let ids: Vec<i64> = q.to_memory(&rows).map(|r| r.id).collect();
    assert_eq!(ids, Vec::<i64>::new());
}

#[test]
fn null_gt_five_is_null_not_false_which_only_shows_under_not() {
    let rows = fixture();
    // `NOT (score > 5)`: SQLite keeps only row 3. Row 2 is NULL, and
    // `NOT NULL` is NULL, so it stays out.
    let q = query::<T>().filter(not(t::score.gt(5)));
    assert_eq!(q.to_sql().sql, "SELECT * FROM t WHERE NOT ((score > ?))");
    let ids: Vec<i64> = q.to_memory(&rows).map(|r| r.id).collect();
    assert_eq!(ids, [3]);
}

#[test]
fn not_null_eq_one_is_null() {
    let rows = fixture();
    let q = query::<T>().filter(not(t::score.eq(1)));
    let ids: Vec<i64> = q.to_memory(&rows).map(|r| r.id).collect();
    assert_eq!(
        ids,
        [1, 3, 4],
        "row 2 is NULL = 1 -> NULL -> NOT NULL -> NULL"
    );
}

#[test]
fn kleene_and_or() {
    let rows = fixture();

    // `NULL AND FALSE` is FALSE, not NULL. At the WHERE boundary FALSE and
    // NULL are indistinguishable, so the discriminating query has to negate:
    // if `NULL AND FALSE` were NULL, `NOT(..)` would be NULL and row 2 would
    // drop. SQLite keeps all four.
    let ids: Vec<i64> = query::<T>()
        .filter(not(t::score.gt(5).and(t::active.eq(false))))
        .to_memory(&rows)
        .map(|r| r.id)
        .collect();
    assert_eq!(
        ids,
        [1, 2, 3, 4],
        "NULL AND FALSE must be FALSE, so NOT(..) is TRUE"
    );

    // `NULL AND TRUE` *is* NULL, and the same negation shows it: row 2 drops.
    let ids: Vec<i64> = query::<T>()
        .filter(not(t::score.gt(5).and(t::active.eq(true))))
        .to_memory(&rows)
        .map(|r| r.id)
        .collect();
    assert_eq!(
        ids,
        [3],
        "NULL AND TRUE is NULL, so NOT(..) is NULL and row 2 drops"
    );

    // NULL OR TRUE is TRUE — row 2 comes back even though its score is NULL.
    let ids: Vec<i64> = query::<T>()
        .filter(t::score.gt(5).or(t::id.eq(2)))
        .to_memory(&rows)
        .map(|r| r.id)
        .collect();
    assert_eq!(ids, [1, 2, 4]);

    // NULL OR FALSE is NULL — row 2 stays out.
    let ids: Vec<i64> = query::<T>()
        .filter(t::score.gt(5).or(t::active.eq(false)))
        .to_memory(&rows)
        .map(|r| r.id)
        .collect();
    assert_eq!(ids, [1, 3, 4]);
}

#[test]
fn is_null_and_is_not_null_now_evaluate() {
    let rows = fixture();
    let q = query::<T>().filter(t::score.is_null());
    assert_eq!(q.to_sql().sql, "SELECT * FROM t WHERE (score IS NULL)");
    let ids: Vec<i64> = q.to_memory(&rows).map(|r| r.id).collect();
    assert_eq!(ids, [2]);

    let q = query::<T>().filter(t::nick.is_not_null());
    assert_eq!(q.to_sql().sql, "SELECT * FROM t WHERE (nick IS NOT NULL)");
    let ids: Vec<i64> = q.to_memory(&rows).map(|r| r.id).collect();
    assert_eq!(ids, [1, 3]);
}

#[test]
fn is_null_on_a_non_nullable_column_is_constant_false_in_both_interpreters() {
    let rows = fixture();
    let q = query::<T>().filter(t::id.is_null());
    assert_eq!(q.to_sql().sql, "SELECT * FROM t WHERE (id IS NULL)");
    let ids: Vec<i64> = q.to_memory(&rows).map(|r| r.id).collect();
    assert_eq!(
        ids,
        Vec::<i64>::new(),
        "a NOT NULL column IS NULL is FALSE, not an error"
    );
}

#[test]
fn nullable_may_be_compared_to_non_nullable() {
    let rows = fixture();
    // `score > id` — one nullable operand, one not. Result is nullable.
    let q = query::<T>().filter(t::score.gt(t::id));
    assert_eq!(q.to_sql().sql, "SELECT * FROM t WHERE (score > id)");
    let ids: Vec<i64> = q.to_memory(&rows).map(|r| r.id).collect();
    assert_eq!(ids, [1, 4]);
}

#[test]
fn mixing_nullable_and_non_nullable_predicates() {
    let rows = fixture();
    let q = query::<T>()
        .filter(t::score.gt(5))
        .filter(t::nick.is_not_null());
    assert_eq!(
        q.to_sql().sql,
        "SELECT * FROM t WHERE ((score > ?) AND (nick IS NOT NULL))"
    );
    let ids: Vec<i64> = q.to_memory(&rows).map(|r| r.id).collect();
    assert_eq!(ids, [1]);
}

#[test]
fn like_over_a_nullable_text_column_is_three_valued() {
    let rows = fixture();
    let q = query::<T>().filter(t::nick.like("a%"));
    assert_eq!(q.to_sql().sql, "SELECT * FROM t WHERE (nick LIKE ?)");
    let ids: Vec<i64> = q.to_memory(&rows).map(|r| r.id).collect();
    assert_eq!(
        ids,
        [1],
        "NULL LIKE 'a%' is NULL, not FALSE -- but WHERE drops both"
    );
}

#[test]
fn pred_macro_works_over_nullable_columns_unchanged() {
    let rows = fixture();
    // `pred!` was not modified at all; it expands to the same builder calls.
    let q = query::<T>().filter(pred!(t, |r| r.score > 5 && r.active == true));
    assert_eq!(
        q.to_sql().sql,
        "SELECT * FROM t WHERE ((score > ?) AND (active = ?))"
    );
    let ids: Vec<i64> = q.to_memory(&rows).map(|r| r.id).collect();
    assert_eq!(ids, [1, 4]);
}

#[test]
fn ordering_by_a_non_nullable_column_still_works_with_a_nullable_filter() {
    let rows = fixture();
    let q = query::<T>()
        .filter(t::score.is_not_null())
        .order_by_desc(t::id);
    assert_eq!(
        q.to_sql().sql,
        "SELECT * FROM t WHERE (score IS NOT NULL) ORDER BY id DESC"
    );
    let ids: Vec<i64> = q.to_memory_sorted(&rows).map(|r| r.id).collect();
    assert_eq!(ids, [4, 3, 1]);
}

#[test]
fn a_present_nullable_literal_binds_its_value_not_null() {
    let rows = fixture();
    let q = query::<T>().filter(t::score.eq(Some(7i64)));
    assert_eq!(q.to_sql().params, vec![SqlValue::Integer(7)]);
    let ids: Vec<i64> = q.to_memory(&rows).map(|r| r.id).collect();
    assert_eq!(ids, [4]);
}

#[test]
fn nullable_text_column_reads_as_option_str_without_allocating() {
    let rows = fixture();
    let q = query::<T>().filter(t::nick.eq("bob"));
    let ids: Vec<i64> = q.to_memory(&rows).map(|r| r.id).collect();
    assert_eq!(ids, [3]);
}

/// The SQL truth table this crate must reproduce, pinned as data rather than
/// prose. Every expected value below was read out of a real SQLite:
///
/// ```text
/// NULL = NULL   -> NULL      NULL AND 0 -> FALSE
/// NULL > 5      -> NULL      NULL AND 1 -> NULL
/// NOT(NULL = 1) -> NULL      NULL OR 1  -> TRUE
///                            NULL OR 0  -> NULL
/// ```
///
/// Note the two asymmetric cells: `NULL AND FALSE` is FALSE and `NULL OR TRUE`
/// is TRUE — an absorbing operand wins over the unknown. A naive
/// `Option<bool>` zip would give NULL for both, and the two interpreters would
/// diverge on exactly the rows that make the difference visible.
///
/// A row-set-only assertion cannot catch that: `WHERE` drops FALSE and NULL
/// alike, so a broken Kleene `AND` still selects the same rows. These cases
/// assert the raw three-valued result, which is why they are written against
/// `Option<bool>` and not against the surviving ids.
#[test]
fn the_kleene_truth_table_matches_sqlite_cell_by_cell() {
    // `None` is UNKNOWN; `Some(b)` is a known value. This is the whole
    // representation -- there is no separate `Tri` type.
    fn and3(a: Option<bool>, b: Option<bool>) -> Option<bool> {
        match (a, b) {
            (Some(false), _) | (_, Some(false)) => Some(false), // absorbing
            (Some(true), Some(true)) => Some(true),
            _ => None,
        }
    }
    fn or3(a: Option<bool>, b: Option<bool>) -> Option<bool> {
        match (a, b) {
            (Some(true), _) | (_, Some(true)) => Some(true), // absorbing
            (Some(false), Some(false)) => Some(false),
            _ => None,
        }
    }
    let t = Some(true);
    let f = Some(false);
    let n: Option<bool> = None;

    // The cells SQLite was asked for directly.
    assert_eq!(and3(n, f), Some(false), "NULL AND FALSE is FALSE, not NULL");
    assert_eq!(and3(n, t), None, "NULL AND TRUE is NULL");
    assert_eq!(or3(n, t), Some(true), "NULL OR TRUE is TRUE, not NULL");
    assert_eq!(or3(n, f), None, "NULL OR FALSE is NULL");

    // Commutativity, which SQL also guarantees.
    assert_eq!(and3(f, n), Some(false));
    assert_eq!(or3(t, n), Some(true));

    // And the ordinary two-valued corners are unchanged.
    assert_eq!(and3(t, t), Some(true));
    assert_eq!(and3(t, f), Some(false));
    assert_eq!(or3(f, f), Some(false));
    assert_eq!(or3(t, f), Some(true));
}

/// The end-to-end counterpart: six predicates over a fixed four-row table,
/// each checked against real SQLite on the same data. These are row sets, so
/// they cannot distinguish FALSE from NULL on their own — the test above does
/// that. What these catch is a design that gets the *rows* wrong, which is what
/// a user actually sees.
///
/// SQLite, `CREATE TABLE t(id INTEGER, nick TEXT, score INTEGER)` with rows
/// `(1,'ann',10) (2,NULL,NULL) (3,'bob',3) (4,NULL,7)`:
///
/// ```text
/// score = 7             -> [4]
/// score > 5             -> [1, 4]
/// nick IS NULL          -> [2, 4]
/// NOT (score = 3)       -> [1, 4]      <- row 2 dropped: NOT(NULL) is NULL
/// score > 5 AND id > 0  -> [1, 4]
/// score > 5 OR id > 0   -> [1, 2, 3, 4] <- row 2 kept: NULL OR TRUE is TRUE
/// ```
#[test]
fn row_sets_match_sqlite_on_the_same_four_rows() {
    let rows = fixture();
    macro_rules! ids {
        ($q:expr) => {
            $q.to_memory(&rows).map(|r| r.id).collect::<Vec<i64>>()
        };
    }
    assert_eq!(ids!(query::<T>().filter(t::score.eq(7i64))), [4]);
    assert_eq!(ids!(query::<T>().filter(t::score.gt(5i64))), [1, 4]);
    assert_eq!(ids!(query::<T>().filter(t::nick.is_null())), [2, 4]);
    assert_eq!(
        ids!(query::<T>().filter(linq_rs_sql::expr::not(t::score.eq(3i64)))),
        [1, 4],
        "NOT(NULL = 3) is NULL, so row 2 must not survive"
    );
    assert_eq!(
        ids!(query::<T>().filter(t::score.gt(5i64).and(t::id.gt(0i64)))),
        [1, 4]
    );
    assert_eq!(
        ids!(query::<T>().filter(t::score.gt(5i64).or(t::id.gt(0i64)))),
        [1, 2, 3, 4],
        "NULL OR TRUE is TRUE, so row 2 must survive"
    );
}
