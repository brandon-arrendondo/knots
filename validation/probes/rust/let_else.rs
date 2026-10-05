// probe: let ... else
// source: McCabe 1976: the pattern either matches or control diverges into the else block, one decision.
//         Cognitive 1.7 predates let-else and has no rule for it, so no cognitive value is asserted.
// expect first mccabe=2
fn first(v: &[i32]) -> i32 {
    let Some(x) = v.first() else {
        return 0;
    };
    *x
}
