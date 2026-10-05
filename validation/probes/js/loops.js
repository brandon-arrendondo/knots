// probe: for, for...in, for...of, while and do...while
// source: McCabe 1976 (each loop test is a decision); Cognitive 1.7 B1/B3 (for and foreach +1, nested +2,
//         while +1, do while +1)
// expect f mccabe=6 cognitive=6
function f(n, obj, xs) {
    let s = 0;
    for (let i = 0; i < n; i++) {
        s += i;
    }
    for (const k in obj) {
        for (const x of xs) {
            s += x;
        }
    }
    while (s > 100) {
        s--;
    }
    do {
        s++;
    } while (s < 0);
    return s;
}
