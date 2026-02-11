<?php
// String Operations Example
// Demonstrates PHP string handling and operations

$str1 = "Hello";
$str2 = "World";
$str3 = "!";

echo "String 1: $str1\n";
echo "String 2: $str2\n";
echo "String 3: $str3\n";

// Concatenation
$concat2 = $str1 . $str2;
echo "Concatenation (2 strings): $concat2\n";

$concat3 = $str1 . $str2 . $str3;
echo "Concatenation (3 strings): $concat3\n";

// String length
echo "Length of '$str1': " . strlen($str1) . "\n";

// String comparison
if ($str1 === "Hello") {
    echo "String comparison works\n";
}

