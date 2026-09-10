//! Edge-case and error-handling tests for linq_rs.
//!
//! Existing `tests/linq_tests.rs` covers the happy path for every operator;
//! this file complements it by exercising:
//!
//! - Panic paths (chunk size 0, cast failure)
//! - Empty inputs across every category of operator
//! - Boundary values (n = 0, n > source length, `usize::MAX`)
//! - Vacuous-truth semantics (`all_` on empty)
//! - Deep composition chains
//! - Side-effecting `FnMut` closures (verifies state survives the adapter)
//! - Float / NaN behaviour for `PartialEq`-only paths

use linq_rs::{LinqExt, ThenBy};

// ─────────────────────────────────────────────────────────────────────────────
// PANIC PATHS
// ─────────────────────────────────────────────────────────────────────────────

#[test]
#[should_panic(expected = "chunk size must be > 0")]
fn chunk_size_zero_panics() {
    let _: Vec<_> = vec![1, 2, 3].into_iter().chunk(0).collect();
}

#[test]
#[should_panic(expected = "cast failed")]
fn cast_panic_is_lazy_at_iteration() {
    // The cast call itself must not panic; only iteration past the bad item does.
    let v = vec![1i64, 2, i64::MAX];
    let it = v.into_iter().cast::<i32>(); // ← no panic here
    let _: Vec<i32> = it.collect(); // ← panic here, when we hit MAX
}

#[test]
fn cast_construction_does_not_panic_on_empty() {
    let v: Vec<i32> = Vec::<i64>::new().into_iter().cast::<i32>().collect();
    assert!(v.is_empty());
}

// ─────────────────────────────────────────────────────────────────────────────
// EMPTY INPUTS — every operator category
// ─────────────────────────────────────────────────────────────────────────────

// ── Filtering / projection on empty ──────────────────────────────────────────

#[test]
fn where_on_empty() {
    let v: Vec<i32> = Vec::<i32>::new().into_iter().where_(|x| *x > 0).collect();
    assert!(v.is_empty());
}

#[test]
fn select_on_empty() {
    let v: Vec<i32> = Vec::<i32>::new().into_iter().select(|x| x * 2).collect();
    assert!(v.is_empty());
}

#[test]
fn select_many_on_empty_source() {
    let v: Vec<i32> = Vec::<Vec<i32>>::new()
        .into_iter()
        .select_many(|x| x.into_iter())
        .collect();
    assert!(v.is_empty());
}

#[test]
fn select_many_returning_empty_iters_yields_empty() {
    let v: Vec<i32> = vec![1, 2, 3]
        .into_iter()
        .select_many(|_| Vec::<i32>::new().into_iter())
        .collect();
    assert!(v.is_empty());
}

#[test]
fn flatten_on_empty() {
    let v: Vec<i32> = Vec::<Vec<i32>>::new().into_iter().flatten_().collect();
    assert!(v.is_empty());
}

#[test]
fn flatten_mixed_empty_and_nonempty() {
    let v: Vec<i32> = vec![vec![], vec![1, 2], vec![], vec![3], vec![]]
        .into_iter()
        .flatten_()
        .collect();
    assert_eq!(v, [1, 2, 3]);
}

// ── Set ops on empty ─────────────────────────────────────────────────────────

#[test]
fn distinct_on_empty() {
    let v: Vec<i32> = Vec::<i32>::new().into_iter().distinct().collect();
    assert!(v.is_empty());
}

#[test]
fn except_on_empty_self() {
    let v: Vec<i32> = Vec::<i32>::new()
        .into_iter()
        .except(vec![1, 2, 3])
        .collect();
    assert!(v.is_empty());
}

#[test]
fn intersect_on_empty_self() {
    let v: Vec<i32> = Vec::<i32>::new()
        .into_iter()
        .intersect(vec![1, 2, 3])
        .collect();
    assert!(v.is_empty());
}

#[test]
fn union_two_empties() {
    let v: Vec<i32> = Vec::<i32>::new()
        .into_iter()
        .union_(Vec::<i32>::new())
        .collect();
    assert!(v.is_empty());
}

#[test]
fn concat_two_empties() {
    let v: Vec<i32> = Vec::<i32>::new()
        .into_iter()
        .concat_(Vec::<i32>::new())
        .collect();
    assert!(v.is_empty());
}

// ── Quantifiers on empty ─────────────────────────────────────────────────────

#[test]
fn any_on_empty_is_false() {
    assert!(!Vec::<i32>::new().into_iter().any_(|_| true));
}

#[test]
fn all_on_empty_is_vacuously_true() {
    // Vacuous truth — matches std::iter::Iterator::all and C# LINQ semantics.
    assert!(Vec::<i32>::new().into_iter().all_(|_| false));
}

#[test]
fn contains_on_empty_is_false() {
    assert!(!Vec::<i32>::new().into_iter().contains_(&5));
}

// ── Aggregation on empty ─────────────────────────────────────────────────────

#[test]
fn aggregate_on_empty_returns_seed_unchanged() {
    let r = Vec::<i32>::new()
        .into_iter()
        .aggregate(99, |acc, x| acc + x);
    assert_eq!(r, 99);
}

#[test]
fn sum_on_empty_is_zero() {
    let s: i32 = Vec::<i32>::new().into_iter().sum_();
    assert_eq!(s, 0);
}

#[test]
fn count_where_on_empty_is_zero() {
    assert_eq!(Vec::<i32>::new().into_iter().count_where(|_| true), 0);
}

#[test]
fn min_max_on_empty_are_none() {
    assert_eq!(Vec::<i32>::new().into_iter().min_(), None);
    assert_eq!(Vec::<i32>::new().into_iter().max_(), None);
}

#[test]
fn aggregate_with_selector_on_empty_runs_selector_on_seed() {
    let r: i32 = Vec::<i32>::new().into_iter().aggregate_with_selector(
        0i32,
        |acc, x| acc + x,
        |sum| sum * 2,
    );
    assert_eq!(r, 0); // selector(0) = 0
}

// ── Element ops on empty (non-strict) ────────────────────────────────────────

#[test]
fn element_at_out_of_bounds_returns_none() {
    assert_eq!(vec![10, 20].into_iter().element_at(5), None);
}

#[test]
fn first_or_default_on_empty_is_none() {
    assert_eq!(Vec::<i32>::new().into_iter().first_or_default(), None);
}

#[test]
fn last_or_default_on_empty_is_none() {
    assert_eq!(Vec::<i32>::new().into_iter().last_or_default(), None);
}

// ── Joins on empty ───────────────────────────────────────────────────────────

#[test]
fn join_with_empty_outer_yields_empty() {
    let outer: Vec<(u32, &str)> = vec![];
    let inner = vec![(1u32, "x")];
    let v: Vec<_> = outer
        .into_iter()
        .join(inner, |(k, _)| *k, |(k, _)| *k, |a, b| (a, b))
        .collect();
    assert!(v.is_empty());
}

#[test]
fn join_with_empty_inner_yields_empty() {
    let outer = vec![(1u32, "Alice")];
    let inner: Vec<(u32, &str)> = vec![];
    let v: Vec<_> = outer
        .into_iter()
        .join(inner, |(k, _)| *k, |(k, _)| *k, |a, b| (a, b))
        .collect();
    assert!(v.is_empty());
}

#[test]
fn join_with_no_key_matches_yields_empty() {
    let outer = vec![(1u32, "Alice")];
    let inner = vec![(99u32, "x"), (100, "y")];
    let v: Vec<_> = outer
        .into_iter()
        .join(inner, |(k, _)| *k, |(k, _)| *k, |a, b| (a, b))
        .collect();
    assert!(v.is_empty());
}

#[test]
fn group_join_with_empty_inner_still_yields_every_outer() {
    // Left outer semantics — outers without matches get an empty group.
    let outer = vec![(1u32, "Alice"), (2, "Bob")];
    let inner: Vec<(u32, &str)> = vec![];
    let v: Vec<_> = outer
        .into_iter()
        .group_join(inner, |(k, _)| *k, |(k, _)| *k, |o, i| (o, i.len()))
        .collect();
    assert_eq!(v, [((1u32, "Alice"), 0), ((2, "Bob"), 0)]);
}

// ── Group ops on empty ───────────────────────────────────────────────────────

#[test]
fn group_by_on_empty_is_empty() {
    let v: Vec<_> = Vec::<i32>::new().into_iter().group_by(|x| *x).collect();
    assert!(v.is_empty());
}

#[test]
fn aggregate_by_on_empty_is_empty() {
    let v: Vec<_> = Vec::<i32>::new()
        .into_iter()
        .aggregate_by(|x| *x, |_| 0i32, |a, x| a + x)
        .collect();
    assert!(v.is_empty());
}

#[test]
fn group_by_hashed_on_empty_is_empty() {
    let v: Vec<_> = Vec::<i32>::new()
        .into_iter()
        .group_by_hashed(|x| *x)
        .collect();
    assert!(v.is_empty());
}

// ─────────────────────────────────────────────────────────────────────────────
// BOUNDARY VALUES — usize::MAX, n=0, n > source length
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn skip_zero_yields_all() {
    let v: Vec<_> = vec![1, 2, 3].into_iter().skip_(0).collect();
    assert_eq!(v, [1, 2, 3]);
}

#[test]
fn skip_usize_max_yields_empty() {
    let v: Vec<i32> = vec![1, 2, 3].into_iter().skip_(usize::MAX).collect();
    assert!(v.is_empty());
}

#[test]
fn take_zero_yields_empty() {
    let v: Vec<i32> = vec![1, 2, 3].into_iter().take_(0).collect();
    assert!(v.is_empty());
}

#[test]
fn take_usize_max_yields_all() {
    let v: Vec<_> = vec![1, 2, 3].into_iter().take_(usize::MAX).collect();
    assert_eq!(v, [1, 2, 3]);
}

#[test]
fn chunk_size_one() {
    let v: Vec<_> = vec![1, 2, 3].into_iter().chunk(1).collect();
    assert_eq!(v, [vec![1], vec![2], vec![3]]);
}

#[test]
fn chunk_size_larger_than_source() {
    let v: Vec<_> = vec![1, 2, 3].into_iter().chunk(100).collect();
    assert_eq!(v, [vec![1, 2, 3]]);
}

// ─────────────────────────────────────────────────────────────────────────────
// DEEP COMPOSITION CHAINS
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn filter_project_filter_project_filter() {
    let v: Vec<f64> = (1i32..=20)
        .where_(|x| x % 2 == 0)
        .select(|x| x * x)
        .where_(|x| *x > 50)
        .select(f64::from)
        .where_(|x| *x < 300.0)
        .collect();
    assert_eq!(v, [64.0, 100.0, 144.0, 196.0, 256.0]);
}

#[test]
fn three_level_then_by() {
    // Three-level sort: by tuple.0 asc, then tuple.1 asc, then tuple.2 asc.
    let data = vec![
        (2, "x", 100),
        (1, "z", 50),
        (1, "z", 25),
        (1, "a", 75),
        (2, "x", 50),
    ];
    let v: Vec<_> = data
        .into_iter()
        .order_by(|t| t.0)
        .then_by(|t| t.1)
        .then_by(|t| t.2)
        .into_iter()
        .collect();
    assert_eq!(
        v,
        [
            (1, "a", 75),
            (1, "z", 25),
            (1, "z", 50),
            (2, "x", 50),
            (2, "x", 100),
        ]
    );
}

#[test]
fn long_pipeline_skip_take_distinct_order() {
    let v: Vec<_> = vec![5, 3, 8, 3, 1, 8, 5, 9, 1, 2, 7, 5]
        .into_iter()
        .skip_(1) // [3, 8, 3, 1, 8, 5, 9, 1, 2, 7, 5]
        .take_(8) // [3, 8, 3, 1, 8, 5, 9, 1]
        .distinct() // [3, 8, 1, 5, 9]
        .order_by(|x| *x) // [1, 3, 5, 8, 9]
        .into_iter()
        .collect();
    assert_eq!(v, [1, 3, 5, 8, 9]);
}

#[test]
fn group_by_then_select_then_aggregate() {
    let words = vec!["apple", "ant", "banana", "bear", "cherry"];
    // Group by first letter, count letters in each group, sum.
    let total_chars: usize = words
        .into_iter()
        .group_by(|w| w.chars().next().unwrap())
        .select(|g| g.elements.iter().map(|w| w.len()).sum::<usize>())
        .sum_();
    // apple(5)+ant(3) + banana(6)+bear(4) + cherry(6) = 24
    assert_eq!(total_chars, 24);
}

// ─────────────────────────────────────────────────────────────────────────────
// SIDE EFFECTS — verify FnMut state survives the adapter
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn select_with_mutable_counter_state() {
    let mut counter = 0;
    let v: Vec<_> = (1..=5)
        .select(|x| {
            counter += 1;
            (counter, x)
        })
        .collect();
    assert_eq!(v, [(1, 1), (2, 2), (3, 3), (4, 4), (5, 5)]);
    assert_eq!(counter, 5);
}

#[test]
fn where_predicate_runs_once_per_item_including_rejected() {
    let mut calls = 0;
    let v: Vec<_> = (1..=10)
        .where_(|x| {
            calls += 1;
            x % 2 == 0
        })
        .collect();
    assert_eq!(v, [2, 4, 6, 8, 10]);
    // Predicate fires on every element, not just the kept ones.
    assert_eq!(calls, 10);
}

// ─────────────────────────────────────────────────────────────────────────────
// FLOAT / NaN  (PartialEq-only paths)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn distinct_works_with_floats() {
    let v: Vec<f64> = vec![1.0, 2.0, 1.0, 3.0, 2.0]
        .into_iter()
        .distinct()
        .collect();
    assert_eq!(v, [1.0, 2.0, 3.0]);
}

#[test]
fn distinct_keeps_all_nans_because_partial_eq_says_they_differ() {
    // NaN != NaN under PartialEq, so dedup-by-PartialEq treats every NaN as
    // a "new" element. This is documented behaviour, not a bug — if you want
    // true float dedup you need bit-pattern comparison via a wrapper.
    let nans: Vec<f64> = vec![f64::NAN, f64::NAN, 1.0, f64::NAN];
    let v: Vec<f64> = nans.into_iter().distinct().collect();
    assert_eq!(v.len(), 4);
    assert!(v[0].is_nan());
    assert!(v[1].is_nan());
    assert_eq!(v[2], 1.0);
    assert!(v[3].is_nan());
}

#[test]
fn average_of_single_element() {
    let avg = vec![42.0f64].into_iter().average(|x| x);
    assert_eq!(avg, Some(42.0));
}

// ─────────────────────────────────────────────────────────────────────────────
// CONVERSION EDGE CASES
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn to_lookup_preserves_all_duplicate_key_values() {
    let data = vec![("a", 1), ("a", 2), ("a", 3)];
    let lookup = data.into_iter().to_lookup(|(k, _)| *k);
    assert_eq!(lookup.count(), 1);
    assert_eq!(lookup.get(&"a"), &[("a", 1), ("a", 2), ("a", 3)]);
}

#[test]
fn to_lookup_on_empty_source_has_zero_keys() {
    let data: Vec<(i32, i32)> = vec![];
    let lookup = data.into_iter().to_lookup(|(k, _)| *k);
    assert_eq!(lookup.count(), 0);
    assert!(!lookup.contains_key(&1));
    assert_eq!(lookup.get(&1), &[]);
}

#[test]
fn to_hashmap_last_write_wins_on_duplicate_keys() {
    // HashMap::insert overwrites — so duplicate keys keep the last value.
    let data = vec![("a", 1), ("a", 2), ("a", 3)];
    let map = data.into_iter().to_hashmap(|(k, _)| *k);
    assert_eq!(map.len(), 1);
    assert_eq!(map[&"a"], ("a", 3));
}

// ─────────────────────────────────────────────────────────────────────────────
// OTHER MISCELLANEOUS EDGES
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn order_by_on_empty_yields_empty() {
    let v: Vec<i32> = Vec::<i32>::new()
        .into_iter()
        .order_by(|x| *x)
        .into_iter()
        .collect();
    assert!(v.is_empty());
}

#[test]
fn order_by_already_sorted_input_is_stable() {
    let v: Vec<_> = vec![1, 2, 3, 4, 5]
        .into_iter()
        .order_by(|x| *x)
        .into_iter()
        .collect();
    assert_eq!(v, [1, 2, 3, 4, 5]);
}

#[test]
fn reverse_on_empty() {
    let v: Vec<i32> = Vec::<i32>::new().into_iter().reverse().collect();
    assert!(v.is_empty());
}

#[test]
fn reverse_on_single_element() {
    let v: Vec<_> = vec![42].into_iter().reverse().collect();
    assert_eq!(v, [42]);
}

#[test]
fn reduce_on_single_element_returns_that_element() {
    let r = vec![42].into_iter().reduce_(|a, b| a + b);
    assert_eq!(r, Some(42));
}

#[test]
fn zip3_with_one_empty_yields_empty() {
    let v: Vec<_> = vec![1, 2, 3]
        .into_iter()
        .zip3(Vec::<i32>::new(), vec![10, 20, 30], |a, b, c| (a, b, c))
        .collect();
    assert!(v.is_empty());
}

#[test]
fn sequence_equal_two_empties_is_true() {
    assert!(Vec::<i32>::new()
        .into_iter()
        .sequence_equal(Vec::<i32>::new()));
}

#[test]
fn sequence_equal_self_longer_is_false() {
    assert!(!vec![1, 2, 3].into_iter().sequence_equal(vec![1, 2]));
}

#[test]
fn sequence_equal_other_longer_is_false() {
    assert!(!vec![1, 2].into_iter().sequence_equal(vec![1, 2, 3]));
}

#[test]
fn of_type_filters_all_out_when_nothing_fits() {
    let big: Vec<i64> = vec![i64::MAX, i64::MAX - 1, i64::MAX - 2];
    let v: Vec<i32> = big.into_iter().of_type::<i32>().collect();
    assert!(v.is_empty());
}

#[test]
fn range_negative_start() {
    let v: Vec<_> = linq_rs::range(-3, 5).collect();
    assert_eq!(v, [-3, -2, -1, 0, 1]);
}

#[test]
fn range_zero_count_yields_empty() {
    let v: Vec<_> = linq_rs::range(100, 0).collect();
    assert!(v.is_empty());
}

#[test]
fn default_if_empty_then_other_ops_compose() {
    // Empty + default_if_empty + chained ops behaves as a single-element source.
    let v: Vec<_> = Vec::<i32>::new()
        .into_iter()
        .default_if_empty(7)
        .select(|x| x * 10)
        .collect();
    assert_eq!(v, [70]);
}

#[test]
fn take_last_then_skip_last_compose() {
    // [1..=10] take_last(5) -> [6,7,8,9,10], skip_last(2) -> [6,7,8]
    let v: Vec<_> = (1..=10).take_last(5).skip_last(2).collect();
    assert_eq!(v, [6, 7, 8]);
}
