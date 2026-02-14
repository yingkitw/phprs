<?php
// Match Expression Example
// Demonstrates PHP 8.0 match expressions

$status = 200;
$text = match($status) {
    200 => 'OK',
    301 => 'Moved',
    404 => 'Not Found',
    500 => 'Server Error',
    default => 'Unknown',
};
echo $text;
echo "\n";

// Match with arithmetic
$val = 3;
$label = match($val * 2) {
    2 => 'two',
    4 => 'four',
    6 => 'six',
    default => 'other',
};
echo $label;
echo "\n";
