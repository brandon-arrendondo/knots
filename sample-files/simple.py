# Example Python file for knots complexity analysis
# Run: knots simple.py


def add(a, b):
    return a + b


def classify(value):
    if value < 0:
        return "negative"
    elif value == 0:
        return "zero"
    else:
        return "positive"


def sum_evens(numbers):
    total = 0
    for n in numbers:
        if n % 2 == 0:
            total += n
    return total
