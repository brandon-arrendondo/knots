// probe: a match arm guard
// source: The Rust Reference, "Match expressions": arms are tried in order, and a guard is tested after its
//         pattern matches, so three arms are two tests and the guard is one more condition (Myers 1977);
//         McCabe 1976, v(G) = decisions + 1. Cognitive 1.7 is silent on guards, so no cognitive value is asserted.
// expect classify mccabe=4
fn classify(o: Option<i32>) -> i32 {
    match o {
        Some(n) if n < 0 => -1,
        Some(_) => 1,
        None => 0,
    }
}
