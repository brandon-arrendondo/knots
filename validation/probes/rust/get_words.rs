// probe: the whitepaper's getWords, with match in place of switch
// source: Cognitive 1.7 section "Increments" worked example (cyclomatic 4, cognitive 1): a match is one
//         increment however many arms; McCabe 1976, an n-way branch is n - 1 decisions (four arms, three)
// expect get_words mccabe=4 cognitive=1
fn get_words(number: u32) -> &'static str {
    match number {
        1 => "one",
        2 => "a couple",
        3 => "a few",
        _ => "lots",
    }
}
