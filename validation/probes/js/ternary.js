// probe: nested conditional operators
// source: McCabe 1976 (each ?: is a two-way decision); Cognitive 1.7 B1-B3 (ternary +1 and nests: inner +2)
// expect f mccabe=3 cognitive=3
function f(a, b) {
    return a ? (b ? 1 : 2) : 3;
}
