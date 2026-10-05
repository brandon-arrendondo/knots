// probe: source lines of code
// source: Park 1992, CMU/SEI-92-TR-20: physical lines, excluding blank lines and comment-only lines;
//         a line with code and a trailing comment is a code line
// expect f sloc=4
function f(x) {
    /* a comment-only line */

    const y = x; /* a trailing comment */
    // another comment-only line
    return y;
}
