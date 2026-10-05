// probe: the whitepaper's sumOfPrimes, in Rust
// source: Cognitive 1.7 section "Increments" worked example (cyclomatic 4, cognitive 7): for +1, nested for +2,
//         nested if +3, continue to a label +1
// expect sum_of_primes mccabe=4 cognitive=7
fn sum_of_primes(max: u32) -> u32 {
    let mut total = 0;
    'out: for i in 1..=max {
        for j in 2..i {
            if i % j == 0 {
                continue 'out;
            }
        }
        total += i;
    }
    total
}
