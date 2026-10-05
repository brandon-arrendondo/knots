// probe: recursion, direct and indirect
// source: Cognitive 1.7 section "Recursion": +1 for each method in a recursion cycle, direct or indirect, once per method
// expect fact mccabe=2 cognitive=2
// expect fib mccabe=2 cognitive=2
// expect is_even mccabe=2 cognitive=2
// expect is_odd mccabe=2 cognitive=2
// expect leaf mccabe=1 cognitive=0
int is_odd(int n);

int fact(int n)
{
    if (n <= 1)
        return 1;
    return n * fact(n - 1);
}

int fib(int n)
{
    if (n < 2)
        return n;
    return fib(n - 1) + fib(n - 2);
}

int is_even(int n)
{
    if (n == 0)
        return 1;
    return is_odd(n - 1);
}

int is_odd(int n)
{
    if (n == 0)
        return 0;
    return is_even(n - 1);
}

int leaf(int n)
{
    return is_even(n);
}
