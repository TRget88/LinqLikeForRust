//! Integration tests for linq_rs.
//!
//! Run with: `cargo test`

use linq_rs::{LinqExt, ThenBy};

// ── where_ / select ──────────────────────────────────────────────────────────

#[test]
fn test_where_select() {
    let result: Vec<_> = (1..=10)
        .where_(|x| x % 2 == 0)
        .select(|x| x * x)
        .collect();
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

#[test]
fn test_flatten_() {
    let data = vec![vec![1, 2], vec![3, 4], vec![5]];
    let flat: Vec<_> = data.into_iter().flatten_().collect();
    assert_eq!(flat, [1, 2, 3, 4, 5]);
}

// ── skip / take ──────────────────────────────────────────────────────────────

#[test]
fn test_skip_take() {
    let result: Vec<_> = (1..=10).skip(3).take_(4).collect();
    assert_eq!(result, [4, 5, 6, 7]);
}

#[test]
fn test_skip_while() {
    let result: Vec<_> = (1..=10).skip_while_(|x| *x < 5).collect();
    assert_eq!(result, [5, 6, 7, 8, 9, 10]);
}

#[test]
fn test_take_while() {
    let result: Vec<_> = (1..=10).take_while_(|x| *x < 6).collect();
    assert_eq!(result, [1, 2, 3, 4, 5]);
}

// ── chunk ────────────────────────────────────────────────────────────────────

#[test]
fn test_chunk_even() {
    let chunks: Vec<_> = (1..=6).chunk(2).collect();
    assert_eq!(chunks, [vec![1, 2], vec![3, 4], vec![5, 6]]);
}

#[test]
fn test_chunk_uneven() {
    let chunks: Vec<_> = (1..=7).chunk(3).collect();
    assert_eq!(chunks, [vec![1, 2, 3], vec![4, 5, 6], vec![7]]);
}

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
    let result: Vec<_> = vec![1, 2, 3, 4, 5]
        .into_iter()
        .except(vec![2, 4])
        .collect();
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
    let result: Vec<_> = vec![1, 2, 3]
        .into_iter()
        .union_(vec![2, 3, 4, 5])
        .collect();
    assert_eq!(result, [1, 2, 3, 4, 5]);
}

// ── ordering ─────────────────────────────────────────────────────────────────

#[test]
fn test_order_by() {
    let result: Vec<_> = vec![3, 1, 4, 1, 5, 9, 2]
        .into_iter()
        .order_by(|x| *x)
        .into_iter()
        .collect();
    assert_eq!(result, [1, 1, 2, 3, 4, 5, 9]);
}

#[test]
fn test_order_by_descending() {
    let result: Vec<_> = vec![3, 1, 4, 1, 5]
        .into_iter()
        .order_by_descending(|x| *x)
        .into_iter()
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
        .into_iter()
        .collect();
    assert_eq!(result, [("Charlie", 1), ("Alice", 2), ("Bob", 2)]);
}

#[test]
fn test_reverse() {
    let result: Vec<_> = (1..=5).reverse().collect();
    assert_eq!(result, [5, 4, 3, 2, 1]);
}

// ── aggregation ──────────────────────────────────────────────────────────────

#[test]
fn test_aggregate() {
    let product = (1..=5).aggregate(1, |acc, x| acc * x);
    assert_eq!(product, 120);
}

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
    let data = vec![5, 1, 8, 2, 9, 3];
    assert_eq!(data.iter().copied().min_(), Some(1));
    assert_eq!(data.iter().copied().max_(), Some(9));
}

#[test]
fn test_average() {
    let avg = vec![1.0f64, 2.0, 3.0, 4.0, 5.0]
        .into_iter()
        .average(|x| x);
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
fn test_element_at() {
    let v = vec![10, 20, 30, 40];
    assert_eq!(v.into_iter().element_at(2), Some(30));
}

#[test]
fn test_single_or_default() {
    assert_eq!(vec![42].into_iter().single_or_default(), Some(42));
    assert_eq!(vec![1, 2].into_iter().single_or_default(), None);
    assert_eq!(Vec::<i32>::new().into_iter().single_or_default(), None);
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

// ── join ─────────────────────────────────────────────────────────────────────

#[test]
fn test_inner_join() {
    let customers = vec![(1u32, "Alice"), (2, "Bob"), (3, "Carol")];
    let orders = vec![(1u32, "Laptop"), (1, "Mouse"), (2, "Keyboard")];

    let mut results: Vec<String> = customers
        .into_iter()
        .join(
            orders,
            |(id, _)| *id,
            |(id, _)| *id,
            |(_, name), (_, product)| format!("{name} bought {product}"),
        )
        .collect();
    results.sort();

    assert_eq!(
        results,
        ["Alice bought Laptop", "Alice bought Mouse", "Bob bought Keyboard"]
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

// ── group_by ──────────────────────────────────────────────────────────────────

#[test]
fn test_group_by() {
    let words = vec!["apple", "ant", "banana", "bear", "cherry"];
    let mut groups: Vec<_> = words
        .into_iter()
        .group_by(|w| w.chars().next().unwrap())
        .collect();
    groups.sort_by_key(|g| g.key);

    assert_eq!(groups[0].key, 'a');
    assert_eq!(groups[0].elements, ["apple", "ant"]);
    assert_eq!(groups[1].key, 'b');
    assert_eq!(groups[1].elements, ["banana", "bear"]);
    assert_eq!(groups[2].key, 'c');
    assert_eq!(groups[2].elements, ["cherry"]);
}

// ── to_lookup ─────────────────────────────────────────────────────────────────

#[test]
fn test_to_lookup() {
    let data = vec![("a", 1), ("b", 2), ("a", 3), ("c", 4), ("b", 5)];
    let lookup = data.into_iter().to_lookup(|(k, _)| *k);

    assert_eq!(lookup.get(&"a"), &[("a", 1), ("a", 3)]);
    assert_eq!(lookup.get(&"b"), &[("b", 2), ("b", 5)]);
    assert_eq!(lookup.get(&"c"), &[("c", 4)]);
    assert_eq!(lookup.get(&"z"), &[]);
    assert!(lookup.contains_key(&"a"));
    assert!(!lookup.contains_key(&"z"));
    assert_eq!(lookup.count(), 3);
}

// ── to_hashmap / to_hashset ───────────────────────────────────────────────────

#[test]
fn test_to_hashmap() {
    let map = vec![("one", 1), ("two", 2), ("three", 3)]
        .into_iter()
        .to_hashmap(|(k, _)| *k);
    assert_eq!(map[&"one"], ("one", 1));
    assert_eq!(map[&"two"], ("two", 2));
}

#[test]
fn test_to_hashset() {
    let set = vec![1, 2, 2, 3, 3, 3].into_iter().to_hashset();
    assert_eq!(set.len(), 3);
    assert!(set.contains(&1));
    assert!(set.contains(&2));
    assert!(set.contains(&3));
}

// ── utility ───────────────────────────────────────────────────────────────────

#[test]
fn test_zip() {
    let result: Vec<_> = vec![1, 2, 3]
        .into_iter()
        .zip_(vec![10, 20, 30], |a, b| a + b)
        .collect();
    assert_eq!(result, [11, 22, 33]);
}

#[test]
fn test_append_prepend() {
    let a: Vec<_> = vec![1, 2, 3].into_iter().append_item(99).collect();
    assert_eq!(a, [1, 2, 3, 99]);

    let p: Vec<_> = vec![1, 2, 3].into_iter().prepend_item(0).collect();
    assert_eq!(p, [0, 1, 2, 3]);
}

#[test]
fn test_is_empty() {
    assert!(Vec::<i32>::new().into_iter().is_empty_());
    assert!(!vec![1].into_iter().is_empty_());
}

#[test]
fn test_sequence_equal() {
    assert!(vec![1, 2, 3].into_iter().sequence_equal(vec![1, 2, 3]));
    assert!(!vec![1, 2, 3].into_iter().sequence_equal(vec![1, 2]));
    assert!(!vec![1, 2, 3].into_iter().sequence_equal(vec![1, 2, 4]));
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
        Employee { name: "Alice",   dept: "Eng",     salary: 120_000 },
        Employee { name: "Bob",     dept: "Eng",     salary: 95_000  },
        Employee { name: "Carol",   dept: "Sales",   salary: 80_000  },
        Employee { name: "Dave",    dept: "Sales",   salary: 75_000  },
        Employee { name: "Eve",     dept: "Eng",     salary: 130_000 },
        Employee { name: "Frank",   dept: "HR",      salary: 70_000  },
    ];

    // Top earners per department (salary > 85k), sorted by dept then salary desc.
    let result: Vec<(&str, &str, u32)> = employees
        .into_iter()
        .where_(|e| e.salary > 85_000)
        .select(|e| (e.dept, e.name, e.salary))
        .order_by(|(dept, _, _)| *dept)
        .then_by_descending(|(_, _, sal)| *sal)
        .into_iter()
        .collect();

    assert_eq!(result.len(), 3);
    assert_eq!(result[0], ("Eng", "Eve",   130_000));
    assert_eq!(result[1], ("Eng", "Alice", 120_000));
    assert_eq!(result[2], ("Eng", "Bob",    95_000));
}
