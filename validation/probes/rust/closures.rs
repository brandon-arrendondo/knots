// probe: a closure nests what is inside it
// source: Cognitive 1.7 B3 (nested methods and lambdas increase the nesting level, with no increment of their
//         own): if inside the closure +2, else +1. Whether McCabe counts a closure's decisions in the enclosing
//         function depends on whether the closure is its own flow graph, so no McCabe value is asserted.
// expect f cognitive=3
fn f(v: &[i32]) -> usize {
    v.iter()
        .filter(|x| {
            if **x > 0 {
                true
            } else {
                false
            }
        })
        .count()
}
