# linq_rs

A **LINQ-style query library for Rust** — zero external dependencies.

Brings the full power of C# LINQ to Rust iterators as lazy, composable extension methods.

---

## Quick Start

```rust
use linq_rs::LinqExt;

let result: Vec<_> = vec![1, 2, 3, 4, 5, 6]
    .into_iter()
    .where_(|x| x % 2 == 0)   // filter
    .select(|x| x * x)         // project
    .to_vec();

assert_eq!(result, [4, 16, 36]);
```

Add to `Cargo.toml`:

```toml
[dependencies]
linq_rs = { path = "." }
```

---

## API Reference

### Filtering

| Rust (linq_rs)              | C# LINQ equivalent                     |
|-----------------------------|----------------------------------------|
| `where_(predicate)`         | `Where(predicate)`                     |
| `where_indexed(\|x, i\| ...)` | `Where((item, index) => ...)`        |

### Projection

| Rust                                    | C#                                     |
|-----------------------------------------|----------------------------------------|
| `select(f)`                             | `Select(f)`                            |
| `select_indexed(\|x, i\| ...)`            | `Select((item, index) => ...)`       |
| `select_many(f)`                        | `SelectMany(f)`                        |
| `select_many_indexed(\|x, i\| ...)`       | `SelectMany((item, index) => ...)`   |
| `flatten_()`                            | `SelectMany(x => x)`                   |
| `of_type::<U>()`                        | `OfType<U>()`                          |
| `cast::<U>()`                           | `Cast<U>()` (panics)                   |

`of_type` and `cast` both route through `TryInto<U>` — Rust's compile-time
analogue to C#'s runtime type test. `of_type` drops elements that don't
convert; `cast` panics on the first failure.

### Paging / Slicing

| Rust                    | C#                     |
|-------------------------|------------------------|
| `skip_(n)`                       | `Skip(n)`                            |
| `skip_while_(p)`                 | `SkipWhile(p)`                       |
| `skip_while_indexed(\|x, i\| ...)` | `SkipWhile((item, index) => ...)`  |
| `skip_last(n)`                   | `SkipLast(n)`                        |
| `take_(n)`                       | `Take(n)`                            |
| `take_while_(p)`                 | `TakeWhile(p)`                       |
| `take_while_indexed(\|x, i\| ...)` | `TakeWhile((item, index) => ...)`  |
| `take_last(n)`                   | `TakeLast(n)`                        |
| `chunk(size)`                    | `Chunk(size)`                        |

### Set Operations

| Rust                              | C#                                 |
|-----------------------------------|------------------------------------|
| `distinct()`                      | `Distinct()`                       |
| `distinct_by(key_fn)`             | `DistinctBy(key_fn)`               |
| `except(other)`                   | `Except(other)`                    |
| `except_by(other_keys, key_fn)`   | `ExceptBy(other_keys, key_fn)`     |
| `intersect(other)`                | `Intersect(other)`                 |
| `intersect_by(other_keys, key_fn)`| `IntersectBy(other_keys, key_fn)`  |
| `union_(other)`                   | `Union(other)`                     |
| `union_by(other, key_fn)`         | `UnionBy(other, key_fn)`           |
| `concat_(other)`                  | `Concat(other)`                    |

For `except_by` / `intersect_by`, the second argument is the iterable of
**keys** (matching C#). For `union_by` it is the iterable of **items**.

### Ordering

| Rust                              | C#                              |
|-----------------------------------|---------------------------------|
| `order()`                         | `Order()` (.NET 7+)             |
| `order_descending()`              | `OrderDescending()` (.NET 7+)   |
| `order_by(key_fn)`                | `OrderBy(key_fn)`               |
| `order_by_descending(key_fn)`     | `OrderByDescending(key_fn)`     |
| `.then_by(key_fn)`                | `.ThenBy(key_fn)`               |
| `.then_by_descending(key_fn)`     | `.ThenByDescending(key_fn)`     |
| `reverse()`                       | `Reverse()`                     |

Sorting is **deferred** to the first `into_iter()` — `order_by` and
`then_by` only stash comparators, so chained calls compose into a single
lexicographic sort rather than re-sorting the data each step.

### Aggregation

| Rust                                       | C#                                  |
|--------------------------------------------|-------------------------------------|
| `aggregate(seed, f)`                       | `Aggregate(seed, func)`             |
| `reduce_(f)`                               | `Aggregate(func)` (no seed)         |
| `aggregate_with_selector(seed, f, sel)`    | `Aggregate(seed, func, resultSel)`  |
| `sum_()`                                   | `Sum()`                             |
| `sum_by(selector)`                         | `Sum(selector)`                     |
| `count_where(p)`                           | `Count(p)`                          |
| `min_()`                                   | `Min()`                             |
| `max_()`                                   | `Max()`                             |
| `min_by_key_(key_fn)`                      | `MinBy(keySelector)`                |
| `max_by_key_(key_fn)`                      | `MaxBy(keySelector)`                |
| `average(selector)`                        | `Average(selector)`                 |

### Element Operations

Strict (panicking) variants on the left; `_or_default` variants return `Option<T>`.

| Rust                            | C#                           |
|---------------------------------|------------------------------|
| `first()`                       | `First()`                          |
| `first_or_default()`            | `FirstOrDefault()`                 |
| `first_or(default)`             | `FirstOrDefault(defaultValue)`     |
| `first_where(p)`                | `FirstOrDefault(p)`                |
| `last_()`                       | `Last()`                           |
| `last_or_default()`             | `LastOrDefault()`                  |
| `last_or(default)`              | `LastOrDefault(defaultValue)`      |
| `last_where(p)`                 | `LastOrDefault(p)`                 |
| `single()`                      | `Single()`                         |
| `single_or_default()`           | `SingleOrDefault()`                |
| `single_or(default)`            | `SingleOrDefault(defaultValue)`    |
| `element_at_strict(index)`      | `ElementAt(index)`                 |
| `element_at(index)`             | `ElementAtOrDefault(index)`        |
| `element_at_or(index, default)` | `ElementAtOrDefault(idx, default)` |
| `default_if_empty(value)`       | `DefaultIfEmpty(value)`            |

### Quantifiers

| Rust                | C#              |
|---------------------|-----------------|
| `any_(p)`           | `Any(p)`        |
| `all_(p)`           | `All(p)`        |
| `contains_(value)`  | `Contains(val)` |
| `is_empty_()`       | `!Any()`        |

### Joining

| Rust                                           | C#                                      |
|------------------------------------------------|-----------------------------------------|
| `join(inner, outerKey, innerKey, resultSel)`   | `Join(inner, ok, ik, rs)`               |
| `group_join(inner, outerKey, innerKey, rs)`    | `GroupJoin(inner, ok, ik, rs)`          |

### Grouping

| Rust                                              | C#                                          |
|---------------------------------------------------|---------------------------------------------|
| `group_by(key_fn)`                                | `GroupBy(keySelector)`                      |
| `group_by_with_element(key_fn, element_fn)`       | `GroupBy(keySelector, elementSelector)`     |
| `group_by_with_result(key_fn, result_fn)`         | `GroupBy(keySelector, resultSelector)`      |
| `count_by(key_fn)`                                | `CountBy(keySelector)` (.NET 9+)            |
| `aggregate_by(key_fn, seed_fn, accum)`            | `AggregateBy(keySelector, seedFn, func)` (.NET 9+) |

`group_by` returns an iterator of [`Grouping<K, T>`] — each item has a
`.key` and `.elements`. The overloads transform the elements or fold each
group into a single value.

### Conversion

| Rust                           | C#                               |
|--------------------------------|----------------------------------|
| `to_vec()`                     | `ToList()`                       |
| `to_hashmap(key_fn)`           | `ToDictionary(key_fn)`           |
| `to_hashset()`                 | `ToHashSet()`                    |
| `to_lookup(key_fn)`            | `ToLookup(key_fn)`               |

### Utility

| Rust                                      | C#                                  |
|-------------------------------------------|-------------------------------------|
| `zip_(other, result_sel)`                 | `Zip(other, resultSelector)`        |
| `zip3(second, third, result_sel)`         | `Zip(second, third, resultSel)`     |
| `append_item(item)`                       | `Append(item)`                      |
| `prepend_item(item)`                      | `Prepend(item)`                     |
| `for_each_(action)`                       | `ForEach(action)`                   |
| `sequence_equal(other)`                   | `SequenceEqual(other)`              |
| `index_()`                                | `Index()` (.NET 9+)                 |

### Source Generators (free functions, not on `LinqExt`)

| Rust                          | C#                                |
|-------------------------------|-----------------------------------|
| `linq_rs::range(start, count)`| `Enumerable.Range(start, count)`  |
| `linq_rs::repeat(val, count)` | `Enumerable.Repeat(val, count)`   |
| `linq_rs::empty::<T>()`       | `Enumerable.Empty<T>()`           |

---

## Realistic Example

```rust
use linq_rs::{LinqExt, ThenBy};

#[derive(Clone)]
struct Employee { name: &'static str, dept: &'static str, salary: u32 }

let employees = vec![
    Employee { name: "Alice",  dept: "Eng",   salary: 120_000 },
    Employee { name: "Bob",    dept: "Eng",   salary: 95_000  },
    Employee { name: "Carol",  dept: "Sales", salary: 80_000  },
    Employee { name: "Eve",    dept: "Eng",   salary: 130_000 },
];

// High-earners per dept, sorted by dept then salary descending
let result: Vec<_> = employees
    .into_iter()
    .where_(|e| e.salary > 90_000)
    .order_by(|e| e.dept)
    .then_by_descending(|e| e.salary)
    .into_iter()
    .select(|e| (e.dept, e.name, e.salary))
    .to_vec();

// [("Eng", "Eve", 130000), ("Eng", "Alice", 120000), ("Eng", "Bob", 95000)]
```

## Group Join (Left Outer Join)

```rust
use linq_rs::LinqExt;

let depts  = vec![(1u32, "Eng"), (2, "Sales")];
let emps   = vec![(1u32, "Alice"), (1, "Bob"), (2, "Carol")];

let result: Vec<_> = depts.into_iter().group_join(
    emps,
    |(id, _)| *id,
    |(dept_id, _)| *dept_id,
    |(_, dept), members| {
        let names: Vec<_> = members.into_iter().map(|(_, n)| n).collect();
        format!("{dept}: {}", names.join(", "))
    },
).collect();

// ["Eng: Alice, Bob", "Sales: Carol"]
```

## Lookup

```rust
use linq_rs::LinqExt;

let data = vec![("fruit", "apple"), ("veggie", "carrot"), ("fruit", "banana")];
let lookup = data.into_iter().to_lookup(|(cat, _)| *cat);

assert_eq!(lookup.get(&"fruit"), &[("fruit", "apple"), ("fruit", "banana")]);
```

---

## Performance — `*_hashed` variants

Several operators ship in two forms: a default `PartialEq`-only version
(O(n²)) and a `*_hashed` variant that requires `Eq + Hash` (O(n)). **Prefer
the hashed version** unless your item or key type can't implement `Hash`
(e.g. it contains floats).

| Slow (PartialEq)        | Fast (Eq + Hash)           |
|-------------------------|----------------------------|
| `distinct`              | `distinct_hashed`          |
| `distinct_by`           | `distinct_by_hashed`       |
| `except`                | `except_hashed`            |
| `intersect`             | `intersect_hashed`         |
| `union_`                | `union_hashed`             |
| `group_by`              | `group_by_hashed`          |
| `count_by`              | `count_by_hashed`          |
| `aggregate_by`          | `aggregate_by_hashed`      |
| `join`                  | `join_hashed`              |
| `group_join`            | `group_join_hashed`        |

The hashed grouping/aggregation variants yield results in **hash order**,
not insertion order — except `group_by_hashed`, which preserves insertion
order of first-occurrence (same as `group_by`).

## Versioning

This crate follows [Semantic Versioning](https://semver.org/). Some
project-specific clarifications:

- **Adding a new method to `LinqExt`** is treated as a **minor** bump, not a
  breaking change. Technically a new method could shadow a user's own
  extension method on `Iterator`, but the extension-trait pattern in the
  Rust ecosystem (`itertools`, `tokio-stream`, etc.) treats this as
  additive. If you derive your own trait methods and want isolation, name
  them explicitly with the full path.
- **Adding a new variant to a public enum or a new public field** is a
  **breaking** change.
- **Tightening a trait bound** on an existing method is a **breaking** change.
- **Loosening a trait bound** (e.g. `Eq + Hash + Clone` → `Eq + Hash`) is a
  **minor** bump.
- **Renaming an operator** is always **breaking**, even for ergonomic fixes.
- **`*_hashed` variants are independent operators** — changing their
  signature is breaking just like the non-hashed ones.
- **Pre-1.0:** anything can break in a minor bump. `0.1.x → 0.2.x` may
  include breaking changes; `0.1.x → 0.1.y` (patch) will not.

## Design Notes

- **Lazy by default** — filtering, projection, and slicing adaptors are lazy iterators; no allocation happens until you `collect()` or iterate.
- **Eager where necessary** — `order_by`, `reverse`, `distinct`, set operations, and joins must buffer the sequence. This mirrors C# LINQ's behaviour.
- **Zero dependencies** — only `std`.
- **Naming** — methods that shadow Rust keywords or `std` trait methods are suffixed with `_` (`where_`, `take_`, `any_`, etc.).
- **size_hint / ExactSizeIterator / DoubleEndedIterator** — propagated through the lazy adaptors where possible (`Select`, `Skip`, `Take`, `Concat`, `Zip`, `Reverse`, `Chunk`, `DefaultIfEmpty`, `SkipLast`) so downstream consumers can pre-allocate or iterate in reverse.
