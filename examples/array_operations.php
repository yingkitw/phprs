<?php
// Array Operations Example
// Demonstrates PHP array/hash table operations

$arr = [];

// Add elements
$arr['name'] = 'PHP-RS';
$arr['version'] = 1;
$arr['pi'] = 3.14;

echo "Array size: " . count($arr) . "\n";

// Access elements
echo "Name: " . $arr['name'] . "\n";
echo "Version: " . $arr['version'] . "\n";
echo "Pi: " . $arr['pi'] . "\n";

// Numeric keys
$arr[0] = 'first';
$arr[1] = 'second';
echo "Index 0: " . $arr[0] . "\n";

// Iterate (value-only; key => value foreach is not implemented yet in phprs)
foreach ($arr as $value) {
    echo "item: $value\n";
}

