//! Declarative macros for the SQL module.
//!
//! Only [`crate::table!`] for now — it's exported at the crate root (the
//! `#[macro_export]` attribute pins it there). Users invoke it as
//! `linq_rs::table! { ... }` from anywhere.

/// Declare a SQL table with typed columns.
///
/// # Syntax
///
/// ```text
/// table! {
///     table_name (primary_key_col [, more_pk_cols ...]) {
///         col_name -> SqlType,
///         ...
///     }
/// }
/// ```
///
/// `SqlType` is one of the markers exported from [`crate::sql::types`]:
/// `Integer`, `Text`, `Boolean`, `Float`.
///
/// # Generated items
///
/// The macro expands into a `pub mod <table_name>` containing:
///
/// - A `Marker` zero-sized type implementing [`Table`](crate::sql::Table) —
///   the type-level identity of the table.
/// - One zero-sized type per column (lowercase, matching the column name)
///   implementing [`Column`](crate::sql::Column) and
///   [`Expr`](crate::sql::Expr). Because they're unit structs, the lowercase
///   name doubles as both a type and a value.
/// - A `table()` constructor returning a fresh
///   [`Query`](crate::sql::Query) seeded with `SELECT *`.
///
/// # Example
///
/// ```rust
/// use linq_rs::sql::*;
///
/// linq_rs::table! {
///     users (id) {
///         id    -> Integer,
///         name  -> Text,
///         age   -> Integer,
///         active -> Boolean,
///     }
/// }
///
/// let q = users::table()
///     .filter(users::age.gt(18))
///     .filter(users::active.eq(true))
///     .select((users::id, users::name))
///     .order_by_desc(users::age)
///     .limit(10)
///     .to_sql();
///
/// assert!(q.sql.contains("FROM users"));
/// assert!(q.sql.contains("WHERE"));
/// assert_eq!(q.params.len(), 2);
/// ```
#[macro_export]
macro_rules! table {
    (
        $table:ident ($($pk:ident),+ $(,)?) {
            $($col:ident -> $sql_ty:ident),+ $(,)?
        }
    ) => {
        #[allow(non_snake_case, non_camel_case_types, unused_imports, dead_code)]
        pub mod $table {
            use $crate::sql::{
                All, Boolean, Column, Expr, Float, Integer, Query, SqlValue, Table, Text,
            };

            /// Type-level marker for this table.
            #[derive(Debug, Clone, Copy)]
            pub struct Marker;

            impl Table for Marker {
                const NAME: &'static str = stringify!($table);
            }

            $(
                #[doc = concat!("Column `", stringify!($col), "` (`", stringify!($sql_ty), "`).")]
                #[derive(Debug, Clone, Copy)]
                pub struct $col;

                impl Column for $col {
                    type Table = Marker;
                    const NAME: &'static str = stringify!($col);
                }

                impl Expr for $col {
                    type SqlType = $sql_ty;
                    fn write_to(&self, sql: &mut String, _params: &mut Vec<SqlValue>) {
                        sql.push_str(stringify!($col));
                    }
                }
            )+

            /// Primary key column names. Reserved for future
            /// `INSERT`/`UPDATE`/`DELETE` builders; unused in Phase 1.
            pub const PRIMARY_KEY: &[&str] = &[$(stringify!($pk)),+];

            /// Begin a new query against this table. `SELECT *` is the
            /// default projection — call `.select(...)` to narrow it.
            pub fn table() -> Query<Marker, All<Marker>> {
                Query::new()
            }
        }
    };
}
