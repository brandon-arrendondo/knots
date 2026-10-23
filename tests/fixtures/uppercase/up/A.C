#include "B.H"

int uses_helper(int x)
{
    if (x > 0) {
        return helper(x);
    }
    return 0;
}
