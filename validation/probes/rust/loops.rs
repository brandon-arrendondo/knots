// probe: for, while and while let
// source: McCabe 1976 (each loop test is a decision; a while let tests its pattern each iteration);
//         Cognitive 1.7 B1/B3 (for +1, nested while +2, while let +1)
// expect f mccabe=4 cognitive=4
fn f(n: u32, mut v: Vec<u32>) -> u32 {
    let mut s = 0;
    for i in 0..n {
        while s < i {
            s += 1;
        }
    }
    while let Some(x) = v.pop() {
        s += x;
    }
    s
}
