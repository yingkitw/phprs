<?php
// Variables Example
// Demonstrates PHP variable handling

// Register variables
$counter = 42;
$name = "PHP-RS";
$pi = 3.14159;
$active = true;

echo "Counter: $counter\n";
echo "Name: $name\n";
echo "Pi: $pi\n";
echo "Active: " . ($active ? 'true' : 'false') . "\n";

// Type checking
echo "\nType checking:\n";
echo "counter is int: " . (is_int($counter) ? 'yes' : 'no') . "\n";
echo "name is string: " . (is_string($name) ? 'yes' : 'no') . "\n";
echo "pi is float: " . (is_float($pi) ? 'yes' : 'no') . "\n";
echo "active is bool: " . (is_bool($active) ? 'yes' : 'no') . "\n";

