//! Cross-crate seam tests. `publish = false`; see the manifest and `D-024`.
//!
//! The library itself is empty — this crate exists to host `tests/` and
//! `examples/` that need both `linq_rs` and `linq_rs_sql` in scope. The module
//! docs below are the one exception: they hold the `D-102` boundary gate, which
//! cannot live in either published crate.
//!
//! # `D-102`: the boundary is a compile error, pinned as a differential pair
//!
//! `Rows` is not an `Iterator` and does not implement `LinqExt`. A
//! non-translatable operator like `select_many` is therefore a **compile error**
//! until `.to_memory()` has been called. `linq_rs_sql/src/rows.rs` explains why
//! the gate cannot live there: a `compile_fail` doctest in that crate cannot
//! import `linq_rs` (`D-020` forbids the dependency), so it would pin only half
//! the pair — and `compile_fail` does not check *which* error it got, so a
//! doctest failing on a missing import passes just as happily as one failing for
//! the intended reason.
//!
//! This crate dev-depends on both, so the pair can be written properly. The two
//! halves are the **same expression**, differing only by `.to_memory(&rows)`:
//!
//! The negative half — `select_many` directly on `Rows` must not compile:
//!
//! ```compile_fail
//! use linq_rs::LinqExt;
//! use linq_rs_sql::prelude::*;
//! use linq_rs_sql::rows::query;
//!
//! linq_rs_sql::table! { staff (id) { id -> Integer, name -> Text } }
//! struct Person { id: i64, name: String }
//! linq_rs_sql::entity! { Person => staff { id: Integer = id, name: Text = name } }
//!
//! let rows = vec![Person { id: 1, name: "Ada".into() }];
//! let _ = query::<Person>().select_many(|p: &Person| vec![p.id]);
//! ```
//!
//! The positive half — byte-identical but for `.to_memory(&rows)`, and it must
//! compile *and run*. This half is what makes the negative half trustworthy: if
//! the imports or the macros were broken, this would fail loudly rather than
//! letting `compile_fail` succeed for the wrong reason.
//!
//! ```rust
//! use linq_rs::LinqExt;
//! use linq_rs_sql::prelude::*;
//! use linq_rs_sql::rows::query;
//!
//! linq_rs_sql::table! { staff (id) { id -> Integer, name -> Text } }
//! struct Person { id: i64, name: String }
//! linq_rs_sql::entity! { Person => staff { id: Integer = id, name: Text = name } }
//!
//! let rows = vec![Person { id: 1, name: "Ada".into() }];
//! let out: Vec<i64> = query::<Person>()
//!     .to_memory(&rows)
//!     .select_many(|p: &Person| vec![p.id])
//!     .collect();
//! assert_eq!(out, [1]);
//! ```
//!
//! The error the negative half produces is `E0277` — "the trait bound
//! `Rows<Person, AlwaysTrue, Unordered>: CallToMemoryFirst` is not satisfied" —
//! because `Rows` carries a `select_many` stub behind a bound that is never
//! implemented. It is **not** `E0599`: the method resolves, its bound does not.
