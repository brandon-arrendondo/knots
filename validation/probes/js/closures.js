// probe: an arrow function nests what is inside it
// source: Cognitive 1.7 B2 (nested methods and lambdas increase the nesting level, with no increment of their
//         own): if inside the arrow +2, else +1. Whether McCabe counts a callback's decisions in the enclosing
//         function depends on whether the callback is its own flow graph, so no McCabe value is asserted.
// expect f cognitive=3
function f(xs) {
    return xs.filter((x) => {
        if (x > 0) {
            return true;
        } else {
            return false;
        }
    }).length;
}
