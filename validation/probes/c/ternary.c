// probe: the conditional operator
// source: McCabe 1976 (a two-way decision); Cognitive 1.7 B1 (ternary +1)
// expect f mccabe=2 cognitive=1
int f(int x)
{
    return x > 0 ? 1 : 0;
}
