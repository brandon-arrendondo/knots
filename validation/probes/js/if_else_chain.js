// probe: if / else if / else
// source: McCabe 1976 (two decisions); Cognitive 1.7 B1 (if +1, else if +1, else +1; hybrid, no nesting increment)
// expect f mccabe=3 cognitive=3
function f(x) {
    if (x > 0) {
        return 1;
    } else if (x < 0) {
        return -1;
    } else {
        return 0;
    }
}
