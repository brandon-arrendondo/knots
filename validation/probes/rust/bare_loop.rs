// probe: a bare loop whose only exit is a conditional break
// source: McCabe 1976, v(G) = E - N + 2 on the control-flow graph: `loop` has no predicate, the if is the one
//         decision (as in ada/exit_when.adb); Cognitive 1.7 B1/B3 (loop +1, nested if +2)
// expect f mccabe=2 cognitive=3
fn f(mut n: u32) -> u32 {
    loop {
        n += 1;
        if n > 10 {
            break;
        }
    }
    n
}
