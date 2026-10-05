// probe: two blocks on the same macro are one choice, not two
// source: McCabe 1976 and Cognitive 1.7, per configuration (see preproc_arms.c). With FAST defined: two ifs; without
//         it: three. No build has all four, so the worst real configuration has three.
// expect g mccabe=4 cognitive=3
int g(int x)
{
#ifdef FAST
    if (x > 1)
        x = 1;
#endif
    if (x < -5)
        x = -5;
#ifndef FAST
    if (x < 0)
        x = 0;
    if (x > 9)
        x = 9;
#endif
    return x;
}
