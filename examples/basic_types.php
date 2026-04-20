<?php
// Basic types example (PHP) — mirrors examples/rust/basic_types.rs

$long_val = 42;
$double_val = 3.14159;
$str_val = "Hello, phprs!";
$true_val = true;
$false_val = false;
$null_val = null;

echo "Long value: $long_val\n";
echo "Double value: $double_val\n";
echo "String value: $str_val\n";
echo "True value: " . ($true_val ? 'true' : 'false') . "\n";
echo "False value: " . ($false_val ? 'true' : 'false') . "\n";
echo "Null is null: " . (is_null($null_val) ? 'yes' : 'no') . "\n";
