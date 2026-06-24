// Example Rust file for knots complexity analysis
// Run: knots simple.rs

fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn classify(value: i32) -> &'static str {
    if value < 0 {
        "negative"
    } else if value == 0 {
        "zero"
    } else {
        "positive"
    }
}

fn sum_evens(numbers: &[i32]) -> i32 {
    let mut total = 0;
    for n in numbers {
        if n % 2 == 0 {
            total += n;
        }
    }
    total
}
