// probe: a function with no decisions
// source: McCabe 1976 (v(G) = decisions + 1); Cognitive 1.7 (nothing to increment)
// expect f mccabe=1 cognitive=0
function f(x) {
    const y = x + 1;
    return y * 2;
}
