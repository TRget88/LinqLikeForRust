//! Integration tests for linq_rs.
//!
//! Run with: `cargo test`

use linq_rs::LinqExt;

// ── where_ / select ──────────────────────────────────────────────────────────

#[test]
fn test_where_select() {
    let result: Vec<_> = (1..=10).where_(|x| x % 2 == 0).select(|x| x * x).collect();
    assert_eq!(result, [4, 16, 36, 64, 100]);
}

#[test]
fn test_where_chained_twice() {
    let result: Vec<_> = (1..=20)
        .where_(|x| x % 2 == 0)
        .where_(|x| x % 3 == 0)
        .collect();
    assert_eq!(result, [6, 12, 18]);
}

// ── select_many / flatten_ ───────────────────────────────────────────────────

#[test]
fn test_select_many() {
    let data = vec![vec![1, 2, 3], vec![4, 5], vec![6]];
    let flat: Vec<_> = data.into_iter().select_many(|v| v.into_iter()).collect();
    assert_eq!(flat, [1, 2, 3, 4, 5, 6]);
}

// ── skip / take ──────────────────────────────────────────────────────────────

#[test]
fn test_skip_take() {
    let result: Vec<_> = (1..=10).skip_(3).take_(4).collect();
    assert_eq!(result, [4, 5, 6, 7]);
}

// ── chunk ────────────────────────────────────────────────────────────────────

// ── distinct ─────────────────────────────────────────────────────────────────

#[test]
fn test_distinct() {
    let d: Vec<_> = vec![3, 1, 2, 1, 3, 4].into_iter().distinct().collect();
    assert_eq!(d, [3, 1, 2, 4]);
}

#[test]
fn test_distinct_by() {
    let words = vec!["apple", "ant", "banana", "bear"];
    let d: Vec<_> = words
        .into_iter()
        .distinct_by(|w| w.chars().next().unwrap())
        .collect();
    assert_eq!(d, ["apple", "banana"]);
}

// ── set operations ───────────────────────────────────────────────────────────

#[test]
fn test_except() {
    let result: Vec<_> = vec![1, 2, 3, 4, 5].into_iter().except(vec![2, 4]).collect();
    assert_eq!(result, [1, 3, 5]);
}

#[test]
fn test_intersect() {
    let result: Vec<_> = vec![1, 2, 3, 4]
        .into_iter()
        .intersect(vec![2, 4, 6])
        .collect();
    assert_eq!(result, [2, 4]);
}

#[test]
fn test_union() {
    let result: Vec<_> = vec![1, 2, 3].into_iter().union_(vec![2, 3, 4, 5]).collect();
    assert_eq!(result, [1, 2, 3, 4, 5]);
}

// ── ordering ─────────────────────────────────────────────────────────────────

#[test]
fn test_order_by() {
    let result: Vec<_> = vec![3, 1, 4, 1, 5, 9, 2]
        .into_iter()
        .order_by(|x| *x)
        .collect();
    assert_eq!(result, [1, 1, 2, 3, 4, 5, 9]);
}

#[test]
fn test_order_by_descending() {
    let result: Vec<_> = vec![3, 1, 4, 1, 5]
        .into_iter()
        .order_by_descending(|x| *x)
        .collect();
    assert_eq!(result, [5, 4, 3, 1, 1]);
}

#[test]
fn test_then_by() {
    let data = vec![("Bob", 2), ("Alice", 2), ("Charlie", 1)];
    let result: Vec<_> = data
        .into_iter()
        .order_by(|(_, age)| *age)
        .then_by(|(name, _)| *name)
        .collect();
    assert_eq!(result, [("Charlie", 1), ("Alice", 2), ("Bob", 2)]);
}

// ── aggregation ──────────────────────────────────────────────────────────────

#[test]
fn test_sum() {
    let s: i32 = (1..=100).sum_();
    assert_eq!(s, 5050);
}

#[test]
fn test_count_where() {
    let n = (1..=20).count_where(|x| x % 3 == 0);
    assert_eq!(n, 6);
}

#[test]
fn test_min_max() {
    let data = [5, 1, 8, 2, 9, 3];
    assert_eq!(data.iter().copied().min_(), Some(1));
    assert_eq!(data.iter().copied().max_(), Some(9));
}

#[test]
fn test_average() {
    let avg = vec![1.0f64, 2.0, 3.0, 4.0, 5.0].into_iter().average(|x| x);
    assert_eq!(avg, Some(3.0));
}

#[test]
fn test_average_empty() {
    let avg = std::iter::empty::<f64>().average(|x| x);
    assert_eq!(avg, None);
}

// ── element operations ────────────────────────────────────────────────────────

#[test]
fn test_first_or_default() {
    assert_eq!((1..=5).first_or_default(), Some(1));
    assert_eq!(std::iter::empty::<i32>().first_or_default(), None);
}

#[test]
fn test_first_where() {
    let v = (1..=10).first_where(|x| *x > 5);
    assert_eq!(v, Some(6));
}

#[test]
fn test_last_or_default() {
    assert_eq!((1..=5).last_or_default(), Some(5));
}

#[test]
fn test_single_or_default_distinguishes_empty_from_too_many() {
    use linq_rs::SingleError;
    // The whole point of W-16: these two used to be indistinguishable.
    assert_eq!(vec![42].into_iter().single_or_default(), Ok(Some(42)));
    assert_eq!(Vec::<i32>::new().into_iter().single_or_default(), Ok(None));
    assert_eq!(
        vec![1, 2].into_iter().single_or_default(),
        Err(SingleError::MoreThanOne)
    );
    // The documented one-liner back to the old collapsing behaviour.
    let collapse = |v: Vec<i32>| v.into_iter().single_or_default().ok().flatten();
    assert_eq!(collapse(vec![42]), Some(42));
    assert_eq!(collapse(vec![]), None);
    assert_eq!(collapse(vec![1, 2]), None);
}

#[test]
fn test_try_single() {
    use linq_rs::SingleError;
    assert_eq!(vec![7].into_iter().try_single(), Ok(7));
    assert_eq!(
        Vec::<i32>::new().into_iter().try_single(),
        Err(SingleError::Empty)
    );
    assert_eq!(
        vec![1, 2].into_iter().try_single(),
        Err(SingleError::MoreThanOne)
    );
    // Consumes at most two elements, so it terminates on an infinite source.
    assert_eq!((1i32..).try_single(), Err(SingleError::MoreThanOne));
}

#[test]
fn test_single_family_shares_one_panic_message() {
    use linq_rs::SingleError;
    // `single` and `single_or` panic with text generated from SingleError, so
    // the doc section, the panic and Display cannot drift apart.
    let e = std::panic::catch_unwind(|| vec![1, 2].into_iter().single()).unwrap_err();
    let msg = e.downcast_ref::<String>().map(String::as_str).unwrap_or("");
    assert_eq!(msg, SingleError::MoreThanOne.message());
    assert_eq!(msg, SingleError::MoreThanOne.to_string());
}

// ── quantifiers ───────────────────────────────────────────────────────────────

#[test]
fn test_any_all() {
    assert!((1..=10).any_(|x| x > 5));
    assert!(!(1..=10).any_(|x| x > 100));
    assert!((1..=10).all_(|x| x > 0));
    assert!(!(1..=10).all_(|x| x > 5));
}

#[test]
fn test_contains() {
    assert!(vec![1, 2, 3].into_iter().contains_(&2));
    assert!(!vec![1, 2, 3].into_iter().contains_(&5));
}

// ── inner_join ─────────────────────────────────────────────────────────────────────

#[test]
fn test_inner_join() {
    let customers = vec![(1u32, "Alice"), (2, "Bob"), (3, "Carol")];
    let orders = vec![(1u32, "Laptop"), (1, "Mouse"), (2, "Keyboard")];

    let mut results: Vec<String> = customers
        .into_iter()
        .inner_join(
            orders,
            |(id, _)| *id,
            |(id, _)| *id,
            |(_, name), (_, product)| format!("{name} bought {product}"),
        )
        .collect();
    results.sort();

    assert_eq!(
        results,
        [
            "Alice bought Laptop",
            "Alice bought Mouse",
            "Bob bought Keyboard"
        ]
    );
}

#[test]
fn test_group_join() {
    let departments = vec![(1u32, "Engineering"), (2u32, "Sales")];
    let employees = vec![(1u32, "Alice"), (1, "Bob"), (2, "Carol")];

    let result: Vec<_> = departments
        .into_iter()
        .group_join(
            employees,
            |(id, _)| *id,
            |(dept_id, _)| *dept_id,
            |(_, dept), emps| {
                let names: Vec<_> = emps.into_iter().map(|(_, n)| n).collect();
                (dept, names)
            },
        )
        .collect();

    assert_eq!(result[0], ("Engineering", vec!["Alice", "Bob"]));
    assert_eq!(result[1], ("Sales", vec!["Carol"]));
}

// ── group_by_key ──────────────────────────────────────────────────────────────────

#[test]
fn test_group_by() {
    let words = vec!["apple", "ant", "banana", "bear", "cherry"];
    let mut groups: Vec<_> = words
        .into_iter()
        .group_by_key(|w| w.chars().next().unwrap())
        .collect();
    groups.sort_by_key(|g| *g.key());

    assert_eq!(*groups[0].key(), 'a');
    assert_eq!(groups[0].elements(), ["apple", "ant"]);
    assert_eq!(*groups[1].key(), 'b');
    assert_eq!(groups[1].elements(), ["banana", "bear"]);
    assert_eq!(*groups[2].key(), 'c');
    assert_eq!(groups[2].elements(), ["cherry"]);
}

// ── into_lookup ─────────────────────────────────────────────────────────────────

#[test]
fn test_to_lookup() {
    let data = vec![("a", 1), ("b", 2), ("a", 3), ("c", 4), ("b", 5)];
    let lookup = data.into_iter().into_lookup(|(k, _)| *k);

    assert_eq!(lookup.get(&"a"), &[("a", 1), ("a", 3)]);
    assert_eq!(lookup.get(&"b"), &[("b", 2), ("b", 5)]);
    assert_eq!(lookup.get(&"c"), &[("c", 4)]);
    assert_eq!(lookup.get(&"z"), &[]);
    assert!(lookup.contains_key(&"a"));
    assert!(!lookup.contains_key(&"z"));
    assert_eq!(lookup.len(), 3);
}

// ── into_hashmap / to_hashset ───────────────────────────────────────────────────

#[test]
fn test_to_hashmap() {
    let map = vec![("one", 1), ("two", 2), ("three", 3)]
        .into_iter()
        .into_hashmap(|(k, _)| *k);
    assert_eq!(map[&"one"], ("one", 1));
    assert_eq!(map[&"two"], ("two", 2));
}

// ── utility ───────────────────────────────────────────────────────────────────

// ── default_if_empty ─────────────────────────────────────────────────────────

// ── take_last / skip_last ────────────────────────────────────────────────────

// ── order / order_descending ─────────────────────────────────────────────────

#[test]
fn test_order() {
    let v: Vec<_> = vec![3, 1, 4, 1, 5, 9, 2].into_iter().order().collect();
    assert_eq!(v, [1, 1, 2, 3, 4, 5, 9]);
}

#[test]
fn test_order_descending() {
    let v: Vec<_> = vec![3, 1, 4, 1, 5].into_iter().order_descending().collect();
    assert_eq!(v, [5, 4, 3, 1, 1]);
}

#[test]
fn test_order_then_by() {
    // After order(), chaining .then_by(...) should still compile and run.
    let data = vec![(1, "b"), (1, "a"), (2, "z")];
    let v: Vec<_> = data
        .into_iter()
        .order() // lexicographic on (i32, &str): (1,"a"), (1,"b"), (2,"z")
        .then_by(|(_, name)| *name) // stable resort by name only
        .collect();
    assert_eq!(v, [(1, "a"), (1, "b"), (2, "z")]);
}

#[test]
fn test_order_empty() {
    let v: Vec<i32> = Vec::<i32>::new().into_iter().order().collect();
    assert!(v.is_empty());
}

// ── first / last_ / single / element_at_strict (panicking) ───────────────────

#[test]
fn test_first_strict() {
    assert_eq!((1..=5).first(), 1);
}

#[test]
#[should_panic(expected = "sequence contains no elements")]
fn test_first_strict_empty_panics() {
    let _ = std::iter::empty::<i32>().first();
}

#[test]
fn test_last_strict() {
    assert_eq!((1..=5).last_(), 5);
}

#[test]
#[should_panic(expected = "sequence contains no elements")]
fn test_last_strict_empty_panics() {
    let _ = std::iter::empty::<i32>().last_();
}

#[test]
fn test_single_strict() {
    assert_eq!(vec![42].into_iter().single(), 42);
}

#[test]
#[should_panic(expected = "sequence contains no elements")]
fn test_single_empty_panics() {
    let _ = Vec::<i32>::new().into_iter().single();
}

#[test]
#[should_panic(expected = "sequence contains more than one element")]
fn test_single_multiple_panics() {
    let _ = vec![1, 2].into_iter().single();
}

// ── reduce_ ──────────────────────────────────────────────────────────────────

// ── aggregate_with_selector ──────────────────────────────────────────────────

// ── sum_by ───────────────────────────────────────────────────────────────────

#[test]
fn test_sum_by() {
    let pairs = vec![("a", 1), ("b", 2), ("c", 3)];
    let total: i32 = pairs.into_iter().sum_by(|(_, n)| n);
    assert_eq!(total, 6);
}

#[test]
fn test_sum_by_empty() {
    let total: i32 = Vec::<(i32, i32)>::new().into_iter().sum_by(|(_, n)| n);
    assert_eq!(total, 0);
}

#[test]
fn test_sum_by_float() {
    let total: f64 = vec![1.5f64, 2.5, 3.0].into_iter().sum_by(|x| x);
    assert_eq!(total, 7.0);
}

// ── except_by / intersect_by / union_by ──────────────────────────────────────

#[test]
fn test_except_by() {
    let items = vec!["apple", "ant", "banana", "bear"];
    let v: Vec<_> = items
        .into_iter()
        .except_by(vec!['a'], |s| s.chars().next().unwrap())
        .collect();
    assert_eq!(v, ["banana", "bear"]);
}

#[test]
fn test_except_by_no_exclusions() {
    let items = vec![1, 2, 3];
    let v: Vec<_> = items
        .into_iter()
        .except_by(Vec::<i32>::new(), |x| *x)
        .collect();
    assert_eq!(v, [1, 2, 3]);
}

#[test]
fn test_intersect_by() {
    let items = vec!["apple", "ant", "banana", "bear", "cherry"];
    let v: Vec<_> = items
        .into_iter()
        .intersect_by(vec!['a', 'c'], |s| s.chars().next().unwrap())
        .collect();
    assert_eq!(v, ["apple", "ant", "cherry"]);
}

#[test]
fn test_intersect_by_no_match() {
    let items = vec![1, 2, 3];
    let v: Vec<_> = items
        .into_iter()
        .intersect_by(vec![10, 20], |x| *x)
        .collect();
    assert!(v.is_empty());
}

#[test]
fn test_union_by_cross_side_dedup() {
    let first = vec!["apple", "ant"];
    let second = vec!["avocado", "banana"];
    let v: Vec<_> = first
        .into_iter()
        .union_by(second, |s| s.chars().next().unwrap())
        .collect();
    assert_eq!(v, ["apple", "banana"]);
}

#[test]
fn test_union_by_no_overlap() {
    let first = vec![("a", 1)];
    let second = vec![("b", 2), ("c", 3)];
    let v: Vec<_> = first.into_iter().union_by(second, |(k, _)| *k).collect();
    assert_eq!(v, [("a", 1), ("b", 2), ("c", 3)]);
}

// ── source generators: range / repeat / empty ────────────────────────────────

#[test]
fn test_range_basic() {
    let v: Vec<_> = linq_rs::range(1, 5).collect();
    assert_eq!(v, [1, 2, 3, 4, 5]);
}

#[test]
fn test_range_zero_count() {
    let v: Vec<i32> = linq_rs::range(0, 0).collect();
    assert!(v.is_empty());
}

#[test]
fn test_range_negative_start() {
    let v: Vec<_> = linq_rs::range(-2, 5).collect();
    assert_eq!(v, [-2, -1, 0, 1, 2]);
}

#[test]
fn test_range_chains_with_linq_ext() {
    let v: Vec<_> = linq_rs::range(1, 10)
        .where_(|x| x % 2 == 0)
        .select(|x| x * x)
        .collect();
    assert_eq!(v, [4, 16, 36, 64, 100]);
}

#[test]
fn test_repeat_basic() {
    let v: Vec<_> = linq_rs::repeat("hi", 3).collect();
    assert_eq!(v, ["hi", "hi", "hi"]);
}

#[test]
fn test_repeat_zero() {
    let v: Vec<i32> = linq_rs::repeat(42, 0).collect();
    assert!(v.is_empty());
}

#[test]
fn test_empty_source() {
    let v: Vec<i32> = linq_rs::empty().collect();
    assert!(v.is_empty());
}

// ── of_type / cast ───────────────────────────────────────────────────────────

// ── indexed variants (Phase 2.1) ─────────────────────────────────────────────

// ── *_or(default) variants (Phase 2.5) ───────────────────────────────────────

#[test]
fn test_first_or() {
    assert_eq!((1..=5).first_or(99), 1);
    assert_eq!(std::iter::empty::<i32>().first_or(99), 99);
}

#[test]
fn test_last_or() {
    assert_eq!((1..=5).last_or(99), 5);
    assert_eq!(std::iter::empty::<i32>().last_or(99), 99);
}

#[test]
fn test_single_or_empty() {
    assert_eq!(Vec::<i32>::new().into_iter().single_or(99), 99);
}

#[test]
fn test_single_or_one_element() {
    assert_eq!(vec![42].into_iter().single_or(99), 42);
}

#[test]
#[should_panic(expected = "sequence contains more than one element")]
fn test_single_or_multiple_panics() {
    let _ = vec![1, 2].into_iter().single_or(99);
}

// ── index_ (Phase 2.6) ───────────────────────────────────────────────────────

// ── group_by overloads (Phase 2.2) ───────────────────────────────────────────

#[test]
fn test_group_by_with_element() {
    // Group by first letter, but store only the length of each word.
    let words = vec!["apple", "ant", "banana", "bear"];
    let mut groups: Vec<_> = words
        .into_iter()
        .group_by_with_element(|w| w.chars().next().unwrap(), |w| w.len())
        .collect();
    groups.sort_by_key(|g| *g.key());
    assert_eq!(*groups[0].key(), 'a');
    assert_eq!(groups[0].elements(), [5, 3]);
    assert_eq!(*groups[1].key(), 'b');
    assert_eq!(groups[1].elements(), [6, 4]);
}

#[test]
fn test_group_by_with_result() {
    // Group by first letter, return (letter, count).
    let words = vec!["apple", "ant", "banana", "bear", "cherry"];
    let mut results: Vec<_> = words
        .into_iter()
        .group_by_with_result(
            |w| w.chars().next().unwrap(),
            |k, members| (k, members.len()),
        )
        .collect();
    results.sort_by_key(|(k, _)| *k);
    assert_eq!(results, [('a', 2), ('b', 2), ('c', 1)]);
}

// ── zip3 (Phase 2.3) ─────────────────────────────────────────────────────────

// ── count_by / aggregate_by (Phase 2.7) ──────────────────────────────────────

#[test]
fn test_count_by() {
    let words = vec!["apple", "ant", "banana", "bear", "cherry"];
    let mut counts: Vec<_> = words
        .into_iter()
        .count_by(|w| w.chars().next().unwrap())
        .collect();
    counts.sort_by_key(|(k, _)| *k);
    assert_eq!(counts, [('a', 2), ('b', 2), ('c', 1)]);
}

#[test]
fn test_count_by_empty() {
    let v: Vec<_> = Vec::<i32>::new().into_iter().count_by(|x| *x).collect();
    assert!(v.is_empty());
}

#[test]
fn test_aggregate_by_sum_per_group() {
    let data = vec![("a", 1), ("b", 2), ("a", 3), ("b", 10), ("a", 100)];
    let mut totals: Vec<_> = data
        .into_iter()
        .aggregate_by(|(k, _)| *k, |_| 0i32, |acc, (_, v)| acc + v)
        .collect();
    totals.sort_by_key(|(k, _)| *k);
    assert_eq!(totals, [("a", 104), ("b", 12)]);
}

#[test]
fn test_aggregate_by_per_key_seed() {
    // Seed varies by key — start at 100 for "x", 0 otherwise.
    let data = vec![("x", 1), ("y", 1), ("x", 1)];
    let mut totals: Vec<_> = data
        .into_iter()
        .aggregate_by(
            |(k, _)| *k,
            |k| if *k == "x" { 100 } else { 0 },
            |acc, (_, v)| acc + v,
        )
        .collect();
    totals.sort_by_key(|(k, _)| *k);
    assert_eq!(totals, [("x", 102), ("y", 1)]);
}

// ── hash-backed set ops (Phase 3.1) ──────────────────────────────────────────

#[test]
fn test_distinct_matches_distinct() {
    let input = vec![3, 1, 2, 1, 3, 4];
    let slow: Vec<_> = input.clone().into_iter().distinct().collect();
    let fast: Vec<_> = input.into_iter().distinct().collect();
    assert_eq!(slow, fast);
}

#[test]
fn test_distinct_by_matches_distinct_by() {
    let input = vec!["apple", "ant", "banana", "bear"];
    let slow: Vec<_> = input
        .clone()
        .into_iter()
        .distinct_by(|w| w.chars().next().unwrap())
        .collect();
    let fast: Vec<_> = input
        .into_iter()
        .distinct_by(|w| w.chars().next().unwrap())
        .collect();
    assert_eq!(slow, fast);
}

#[test]
// Escape hatch: except_partial_eq works on types that are only
// PartialEq (no Hash + Eq), at O(n^2). See DECISIONS.md D-101.
fn test_except_partial_eq() {
    let v: Vec<_> = vec![1, 2, 3, 4, 5]
        .into_iter()
        .except_partial_eq(vec![2, 4])
        .collect();
    assert_eq!(v, [1, 3, 5]);
}
// Escape hatch: intersect_partial_eq works on types that are only
// PartialEq (no Hash + Eq), at O(n^2). See DECISIONS.md D-101.

#[test]
fn test_intersect_partial_eq() {
    let v: Vec<_> = vec![1, 2, 3, 4]
        .into_iter()
        .intersect_partial_eq(vec![2, 4, 6])
        .collect();
    assert_eq!(v, [2, 4]);
}

#[test]
fn test_union_() {
    let v: Vec<_> = vec![1, 2, 3].into_iter().union_(vec![2, 3, 4, 5]).collect();
    assert_eq!(v, [1, 2, 3, 4, 5]);
}

// ── hash-backed grouping (Phase 3.2) ─────────────────────────────────────────

#[test]
fn test_group_by_key_preserves_insertion_order() {
    // group_by_key must yield groups in their first-occurrence order,
    // unlike count_by / aggregate_by which yield in hash order.
    let words = vec!["cherry", "apple", "ant", "banana", "bear"];
    let groups: Vec<_> = words
        .into_iter()
        .group_by_key(|w| w.chars().next().unwrap())
        .collect();
    // First-occurrence order: 'c', 'a', 'b'.
    assert_eq!(groups.len(), 3);
    assert_eq!(*groups[0].key(), 'c');
    assert_eq!(groups[0].elements(), ["cherry"]);
    assert_eq!(*groups[1].key(), 'a');
    assert_eq!(groups[1].elements(), ["apple", "ant"]);
    assert_eq!(*groups[2].key(), 'b');
    // Escape hatch: count_by_partial_eq works on types that are only
    // PartialEq (no Hash + Eq), at O(n^2). See DECISIONS.md D-101.
    assert_eq!(groups[2].elements(), ["banana", "bear"]);
}

// Escape hatch: count_by_partial_eq works on types that are only
// PartialEq (no Hash + Eq), at O(n^2). See DECISIONS.md D-101.
#[test]
fn test_count_by_partial_eq() {
    let words = vec!["apple", "ant", "banana", "bear", "cherry"];
    let mut counts: Vec<_> = words
        .into_iter()
        .count_by_partial_eq(|w| w.chars().next().unwrap())
        .collect();
    counts.sort_by_key(|(k, _)| *k);
    assert_eq!(counts, [('a', 2), ('b', 2), ('c', 1)]);
}

#[test]
fn test_aggregate_by() {
    let data = vec![("a", 1), ("b", 2), ("a", 3), ("b", 10)];
    let mut totals: Vec<_> = data
        .into_iter()
        .aggregate_by(|(k, _)| *k, |_| 0, |acc, (_, v)| acc + v)
        .collect();
    totals.sort_by_key(|(k, _)| *k);
    assert_eq!(totals, [("a", 4), ("b", 12)]);
}
// Escape hatch: inner_join_partial_eq works on types that are only
// PartialEq (no Hash + Eq), at O(n^2). See DECISIONS.md D-101.

// ── hash-backed joins (Phase 3.3) ────────────────────────────────────────────

#[test]
fn test_inner_join_partial_eq() {
    let customers = vec![(1u32, "Alice"), (2, "Bob"), (3, "Carol")];
    let orders = vec![(1u32, "Laptop"), (1, "Mouse"), (2, "Keyboard")];
    let mut results: Vec<String> = customers
        .into_iter()
        .inner_join_partial_eq(
            orders,
            |(id, _)| *id,
            |(id, _)| *id,
            |(_, name), (_, product)| format!("{name} bought {product}"),
        )
        .collect();
    results.sort();
    assert_eq!(
        results,
        [
            "Alice bought Laptop",
            "Alice bought Mouse",
            // Escape hatch: group_join_partial_eq works on types that are only
            // PartialEq (no Hash + Eq), at O(n^2). See DECISIONS.md D-101.
            "Bob bought Keyboard"
        ]
    );
}

// Escape hatch: group_join_partial_eq works on types that are only
// PartialEq (no Hash + Eq), at O(n^2). See DECISIONS.md D-101.
#[test]
fn test_group_join_partial_eq() {
    let departments = vec![(1u32, "Engineering"), (2u32, "Sales")];
    let employees = vec![(1u32, "Alice"), (1, "Bob"), (2, "Carol")];
    let result: Vec<_> = departments
        .into_iter()
        .group_join_partial_eq(
            employees,
            |(id, _)| *id,
            |(dept_id, _)| *dept_id,
            |(_, dept), emps| {
                let names: Vec<_> = emps.into_iter().map(|(_, n)| n).collect();
                (dept, names)
            },
        )
        .collect();
    assert_eq!(result[0], ("Engineering", vec!["Alice", "Bob"]));
    assert_eq!(result[1], ("Sales", vec!["Carol"]));
}

// ── size_hint, ExactSizeIterator, DoubleEndedIterator (Phase 3.4 / 3.5) ──────
// Tests use Vec::into_iter() because RangeInclusive doesn't implement
// ExactSizeIterator in std.

#[test]
fn test_select_propagates_exact_size() {
    let it = vec![1, 2, 3, 4, 5].into_iter().select(|x| x * 2);
    assert_eq!(it.len(), 5);
}

#[test]
fn test_skip_propagates_size() {
    let it = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10].into_iter().skip_(3);
    assert_eq!(it.size_hint(), (7, Some(7)));
    assert_eq!(it.len(), 7);
}

#[test]
fn test_take_propagates_size() {
    let it = vec![1; 100].into_iter().take_(5);
    assert_eq!(it.size_hint(), (5, Some(5)));
    assert_eq!(it.len(), 5);
}

#[test]
fn test_take_clamps_to_inner() {
    let it = vec![1, 2, 3].into_iter().take_(99);
    assert_eq!(it.size_hint(), (3, Some(3)));
}

#[test]
fn test_select_double_ended() {
    let mut it = vec![1, 2, 3, 4, 5].into_iter().select(|x| x * 10);
    assert_eq!(it.next_back(), Some(50));
    assert_eq!(it.next(), Some(10));
    assert_eq!(it.next_back(), Some(40));
}

// ── realistic end-to-end pipeline ────────────────────────────────────────────

#[test]
fn test_realistic_pipeline() {
    #[derive(Clone, Debug)]
    struct Employee {
        name: &'static str,
        dept: &'static str,
        salary: u32,
    }

    let employees = vec![
        Employee {
            name: "Alice",
            dept: "Eng",
            salary: 120_000,
        },
        Employee {
            name: "Bob",
            dept: "Eng",
            salary: 95_000,
        },
        Employee {
            name: "Carol",
            dept: "Sales",
            salary: 80_000,
        },
        Employee {
            name: "Dave",
            dept: "Sales",
            salary: 75_000,
        },
        Employee {
            name: "Eve",
            dept: "Eng",
            salary: 130_000,
        },
        Employee {
            name: "Frank",
            dept: "HR",
            salary: 70_000,
        },
    ];

    // Top earners per department (salary > 85k), sorted by dept then salary desc.
    let result: Vec<(&str, &str, u32)> = employees
        .into_iter()
        .where_(|e| e.salary > 85_000)
        .select(|e| (e.dept, e.name, e.salary))
        .order_by(|(dept, _, _)| *dept)
        .then_by_descending(|(_, _, sal)| *sal)
        .collect();

    assert_eq!(result.len(), 3);
    assert_eq!(result[0], ("Eng", "Eve", 130_000));
    assert_eq!(result[1], ("Eng", "Alice", 120_000));
    assert_eq!(result[2], ("Eng", "Bob", 95_000));
}
