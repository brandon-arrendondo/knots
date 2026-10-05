// probe: boolean operators in conditions
// source: Cognitive 1.7 section "Sequences of logical operators" (its two worked examples, scores 4 and 3);
//         McCabe extended to conditions (Myers 1977): each && and || is a decision
// expect mixed mccabe=7 cognitive=4
// expect negated mccabe=4 cognitive=3
fn mixed(a: bool, b: bool, c: bool, d: bool, e: bool, g: bool) -> i32 {
    if a && b && c || d || e && g {
        return 1;
    }
    0
}

fn negated(a: bool, b: bool, c: bool) -> i32 {
    if a && !(b && c) {
        return 1;
    }
    0
}
