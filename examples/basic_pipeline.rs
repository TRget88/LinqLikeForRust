//! A basic LINQ-style pipeline over a small dataset.
//!
//! Run with: `cargo run --example basic_pipeline`

use linq_rs::LinqExt;

fn main() {
    let numbers = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

    // Square the even numbers, then keep only the first three.
    let result: Vec<_> = numbers
        .iter()
        .copied()
        .where_(|x| x % 2 == 0)
        .select(|x| x * x)
        .take_(3)
        .to_vec();

    println!("First three even squares: {result:?}");
    assert_eq!(result, [4, 16, 36]);

    // Sum the same projection lazily.
    let total: i32 = numbers.iter().copied().where_(|x| x % 2 == 0).sum_();
    println!("Sum of evens: {total}");
}
