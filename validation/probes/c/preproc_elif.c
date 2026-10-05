// probe: an #if / #elif / #else chain
// source: McCabe 1976 and Cognitive 1.7, per configuration (see preproc_arms.c). The three arms have two ifs, one
//         and none; the worst real configuration has two.
// expect k mccabe=3 cognitive=2
int k(int x)
{
#if LEVEL > 2
    if (x > 3)
        x = 3;
    if (x < -3)
        x = -3;
#elif LEVEL > 1
    if (x > 2)
        x = 2;
#else
    x = 0;
#endif
    return x;
}
