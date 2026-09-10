use linq_rs::LinqExt; // the in-memory vocabulary
use linq_rs_sql::rows::query; // the seam
use linq_rs_sql::{ExprExt, IntOps, TextOps}; // the SQL vocabulary

use linq_rs_sql::table;

table! {
    employees (id) {
        id     -> Integer,
        name   -> Text,
        dept   -> Text,
        salary -> Integer,
        remote -> Boolean,
    }
}

#[derive(Debug, PartialEq)]
pub struct Employee {
    pub id: i64,
    pub name: String,
    pub dept: String,
    pub salary: i64,
    pub remote: bool,
}

linq_rs_sql::entity! {
    Employee => employees {
        id:     Integer = id,
        name:   Text    = name,
        dept:   Text    = dept,
        salary: Integer = salary,
        remote: Boolean = remote,
    }
}

pub fn staff() -> Vec<Employee> {
    fn e(id: i64, name: &str, dept: &str, salary: i64, remote: bool) -> Employee {
        Employee {
            id,
            name: name.into(),
            dept: dept.into(),
            salary,
            remote,
        }
    }
    vec![
        e(1, "Ada", "eng", 180_000, true),
        e(2, "Brent", "sales", 90_000, false),
        e(3, "Cora", "eng", 150_000, false),
        e(4, "Dev", "eng", 210_000, true),
        e(5, "Eve", "sales", 120_000, true),
    ]
}

fn main() {
    // ── ONE VALUE ────────────────────────────────────────────────────────
    let q = query::<Employee>()
        .filter(employees::salary.gt(100_000i64))
        .filter(employees::dept.eq("eng"))
        .order_by(employees::dept)
        .order_by_desc(employees::salary)
        .limit(2);

    println!("query value type:\n  {}\n", short(tn(&q)));

    // ── INTERPRETER ONE: SQL ─────────────────────────────────────────────
    let rendered = q.to_sql();
    println!("to_sql()   sql    = {}", rendered.sql);
    println!("to_sql()   params = {:?}\n", rendered.params);

    // ── INTERPRETER TWO: the same value, over a Vec ───────────────────────
    let people = staff();
    let hits: Vec<&Employee> = q.to_memory_sorted(&people).collect();
    println!("to_memory_sorted(&people):");
    for e in &hits {
        println!("    {:>2}  {:<6} {:<6} {}", e.id, e.name, e.dept, e.salary);
    }

    // ── D-102: crossing the boundary restores the full LinqExt surface ────
    let q2 = query::<Employee>().filter(employees::name.like("_o%"));
    let names: Vec<String> = q2
        .to_memory(&people) // <- the one visible token
        .select(|e| e.name.to_uppercase()) // LinqExt::select — not translatable
        .order_by(|s| s.clone()) // LinqExt::order_by — not translatable
        .collect();
    println!("\nafter .to_memory(): LIKE '_o%' -> {:?}", names);
}

// `Rows<Employee, linq_rs_sql::expr::Eq<..>, ..>` is long; trim the paths so the
// printed type reads as the query it describes.
fn short(s: &str) -> String {
    s.replace("linq_rs_sql::expr::", "")
        .replace("linq_rs_sql::rows::", "")
        .replace("employees::", "")
        .replace("seam::", "")
}

fn tn<T>(_: &T) -> &'static str {
    std::any::type_name::<T>()
}
