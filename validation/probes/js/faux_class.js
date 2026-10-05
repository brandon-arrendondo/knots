// probe: an outer function used only to declare things, the whitepaper's own JavaScript examples
// source: Cognitive 1.7 Appendix A, "JavaScript: Missing class structures": an outer function that "contain[s]
//         only declarations at the top level" is ignored, so a function nested in it starts at nesting 0
//         (total 1); one with a structural increment at its top level gets standard treatment (total 3).
//         No McCabe value is asserted (see closures.js).
// expect ns cognitive=1
// expect ns2 cognitive=3
function ns(bar, condition) {
    var foo;

    bar.myFun = function () {
        if (condition) {
            foo = 1;
        }
    };
}

function ns2(bar, condition) {
    var foo;
    if (condition) {
        foo = 2;
    }

    bar.myFun = function () {
        if (condition) {
            foo = 1;
        }
    };
}
