//! Inner join and group join on small tabular data.
//!
//! Run with: `cargo run --example join`

use linq_rs::LinqExt;

#[derive(Debug, Clone)]
struct Customer {
    id: u32,
    name: &'static str,
}

#[derive(Debug, Clone)]
struct Order {
    customer_id: u32,
    product: &'static str,
}

fn main() {
    let customers = vec![
        Customer {
            id: 1,
            name: "Alice",
        },
        Customer { id: 2, name: "Bob" },
        Customer {
            id: 3,
            name: "Carol",
        },
    ];

    let orders = vec![
        Order {
            customer_id: 1,
            product: "Laptop",
        },
        Order {
            customer_id: 1,
            product: "Mouse",
        },
        Order {
            customer_id: 2,
            product: "Keyboard",
        },
    ];

    // Inner join — Carol gets no rows because she has no orders.
    println!("Inner join:");
    let rows: Vec<String> = customers
        .clone()
        .into_iter()
        .join_hashed(
            orders.clone(),
            |c| c.id,
            |o| o.customer_id,
            |c, o| format!("  {} bought {}", c.name, o.product),
        )
        .collect();
    for row in &rows {
        println!("{row}");
    }

    // Group join — Carol appears with an empty member list (left outer).
    println!("\nGroup join (left outer):");
    let grouped: Vec<String> = customers
        .into_iter()
        .group_join_hashed(
            orders,
            |c| c.id,
            |o| o.customer_id,
            |c, ords| {
                let products: Vec<_> = ords.into_iter().map(|o| o.product).collect();
                if products.is_empty() {
                    format!("  {}: (no orders)", c.name)
                } else {
                    format!("  {}: {}", c.name, products.join(", "))
                }
            },
        )
        .collect();
    for row in &grouped {
        println!("{row}");
    }
}
