// probe: the ? operator
// source: McCabe 1976: each ? tests its operand and either continues or returns early, one decision each;
//         Cognitive 1.7 "Ignore shorthand" (operators that shorten a check, like null-coalescing, add nothing;
//         an early return adds nothing)
// expect parse_two mccabe=3 cognitive=0
fn parse_two(a: &str, b: &str) -> Result<i32, std::num::ParseIntError> {
    let x: i32 = a.parse()?;
    let y: i32 = b.parse()?;
    Ok(x + y)
}
