# Derived-documentation data

Inputs to `.github/scripts/gen-docs.py`, which generates the tables in the
project `README.md` and fails CI if the committed output has drifted. This is
`DECISIONS.md` **D-016**'s gate.

The split matters:

* These files are **data** — curated by a human, because C#'s API surface cannot
  be derived from this repository.
* The **linq_rs surface is derived** from `src/` on every run.
* The **gate** is the cross-check between them. A method renamed in `src/`
  without updating `operator-map.tsv` fails the build; so does a map row naming
  a method that no longer exists, or a C# name absent from the inventory.
* **Every count is computed**, never written. Three separate hand-written counts
  in this project's README have been wrong, one of them inside the fix for the
  previous one.

## `csharp-operators.tsv`

Every distinct `System.Linq.Enumerable` method name, from
<https://learn.microsoft.com/en-us/dotnet/api/system.linq.enumerable>.

`dotnet_added` is what makes per-version counts derivable, and it matters: the
docs page ships every version's rows in one HTML table and filters client-side
via `data-moniker`, so a naive row count returns the **.NET 11 preview**
superset. Measured:

| moniker | names | overloads | comparer overloads |
|---|---|---|---|
| net-8.0 | 66 | 216 | 33 |
| net-9.0 | 69 | 220 | 36 |
| net-10.0 | 74 | 228 | 38 |
| net-11.0 (preview) | 75 | 234 | 44 |

`TARGET_DOTNET` in the generator picks which column the README quotes.

## `operator-map.tsv`

One row per public linq_rs item. `csharp` is empty where there is genuinely no
correspondence — `for_each_` and `is_empty_` are the two, since `ForEach` is
`List<T>.ForEach` and `!Any()` is an expression, not an operator.
`std_equivalent` is the load-bearing column: it is what lets the generated table
tell a reader to use `std` instead.
