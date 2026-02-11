<?php
// Array and Count Example
// Demonstrates array operations and count()

// Creating arrays with []
$fruits = ['apple', 'banana', 'cherry'];
echo 'Fruits array created' . "\n";

// Access array elements
$first = $fruits[0];
echo 'First fruit: ';
echo $first;
echo "\n";

$second = $fruits[1];
echo 'Second fruit: ';
echo $second;
echo "\n";

// Count elements
$result = count($fruits);
echo 'Count: ';
echo $result;
echo "\n";

// Empty array
$empty = [];
$result = count($empty);
echo 'Empty array count: ';
echo $result;
echo "\n";

// Array with mixed types
$numbers = [1, 2, 3, 4, 5];
$result = count($numbers);
echo 'Numbers array count: ';
echo $result;
echo "\n";
