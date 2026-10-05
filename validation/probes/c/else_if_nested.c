// probe: else if and else take no nesting increment
// source: Cognitive 1.7 B3 (nesting increments apply to if, not to else if or else)
//         outer if +1, inner if +2, else if +1, else +1
// expect f mccabe=4 cognitive=5
int f(int a, int b)
{
    if (a) {
        if (b) {
            return 1;
        } else if (a > b) {
            return 2;
        } else {
            return 3;
        }
    }
    return 0;
}
