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
macro_rules! terminal_case {
    ($name:literal, pulls = $p:expr, |$s:ident| $body:expr) => {{
        let (source, pulls) = counted();
        let _ = {
            let $s = source;
            $body
        };
        assert_eq!(
            pulls.get(),
            $p,
            concat!($name, ": elements pulled from the receiver")
        );
    }};
}

// ─────────────────────────────────────────────────────────────────────────────
// FILTERING / PROJECTION
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn filtering_and_projection_are_lazy() {
    case!("where_", "lazy", at_call = 0, take2 = 2, |s| s
        .where_(|_| true));
    case!("where_indexed", "lazy", at_call = 0, take2 = 2, |s| {
        s.where_indexed(|_, _| true)
    });
    case!("select", "lazy", at_call = 0, take2 = 2, |s| s
        .select(|x| x));
    case!("select_indexed", "lazy", at_call = 0, take2 = 2, |s| {
        s.select_indexed(|x, _| x)
    });
    case!("select_many", "lazy", at_call = 0, take2 = 2, |s| {
        s.select_many(std::iter::once)
    });
    case!("select_many_indexed", "lazy", at_call = 0, take2 = 2, |s| {
        s.select_many_indexed(|x, _| std::iter::once(x))
    });
    case!("of_type", "lazy", at_call = 0, take2 = 2, |s| {
        s.of_type::<i64>()
    });
    case!("cast", "lazy", at_call = 0, take2 = 2, |s| s.cast::<i64>());
}

#[test]
fn flatten_is_lazy() {
    // `flatten_` needs an `IntoIterator` item type, so it gets its own source.
    let (source, pulls) = count((0..N as i32).map(|x| vec![x]));
    let built = source.flatten_();
    assert_eq!(pulls.get(), 0, "flatten_: pulled at call time");
    drop(built);

    let (source, pulls) = count((0..N as i32).map(|x| vec![x]));
    let out: Vec<i32> = source.flatten_().take_(2).collect();
    assert_eq!(out, [0, 1]);
    assert_eq!(pulls.get(), 2, "flatten_: pulled under take_(2)");
    assert_class("flatten_", "lazy", 0, None, 2);
}

// ─────────────────────────────────────────────────────────────────────────────
// PAGING / SLICING
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn paging_operators() {
    case!("skip_", "lazy", at_call = 0, take2 = 12, |s| s.skip_(10));
    case!("skip_while_", "lazy", at_call = 0, take2 = 12, |s| {
        s.skip_while_(|x| *x < 10)
    });
    case!("skip_while_indexed", "lazy", at_call = 0, take2 = 12, |s| {
        s.skip_while_indexed(|_, i| i < 10)
    });
    case!("take_", "lazy", at_call = 0, take2 = 2, |s| s.take_(500));
    case!("take_while_", "lazy", at_call = 0, take2 = 2, |s| {
        s.take_while_(|_| true)
    });
    case!("take_while_indexed", "lazy", at_call = 0, take2 = 2, |s| {
        s.take_while_indexed(|_, _| true)
    });
    // Two chunks of three = six source elements.
    case!("chunk", "lazy", at_call = 0, take2 = 6, |s| s.chunk(3));
    // Fills a ring buffer of five, then one source element per yielded element.
    case!("skip_last", "lazy", at_call = 0, take2 = 7, |s| s
        .skip_last(5));
    // The one paging operator that is not lazy.
    case!("take_last", "eager_at_call", at_call = N, take2 = N, |s| {
        s.take_last(5)
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// SET OPERATIONS
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn set_operators() {
    case!("distinct", "lazy", at_call = 0, take2 = 2, |s| s.distinct());
    case!("distinct_by", "lazy", at_call = 0, take2 = 2, |s| {
        s.distinct_by(|x| *x)
    });
    case!("distinct_partial_eq", "lazy", at_call = 0, take2 = 2, |s| {
        s.distinct_partial_eq()
    });
    case!(
        "distinct_by_partial_eq",
        "lazy",
        at_call = 0,
        take2 = 2,
        |s| s.distinct_by_partial_eq(|x| *x)
    );

    // `concat_` requires the argument to be the *same* iterator type.
    case_arg!(
        "concat_",
        "lazy",
        at_call = 0,
        arg_at_call = 0,
        take2 = 2,
        arg = 0..ARG_N as i32,
        |s, a| s.concat_(a)
    );

    // `union_` is `self.chain(other).filter(..)` — fully lazy on both sides.
    case_arg!(
        "union_",
        "lazy",
        at_call = 0,
        arg_at_call = 0,
        take2 = 2,
        arg = 0..ARG_N as i32,
        |s, a| s.union_(a)
    );

    // The escape hatch and the `_by` variant buffer *everything* instead.
    case_arg!(
        "union_partial_eq",
        "eager_at_call",
        at_call = N,
        arg_at_call = ARG_N,
        take2 = N,
        arg = 0..ARG_N as i32,
        |s, a| s.union_partial_eq(a)
    );
    case_arg!(
        "union_by",
        "eager_at_call",
        at_call = N,
        arg_at_call = ARG_N,
        take2 = N,
        arg = 0..ARG_N as i32,
        |s, a| s.union_by(a, |x| *x)
    );

    // Receiver stays lazy; the argument is drained into a set/vec at call time.
    case_arg!(
        "except",
        "half_eager",
        at_call = 0,
        arg_at_call = ARG_N,
        take2 = 2,
        arg = 500..500 + ARG_N as i32,
        |s, a| s.except(a)
    );
    case_arg!(
        "except_partial_eq",
        "half_eager",
        at_call = 0,
        arg_at_call = ARG_N,
        take2 = 2,
        arg = 500..500 + ARG_N as i32,
        |s, a| s.except_partial_eq(a)
    );
    case_arg!(
        "except_by",
        "half_eager",
        at_call = 0,
        arg_at_call = ARG_N,
        take2 = 2,
        arg = 500..500 + ARG_N as i32,
        |s, a| s.except_by(a, |x| *x)
    );
    case_arg!(
        "intersect",
        "half_eager",
        at_call = 0,
        arg_at_call = ARG_N,
        take2 = 2,
        arg = 0..ARG_N as i32,
        |s, a| s.intersect(a)
    );
    case_arg!(
        "intersect_partial_eq",
        "half_eager",
        at_call = 0,
        arg_at_call = ARG_N,
        take2 = 2,
        arg = 0..ARG_N as i32,
        |s, a| s.intersect_partial_eq(a)
    );
    case_arg!(
        "intersect_by",
        "half_eager",
        at_call = 0,
        arg_at_call = ARG_N,
        take2 = 2,
        arg = 0..ARG_N as i32,
        |s, a| s.intersect_by(a, |x| *x)
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// ORDERING
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ordering_operators_buffer_at_call() {
    case!("order_by", "eager_at_call", at_call = N, take2 = N, |s| {
        s.order_by(|x| *x)
    });
    case!(
        "order_by_descending",
        "eager_at_call",
        at_call = N,
        take2 = N,
        |s| s.order_by_descending(|x| *x)
    );
    case!("order", "eager_at_call", at_call = N, take2 = N, |s| s
        .order());
    case!(
        "order_descending",
        "eager_at_call",
        at_call = N,
        take2 = N,
        |s| s.order_descending()
    );
    case!("reverse", "eager_at_call", at_call = N, take2 = N, |s| {
        s.reverse()
    });
}

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

#[test]
fn utility_adaptors_are_lazy() {
    case!("default_if_empty", "lazy", at_call = 0, take2 = 2, |s| {
        s.default_if_empty(-1)
    });
    case!("append_item", "lazy", at_call = 0, take2 = 2, |s| {
        s.append_item(-1)
    });
    // The prepended element is yielded first, so only one source pull.
    case!("prepend_item", "lazy", at_call = 0, take2 = 1, |s| {
        s.prepend_item(-1)
    });
    case!("index_", "lazy", at_call = 0, take2 = 2, |s| s.index_());
    case_arg!(
        "zip_",
        "lazy",
        at_call = 0,
        arg_at_call = 0,
        take2 = 2,
        arg = 0..ARG_N as i32,
        |s, a| s.zip_(a, |x, y| x + y)
    );
}

#[test]
fn zip3_is_lazy_on_all_three_sequences() {
    let (source, pulls) = counted();
    let (second, second_pulls) = count(0..ARG_N as i32);
    let (third, third_pulls) = count(0..ARG_N as i32);
    let built = source.zip3(second, third, |a, b, c| a + b + c);
    assert_eq!(pulls.get(), 0, "zip3: pulled the receiver at call time");
    assert_eq!(second_pulls.get(), 0, "zip3: drained `second` at call time");
    assert_eq!(third_pulls.get(), 0, "zip3: drained `third` at call time");
    drop(built);

    let (source, pulls) = counted();
    let (second, second_pulls) = count(0..ARG_N as i32);
    let (third, third_pulls) = count(0..ARG_N as i32);
    let out: Vec<i32> = source
        .zip3(second, third, |a, b, c| a + b + c)
        .take_(2)
        .collect();
    assert_eq!(out, [0, 3]);
    assert_eq!(pulls.get(), 2);
    assert_eq!(second_pulls.get(), 2);
    assert_eq!(third_pulls.get(), 2);
    assert_class("zip3", "lazy", 0, Some(0), 2);
}

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

#[test]
fn skip_last_streams_an_endless_source() {
    let (source, pulls) = endless();
    let out: Vec<i32> = source.skip_last(3).take_(2).collect();
    assert_eq!(out, [0, 1]);
    assert_eq!(pulls.get(), 5);
}

// ─────────────────────────────────────────────────────────────────────────────
// TERMINALS
//
// Terminals are immediate by definition; the only measurable property is
// whether they stop before the end of the source.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn short_circuiting_terminals() {
    terminal_case!("first", pulls = 1, |s| s.first());
    terminal_case!("first_or_default", pulls = 1, |s| s.first_or_default());
    terminal_case!("first_or", pulls = 1, |s| s.first_or(-1));
    terminal_case!("first_where", pulls = 6, |s| s.first_where(|x| *x == 5));
    terminal_case!("element_at", pulls = 6, |s| s.element_at(5));
    terminal_case!("element_at_strict", pulls = 6, |s| s.element_at_strict(5));
    terminal_case!("element_at_or", pulls = 6, |s| s.element_at_or(5, -1));
    // `single`/`single_or` pull exactly two and then panic; `single_or_default`
    // is the same control flow without the panic.
    terminal_case!("single_or_default", pulls = 2, |s| s.single_or_default());
    terminal_case!("any_", pulls = 6, |s| s.any_(|x| x == 5));
    terminal_case!("all_", pulls = 6, |s| s.all_(|x| x < 5));
    terminal_case!("contains_", pulls = 6, |s| s.contains_(&5));
    terminal_case!("is_empty_", pulls = 1, |s| s.is_empty_());
    terminal_case!("sequence_equal", pulls = 4, |s| {
        s.sequence_equal(vec![0, 1, 2, 99])
    });
}

#[test]
fn draining_terminals() {
    terminal_case!("aggregate", pulls = N, |s| s
        .aggregate(0i64, |a, x| a + x as i64));
    terminal_case!("reduce_", pulls = N, |s| s.reduce_(|a, x| a + x));
    terminal_case!("aggregate_with_selector", pulls = N, |s| {
        s.aggregate_with_selector(0i64, |a, x| a + x as i64, |a| a * 2)
    });
    terminal_case!("sum_", pulls = N, |s| s.sum_::<i32>());
    terminal_case!("sum_by", pulls = N, |s| s
        .sum_by::<i64, i64, _>(|x| x as i64));
    terminal_case!("count_where", pulls = N, |s| s.count_where(|x| *x > 0));
    terminal_case!("min_", pulls = N, |s| s.min_());
    terminal_case!("max_", pulls = N, |s| s.max_());
    terminal_case!("min_by_key_", pulls = N, |s| s.min_by_key_(|x| *x));
    terminal_case!("max_by_key_", pulls = N, |s| s.max_by_key_(|x| *x));
    terminal_case!("average", pulls = N, |s| s.average(|x| x as f64));
    terminal_case!("last_", pulls = N, |s| s.last_());
    terminal_case!("last_or_default", pulls = N, |s| s.last_or_default());
    terminal_case!("last_or", pulls = N, |s| s.last_or(-1));
    terminal_case!("last_where", pulls = N, |s| s.last_where(|x| *x > 0));
    terminal_case!("for_each_", pulls = N, |s| s.for_each_(|_| {}));
    terminal_case!("to_vec", pulls = N, |s| s.to_vec());
    terminal_case!("to_hashmap", pulls = N, |s| s.to_hashmap(|x| *x));
    terminal_case!("to_hashset", pulls = N, |s| s.to_hashset());
    terminal_case!("to_lookup", pulls = N, |s| s.to_lookup(|x| x % 4));
}
