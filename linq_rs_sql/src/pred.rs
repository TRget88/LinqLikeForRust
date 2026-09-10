//! [`pred!`](crate::pred!) — write a filter the way you would write a closure.
//!
//! The builder form is precise but does not read like the query it expresses:
//!
//! ```text
//! .filter(employees::salary.gt(100_000i64).and(employees::dept.eq("eng")))
//! ```
//!
//! [`pred!`](crate::pred!) takes the closure-shaped source instead and expands to exactly
//! that call:
//!
//! ```text
//! .filter(pred!(employees, |e| e.salary > 100_000 && e.dept == "eng"))
//! ```
//!
//! # Why a macro, and why this is not a DSL
//!
//! A closure cannot be translated. `|e| e.salary > 100_000` compiles to a
//! function; nothing at runtime can ask it which column, which operator, which
//! value. C# gets around this with a compiler feature Rust does not have — a
//! lambda typed `Expression<Func<T,bool>>` is emitted as a syntax tree rather
//! than a method, and that tree is what EF walks.
//!
//! Rust's substitute is a macro, because macros see syntax before it becomes
//! code. `pred!` is a **front end and nothing more**: it expands to the same
//! builder calls you could write by hand, so the type checking, the SQL, and
//! the in-memory evaluation are all unchanged. It is not a query language, and
//! it does not replace method chaining — see [`DECISIONS.md`](https://github.com/TRget88/LinqLikeForRust/blob/main/DECISIONS.md) `D-201`, which
//! rejects a `from … where … select …` comprehension, and `D-021`, which
//! records why this is a different thing.
//!
//! # The grammar
//!
//! Deliberately small:
//!
//! - a comparison — `binding.field OP operand`, where `OP` is one of
//!   `>`, `>=`, `<`, `<=`, `==`, `!=`
//! - those joined by `&&` and `||`, with the usual precedence (`&&` binds
//!   tighter)
//! - parentheses to override it
//!
//! Anything else is a compile error, and that is the point: **the grammar
//! boundary and the translation boundary are the same line.**
//! `|e| e.salary * 2 > budget` does not parse here, and it could not have
//! become SQL either.
//! Reach for [`to_memory`](crate::rows::Rows::to_memory) and a real closure
//! when you want arbitrary Rust.
//!
//! # Import the prelude
//!
//! `pred!` expands to method calls on the ops traits, so they must be in scope.
//! Use `linq_rs_sql::prelude::*`. Without it the error is actively misleading,
//! because [`Iterator::gt`] exists and rustc finds that instead:
//! `` error[E0599]: `salary` is not an iterator ``.

/// Builds a predicate from closure-shaped source. See the [module
/// docs](self) for the grammar.
///
/// ```rust
/// use linq_rs_sql::prelude::*;
///
/// table! { employees (id) { id -> Integer, dept -> Text, salary -> Integer } }
/// pub struct Employee { pub id: i64, pub dept: String, pub salary: i64 }
/// entity! { Employee => employees { id: Integer = id, dept: Text = dept, salary: Integer = salary } }
///
/// let q = query::<Employee>()
///     .filter(pred!(employees, |e| e.salary > 100_000i64 && e.dept == "eng"));
///
/// assert_eq!(
///     q.to_sql().sql,
///     "SELECT * FROM employees WHERE ((salary > ?) AND (dept = ?))"
/// );
///
/// // The same value evaluates in memory.
/// let staff = vec![
///     Employee { id: 1, dept: "eng".into(),   salary: 180_000 },
///     Employee { id: 2, dept: "sales".into(), salary: 190_000 },
/// ];
/// let ids: Vec<i64> = q.to_memory(&staff).map(|e| e.id).collect();
/// assert_eq!(ids, [1]);
/// ```
///
/// # The grammar boundary is a compile error, on your own line
///
/// Arithmetic is outside the grammar — and could not have become SQL either:
///
/// ```compile_fail
/// use linq_rs_sql::prelude::*;
/// table! { employees (id) { id -> Integer, salary -> Integer } }
/// pub struct Employee { pub id: i64, pub salary: i64 }
/// entity! { Employee => employees { id: Integer = id, salary: Integer = salary } }
///
/// // error: no rules expected `*`  --> your file, at the `*`
/// let _ = query::<Employee>().filter(pred!(employees, |e| e.salary * 2 > 100i64));
/// ```
///
/// Column types are still enforced through the expansion — comparing a `Text`
/// column to an integer does not compile:
///
/// ```compile_fail
/// use linq_rs_sql::prelude::*;
/// table! { employees (id) { id -> Integer, dept -> Text } }
/// pub struct Employee { pub id: i64, pub dept: String }
/// entity! { Employee => employees { id: Integer = id, dept: Text = dept } }
///
/// // error[E0271]: type mismatch resolving `<i64 as Expr>::SqlType == Text`
/// let _ = query::<Employee>().filter(pred!(employees, |e| e.dept > 3i64));
/// ```
#[macro_export]
macro_rules! pred {
    // Entry point: drop the closure header and descend by precedence.
    ($tbl:ident, |$b:ident| $($body:tt)+) => { $crate::pred!(@or $tbl, $b, [] $($body)+) };

    // `||` binds loosest, so split on it first. The accumulator collects tokens
    // until a top-level operator appears.
    (@or $tbl:ident, $b:ident, [$($acc:tt)*] || $($rest:tt)+) => {
        $crate::BoolOps::or(
            $crate::pred!(@and $tbl, $b, [] $($acc)*),
            $crate::pred!(@or $tbl, $b, [] $($rest)+),
        )
    };
    (@or $tbl:ident, $b:ident, [$($acc:tt)*] $t:tt $($rest:tt)*) => {
        $crate::pred!(@or $tbl, $b, [$($acc)* $t] $($rest)*)
    };
    (@or $tbl:ident, $b:ident, [$($acc:tt)*]) => { $crate::pred!(@and $tbl, $b, [] $($acc)*) };

    // Then `&&`.
    (@and $tbl:ident, $b:ident, [$($acc:tt)*] && $($rest:tt)+) => {
        $crate::BoolOps::and(
            $crate::pred!(@cmp $tbl, $b, $($acc)*),
            $crate::pred!(@and $tbl, $b, [] $($rest)+),
        )
    };
    (@and $tbl:ident, $b:ident, [$($acc:tt)*] $t:tt $($rest:tt)*) => {
        $crate::pred!(@and $tbl, $b, [$($acc)* $t] $($rest)*)
    };
    (@and $tbl:ident, $b:ident, [$($acc:tt)*]) => { $crate::pred!(@cmp $tbl, $b, $($acc)*) };

    // Leaves: a parenthesised group, or a single comparison.
    (@cmp $tbl:ident, $b:ident, ( $($inner:tt)+ )) => { $crate::pred!(@or $tbl, $b, [] $($inner)+) };
    (@cmp $tbl:ident, $b:ident, $bb:ident . $f:ident >  $($rhs:tt)+) => { $tbl::$f.gt($($rhs)+) };
    (@cmp $tbl:ident, $b:ident, $bb:ident . $f:ident >= $($rhs:tt)+) => { $tbl::$f.gte($($rhs)+) };
    (@cmp $tbl:ident, $b:ident, $bb:ident . $f:ident <  $($rhs:tt)+) => { $tbl::$f.lt($($rhs)+) };
    (@cmp $tbl:ident, $b:ident, $bb:ident . $f:ident <= $($rhs:tt)+) => { $tbl::$f.lte($($rhs)+) };
    (@cmp $tbl:ident, $b:ident, $bb:ident . $f:ident == $($rhs:tt)+) => { $tbl::$f.eq($($rhs)+) };
    (@cmp $tbl:ident, $b:ident, $bb:ident . $f:ident != $($rhs:tt)+) => { $tbl::$f.ne($($rhs)+) };
}
