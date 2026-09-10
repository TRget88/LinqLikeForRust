//! Measured laziness classification for every non-terminal `LinqExt` operator.
//!
//! `DECISIONS.md` `D-016` forbids hand-written behavioural claims. The
//! eagerness table in `README.md` is generated from the same facts this file
//! asserts, so the table cannot drift from the code without a test failure.
//!
//! ## What is measured
//!
//! [`Counted`] wraps a source iterator and increments a counter every time
//! `next()` hands back an element. Each operator is then exercised twice
//! against a fresh 1,000-element source:
//!
//! 1. **at call time** — the operator is called and the result is dropped
//!    without ever being iterated. A non-zero count means the operator
//!    drained (part of) its source just by being called.
//! 2. **under `take_(2)`** — the result is iterated through a downstream
//!    `take_(2)`. A count below 1,000 means the operator short-circuits.
//!
//! Operators that take a second sequence get a second, separately counted
//! argument, so "drains the receiver" and "drains the argument" are
//! distinguishable.
//!
//! The four classes asserted by [`assert_class`]:
//!
//! | class           | source at call | argument at call |
//! |-----------------|----------------|------------------|
//! | `lazy`          | 0              | 0 (or absent)    |
//! | `half_eager`    | 0              | all of it        |
//! | `eager_at_call` | all of it      | — (unconstrained)|
//! | `terminal`      | n/a — returns a value, not a sequence      |

use linq_rs::LinqExt;
use std::cell::Cell;
use std::rc::Rc;

/// Elements in every measured source.
const N: usize = 1000;
/// Elements in every measured argument sequence.
const ARG_N: usize = 1000;

// ─────────────────────────────────────────────────────────────────────────────
// The counting source
// ─────────────────────────────────────────────────────────────────────────────

/// A source iterator that counts the elements it has handed out.
///
/// Deliberately does **not** forward `size_hint`, so no operator can appear
/// lazy by reading a length instead of pulling.
struct Counted<I> {
    inner: I,
    pulls: Rc<Cell<usize>>,
}

impl<I: Iterator> Iterator for Counted<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> {
        let v = self.inner.next();
        if v.is_some() {
            self.pulls.set(self.pulls.get() + 1);
        }
        v
    }
}

/// Wraps any iterator in a counter.
fn count<I: Iterator>(inner: I) -> (Counted<I>, Rc<Cell<usize>>) {
    let pulls = Rc::new(Cell::new(0));
    (
        Counted {
            inner,
            pulls: Rc::clone(&pulls),
        },
        pulls,
    )
}

/// The standard 1,000-element source: `0..1000`.
fn counted() -> (Counted<std::ops::Range<i32>>, Rc<Cell<usize>>) {
    count(0..N as i32)
}

/// An endless source. Any operator that drains its receiver hangs on this.
fn endless() -> (Counted<std::ops::RangeFrom<i32>>, Rc<Cell<usize>>) {
    count(0..)
}

// ─────────────────────────────────────────────────────────────────────────────
// Classification
// ─────────────────────────────────────────────────────────────────────────────

/// Cross-checks the measured numbers against the declared class, so the class
/// name in this file (and therefore in the README) is itself asserted.
fn assert_class(
    name: &str,
    class: &str,
    source_at_call: usize,
    arg_at_call: Option<usize>,
    under_take2: usize,
) {
    match class {
        "lazy" => {
            assert_eq!(
                source_at_call, 0,
                "{name}: declared `lazy` but pulled {source_at_call} from the source at call time"
            );
            assert_eq!(
                arg_at_call.unwrap_or(0),
                0,
                "{name}: declared `lazy` but drained its argument at call time"
            );
            assert!(
                under_take2 < N,
                "{name}: declared `lazy` but pulled all {N} elements under take_(2)"
            );
        }
        "half_eager" => {
            assert_eq!(
                source_at_call, 0,
                "{name}: declared `half_eager` but drained the receiver at call time"
            );
            assert_eq!(
                arg_at_call.expect("half_eager requires a sequence argument"),
                ARG_N,
                "{name}: declared `half_eager` but did not drain its argument at call time"
            );
            assert!(
                under_take2 < N,
                "{name}: declared `half_eager` but pulled all {N} elements under take_(2)"
            );
        }
        "eager_at_call" => {
            assert_eq!(
                source_at_call, N,
                "{name}: declared `eager_at_call` but pulled only {source_at_call} at call time"
            );
            assert_eq!(
                under_take2, N,
                "{name}: declared `eager_at_call` but did not pull the whole source under take_(2)"
            );
        }
        other => panic!("{name}: unknown class `{other}`"),
    }
}

/// An operator with no sequence argument.
///
/// `at_call` is what the operator pulls from the receiver merely by being
/// called; `take2` is what it pulls with a `take_(2)` downstream.
macro_rules! case {
    ($name:literal, $class:literal, at_call = $at:expr, take2 = $t2:expr, |$s:ident| $body:expr) => {{
        let (source, pulls) = counted();
        let built = {
            let $s = source;
            $body
        };
        assert_eq!(
            pulls.get(),
            $at,
            concat!($name, ": elements pulled from the receiver at call time")
        );
        drop(built);

        let (source, pulls) = counted();
        let _out: Vec<_> = {
            let $s = source;
            $body
        }
        .into_iter()
        .take_(2)
        .collect();
        assert_eq!(
            pulls.get(),
            $t2,
            concat!($name, ": elements pulled from the receiver under take_(2)")
        );

        assert_class($name, $class, $at, None, $t2);
    }};
}

/// An operator that takes a second sequence, counted separately.
macro_rules! case_arg {
    (
        $name:literal, $class:literal,
        at_call = $at:expr, arg_at_call = $aat:expr, take2 = $t2:expr,
        arg = $arg:expr,
        |$s:ident, $a:ident| $body:expr
    ) => {{
        let (source, pulls) = counted();
        let (argument, arg_pulls) = count($arg);
        let built = {
            let $s = source;
            let $a = argument;
            $body
        };
        assert_eq!(
            pulls.get(),
            $at,
            concat!($name, ": elements pulled from the receiver at call time")
        );
        assert_eq!(
            arg_pulls.get(),
            $aat,
            concat!($name, ": elements pulled from the argument at call time")
        );
        drop(built);

        let (source, pulls) = counted();
        let (argument, _) = count($arg);
        let _out: Vec<_> = {
            let $s = source;
            let $a = argument;
            $body
        }
        .into_iter()
        .take_(2)
        .collect();
        assert_eq!(
            pulls.get(),
            $t2,
            concat!($name, ": elements pulled from the receiver under take_(2)")
        );

        assert_class($name, $class, $at, Some($aat), $t2);
    }};
}

/// A terminal operator: no laziness to classify, only whether it stops early.

// ─────────────────────────────────────────────────────────────────────────────
// FILTERING / PROJECTION
// ─────────────────────────────────────────────────────────────────────────────

// ─────────────────────────────────────────────────────────────────────────────
// PAGING / SLICING
// ─────────────────────────────────────────────────────────────────────────────

// ─────────────────────────────────────────────────────────────────────────────
// SET OPERATIONS
// ─────────────────────────────────────────────────────────────────────────────

// ─────────────────────────────────────────────────────────────────────────────
// ORDERING
// ─────────────────────────────────────────────────────────────────────────────

// ─────────────────────────────────────────────────────────────────────────────
// JOINING
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn join_operators_buffer_at_call() {
    case_arg!(
        "inner_join",
        "eager_at_call",
        at_call = N,
        arg_at_call = ARG_N,
        take2 = N,
        arg = 0..ARG_N as i32,
        |s, a| s.inner_join(a, |x| *x, |y| *y, |x, y| (x, y))
    );
    case_arg!(
        "inner_join_partial_eq",
        "eager_at_call",
        at_call = N,
        arg_at_call = ARG_N,
        take2 = N,
        arg = 0..ARG_N as i32,
        |s, a| s.inner_join_partial_eq(a, |x| *x, |y| *y, |x, y| (x, y))
    );
    case_arg!(
        "group_join",
        "eager_at_call",
        at_call = N,
        arg_at_call = ARG_N,
        take2 = N,
        arg = 0..ARG_N as i32,
        |s, a| s.group_join(a, |x| *x, |y| *y, |x, g| (x, g))
    );
    case_arg!(
        "group_join_partial_eq",
        "eager_at_call",
        at_call = N,
        arg_at_call = ARG_N,
        take2 = N,
        arg = 0..ARG_N as i32,
        |s, a| s.group_join_partial_eq(a, |x| *x, |y| *y, |x, g| (x, g))
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// GROUPING
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn grouping_operators_buffer_at_call() {
    case!(
        "group_by_key",
        "eager_at_call",
        at_call = N,
        take2 = N,
        |s| s.group_by_key(|x| x % 4)
    );
    case!(
        "group_by_key_partial_eq",
        "eager_at_call",
        at_call = N,
        take2 = N,
        |s| s.group_by_key_partial_eq(|x| x % 4)
    );
    case!(
        "group_by_with_element",
        "eager_at_call",
        at_call = N,
        take2 = N,
        |s| s.group_by_with_element(|x| x % 4, |x| x)
    );
    case!(
        "group_by_with_result",
        "eager_at_call",
        at_call = N,
        take2 = N,
        |s| s.group_by_with_result(|x| x % 4, |k, v| (k, v.len()))
    );
    case!("count_by", "eager_at_call", at_call = N, take2 = N, |s| {
        s.count_by(|x| x % 4)
    });
    case!(
        "count_by_partial_eq",
        "eager_at_call",
        at_call = N,
        take2 = N,
        |s| s.count_by_partial_eq(|x| x % 4)
    );
    case!(
        "aggregate_by",
        "eager_at_call",
        at_call = N,
        take2 = N,
        |s| s.aggregate_by(|x| x % 4, |_| 0i64, |acc, x| acc + x as i64)
    );
    case!(
        "aggregate_by_partial_eq",
        "eager_at_call",
        at_call = N,
        take2 = N,
        |s| s.aggregate_by_partial_eq(|x| x % 4, |_| 0i64, |acc, x| acc + x as i64)
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// ELEMENT / UTILITY ADAPTORS
// ─────────────────────────────────────────────────────────────────────────────

// ─────────────────────────────────────────────────────────────────────────────
// THE INFINITE-SOURCE PROOFS
//
// An operator that buffers its receiver cannot terminate on an endless
// source. These tests would hang, not fail, on a regression — which is
// exactly the point: they prove streaming rather than merely counting it.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn distinct_streams_an_endless_source() {
    let (source, pulls) = endless();
    let out: Vec<i32> = source.distinct().take_(2).collect();
    assert_eq!(out, [0, 1]);
    assert_eq!(
        pulls.get(),
        2,
        "distinct is hash-backed but still streaming"
    );
}

#[test]
fn distinct_by_streams_an_endless_source() {
    let (source, pulls) = endless();
    let out: Vec<i32> = source.distinct_by(|x| *x).take_(2).collect();
    assert_eq!(out, [0, 1]);
    assert_eq!(pulls.get(), 2);
}

#[test]
fn union_streams_an_endless_receiver() {
    let (source, pulls) = endless();
    let out: Vec<i32> = source.union_(vec![-1, -2]).take_(2).collect();
    assert_eq!(out, [0, 1]);
    assert_eq!(
        pulls.get(),
        2,
        "union_ chains and filters; it does not buffer"
    );
}

#[test]
fn half_eager_operators_keep_an_endless_receiver_streaming() {
    let (source, pulls) = endless();
    let out: Vec<i32> = source.except(vec![0, 1]).take_(2).collect();
    assert_eq!(out, [2, 3]);
    assert_eq!(pulls.get(), 4);

    let (source, pulls) = endless();
    let out: Vec<i32> = source.intersect(vec![5, 7, 9]).take_(2).collect();
    assert_eq!(out, [5, 7]);
    assert_eq!(pulls.get(), 8);
}

// ─────────────────────────────────────────────────────────────────────────────
// TERMINALS
//
// Terminals are immediate by definition; the only measurable property is
// whether they stop before the end of the source.
// ─────────────────────────────────────────────────────────────────────────────
