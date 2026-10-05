// probe: plain and labeled break and continue
// source: McCabe 1976 (unconditional transfers are not decisions); Cognitive 1.7 B1 (only break/continue to a
//         label increment): for +1, for +2, three ifs +3 each, continue outer +1, break outer +1; plain break 0
// expect f mccabe=6 cognitive=14
function f(m) {
    let t = 0;
    outer: for (let i = 0; i < m; i++) {
        for (let j = 0; j < m; j++) {
            if (j > i) continue outer;
            if (j === 7) break outer;
            if (j === 9) break;
            t++;
        }
    }
    return t;
}
