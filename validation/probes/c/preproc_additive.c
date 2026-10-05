// probe: an #ifdef with no #else adds code to one configuration
// source: McCabe 1976 and Cognitive 1.7, per configuration (see preproc_arms.c). With EXTRA_CHECKS defined the
//         function has two ifs, without it one; the worst real configuration has two.
// expect h mccabe=3 cognitive=2
int h(int x)
{
    if (x < 0)
        x = 0;
#ifdef EXTRA_CHECKS
    if (x > 100)
        x = 100;
#endif
    return x;
}
