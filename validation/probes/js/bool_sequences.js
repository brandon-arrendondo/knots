// probe: boolean operators in conditions
// source: Cognitive 1.7 section "Sequences of logical operators" (its two worked examples, scores 4 and 3);
//         McCabe extended to conditions (Myers 1977): each && and || is a decision
// expect mixed mccabe=7 cognitive=4
// expect negated mccabe=4 cognitive=3
function mixed(a, b, c, d, e, g) {
    if (a && b && c || d || e && g) {
        return 1;
    }
    return 0;
}

function negated(a, b, c) {
    if (a && !(b && c)) {
        return 1;
    }
    return 0;
}
