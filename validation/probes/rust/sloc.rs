// probe: source lines of code
// source: Park 1992, CMU/SEI-92-TR-20: physical lines, excluding blank lines and comment-only lines (every line
//         of a block comment is comment-only); a line with code and a trailing comment is a code line
// expect f sloc=4
fn f(x: i32) -> i32 {
    /* a comment-only line */

    let y = x; /* a trailing comment */
    // another comment-only line
    /*
       a block comment
       over several lines
    */
    y
}
