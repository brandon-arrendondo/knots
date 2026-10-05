// probe: nesting increments, the whitepaper's myMethod example without its try/catch
// source: Cognitive 1.7 section "Increments for nested flow-break structures": if +1, for +2 (nesting 1), while +3 (nesting 2)
// expect f mccabe=4 cognitive=6
void f(int c1, int c2)
{
    if (c1) {
        for (int i = 0; i < 10; i++) {
            while (c2) {
                c2--;
            }
        }
    }
}
