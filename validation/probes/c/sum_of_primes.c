// probe: nesting, the whitepaper's sumOfPrimes example translated to C (its labeled `continue OUT` becomes goto)
// source: Cognitive 1.7 gives sumOfPrimes Cyclomatic Complexity 4 and Cognitive Complexity 7
//         (+1 outer for, +2 inner for, +3 if, +1 jump to a label)
// expect sumOfPrimes mccabe=4 cognitive=7
int sumOfPrimes(int max)
{
    int total = 0;
    for (int i = 1; i <= max; ++i) {
        for (int j = 2; j < i; ++j) {
            if (i % j == 0) {
                goto next;
            }
        }
        total += i;
    next:;
    }
    return total;
}
