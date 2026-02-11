<?php
// Functions Example
// Demonstrates PHP function definitions and calls

// Simple function
function greet($name) {
    return "Hello, $name!";
}

echo greet("PHP-RS") . "\n";

// Function with default parameter
function add($a, $b = 0) {
    return $a + $b;
}

echo "add(5, 3) = " . add(5, 3) . "\n";
echo "add(5) = " . add(5) . "\n";

// Function with type hints
function multiply(int $a, int $b): int {
    return $a * $b;
}

echo "multiply(4, 5) = " . multiply(4, 5) . "\n";

// Recursive function
function factorial($n) {
    if ($n <= 1) {
        return 1;
    }
    return $n * factorial($n - 1);
}

echo "factorial(5) = " . factorial(5) . "\n";

// Anonymous function
$square = function($x) {
    return $x * $x;
};

echo "square(6) = " . $square(6) . "\n";

