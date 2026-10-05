// probe: for, while and do-while
// source: McCabe 1976 (each loop test is a decision); Cognitive 1.7 B1/B3 (for +1, nested while +2, do-while +1)
// expect f mccabe=4 cognitive=4
int f(int n)
{
    int s = 0;
    for (int i = 0; i < n; i++) {
        while (s < i) {
            s++;
        }
    }
    do {
        s--;
    } while (s > 100);
    return s;
}
