// probe: nesting, the whitepaper's sumOfPrimes example in JavaScript (labeled continue, as in its Java original)
// source: Cognitive 1.7 gives sumOfPrimes Cyclomatic Complexity 4 and Cognitive Complexity 7
//         (+1 outer for, +2 inner for, +3 if, +1 continue to a label)
// expect sumOfPrimes mccabe=4 cognitive=7
function sumOfPrimes(max) {
    let total = 0;
    OUT: for (let i = 1; i <= max; ++i) {
        for (let j = 2; j < i; ++j) {
            if (i % j === 0) {
                continue OUT;
            }
        }
        total += i;
    }
    return total;
}
