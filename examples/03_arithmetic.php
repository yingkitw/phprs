<?php
// Arithmetic Operators Example
// Demonstrates basic math operations

$x = 10;
$y = 3;

echo 'x = ';
echo $x;
echo ', y = ';
echo $y;
echo "\n\n";

echo 'Addition (x + y): ';
echo $x + $y;
echo "\n";

echo 'Subtraction (x - y): ';
echo $x - $y;
echo "\n";

echo 'Multiplication (x * y): ';
echo $x * $y;
echo "\n";

echo 'Division (x / y): ';
echo $x / $y;
echo "\n";

echo 'Modulo (x % y): ';
echo $x % $y;
echo "\n";

// Parentheses and order of operations
$result = ($x + $y) * 2;
echo 'Parentheses (x + y) * 2: ';
echo $result;
echo "\n";
