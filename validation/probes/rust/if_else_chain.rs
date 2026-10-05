// probe: if / else if / else, and the same chain with if let
// source: McCabe 1976 (each if is one decision; else is not); Cognitive 1.7 B1/B2 (if +1, else if +1, else +1,
//         no nesting increment for else if or else); an if let is an if whose test is a pattern match
// expect f mccabe=3 cognitive=3
// expect g mccabe=3 cognitive=3
fn f(x: i32) -> i32 {
    if x < 0 {
        -1
    } else if x == 0 {
        0
    } else {
        1
    }
}

fn g(o: Option<i32>, p: Option<i32>) -> i32 {
    if let Some(x) = o {
        x
    } else if let Some(y) = p {
        y
    } else {
        0
    }
}
