// Example JavaScript file for knots complexity analysis
// Run: knots simple.js

function add(a, b) {
    return a + b;
}

function classify(value) {
    if (value < 0) {
        return "negative";
    } else if (value === 0) {
        return "zero";
    } else {
        return "positive";
    }
}

function sumEvens(numbers) {
    let total = 0;
    for (const n of numbers) {
        if (n % 2 === 0) {
            total += n;
        }
    }
    return total;
}
