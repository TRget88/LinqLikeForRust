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

| Rust (linq_rs)          | C# LINQ equivalent     |
|-------------------------|------------------------|
| `where_(predicate)`     | `Where(predicate)`     |

### Projection

| Rust                              | C#                          |
|-----------------------------------|-----------------------------|
| `select(f)`                       | `Select(f)`                 |
| `select_many(f)`                  | `SelectMany(f)`             |
| `flatten_()`                      | `SelectMany(x => x)`        |

### Paging / Slicing

| Rust                    | C#                     |
|-------------------------|------------------------|
| `skip(n)`               | `Skip(n)`              |
| `skip_while_(p)`        | `SkipWhile(p)`         |
| `take_(n)`              | `Take(n)`              |
| `take_while_(p)`        | `TakeWhile(p)`         |
| `chunk(size)`           | `Chunk(size)`          |

### Set Operations

| Rust                    | C#                     |
|-------------------------|------------------------|
| `distinct()`            | `Distinct()`           |
| `distinct_by(key_fn)`   | `DistinctBy(key_fn)`   |
| `except(other)`         | `Except(other)`        |
| `intersect(other)`      | `Intersect(other)`     |
| `union_(other)`         | `Union(other)`         |
| `concat_(other)`        | `Concat(other)`        |

### Ordering

| Rust                              | C#                              |
|-----------------------------------|---------------------------------|
| `order_by(key_fn)`                | `OrderBy(key_fn)`               |
| `order_by_descending(key_fn)`     | `OrderByDescending(key_fn)`     |
| `.then_by(key_fn)`                | `.ThenBy(key_fn)`               |
| `.then_by_descending(key_fn)`     | `.ThenByDescending(key_fn)`     |
| `reverse()`                       | `Reverse()`                     |

### Aggregation

| Rust                        | C#                      |
|-----------------------------|-------------------------|
| `aggregate(seed, f)`        | `Aggregate(seed, f)`    |
| `sum_()`                    | `Sum()`                 |
| `count_where(p)`            | `Count(p)`              |
| `min_()`                    | `Min()`                 |
| `max_()`                    | `Max()`                 |
| `min_by_key_(key_fn)`       | `MinBy(key_fn)`         |
| `max_by_key_(key_fn)`       | `MaxBy(key_fn)`         |
| `average(selector)`         | `Average(selector)`     |

### Element Operations

| Rust                        | C#                           |
|-----------------------------|------------------------------|
| `first_or_default()`        | `FirstOrDefault()`           |
| `first_where(p)`            | `FirstOrDefault(p)`          |
| `last_or_default()`         | `LastOrDefault()`            |
| `last_where(p)`             | `LastOrDefault(p)`           |
| `element_at(index)`         | `ElementAtOrDefault(index)`  |
| `single_or_default()`       | `SingleOrDefault()`          |

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

| Rust                    | C#                  |
|-------------------------|---------------------|
| `group_by(key_fn)`      | `GroupBy(key_fn)`   |

Returns an iterator of [`Grouping<K, T>`] — each item has a `.key` and `.elements`.

### Conversion

| Rust                           | C#                               |
|--------------------------------|----------------------------------|
| `to_vec()`                     | `ToList()`                       |
| `to_hashmap(key_fn)`           | `ToDictionary(key_fn)`           |
| `to_hashset()`                 | `ToHashSet()`                    |
| `to_lookup(key_fn)`            | `ToLookup(key_fn)`               |

### Utility

| Rust                          | C#                           |
|-------------------------------|------------------------------|
| `zip_(other, result_sel)`     | `Zip(other, resultSelector)` |
| `append_item(item)`           | `Append(item)`               |
| `prepend_item(item)`          | `Prepend(item)`              |
| `for_each_(action)`           | `ForEach(action)`            |
| `sequence_equal(other)`       | `SequenceEqual(other)`       |

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

## Design Notes

- **Lazy by default** — filtering, projection, and slicing adaptors are lazy iterators; no allocation happens until you `collect()` or iterate.
- **Eager where necessary** — `order_by`, `reverse`, `distinct`, set operations, and joins must buffer the sequence. This mirrors C# LINQ's behaviour.
- **Zero dependencies** — only `std`.
- **Naming** — methods that shadow Rust keywords or `std` trait methods are suffixed with `_` (`where_`, `take_`, `any_`, etc.).
