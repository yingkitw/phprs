<?php
// Type Checking Example
// Demonstrates type checking functions

// is_int() - check if value is integer
$int_val = 42;
$result = is_int($int_val);
echo 'is_int(42): ';
echo $result;
echo "\n";

$str_val = 'hello';
$result = is_int($str_val);
echo 'is_int("hello"): ';
echo $result;
echo "\n";

// is_string() - check if value is string
$result = is_string($str_val);
echo 'is_string("hello"): ';
echo $result;
echo "\n";

$result = is_string($int_val);
echo 'is_string(42): ';
echo $result;
echo "\n";

// is_null() - check if value is null
$null_val = null;
$result = is_null($null_val);
echo 'is_null(null): ';
echo $result;
echo "\n";

$result = is_null($int_val);
echo 'is_null(42): ';
echo $result;
echo "\n";

// is_bool() - check if value is boolean
$bool_val = true;
$result = is_bool($bool_val);
echo 'is_bool(true): ';
echo $result;
echo "\n";
