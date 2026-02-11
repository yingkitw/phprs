<?php
// Variable type checking and null handling

// Type checking - using gettype()
$var1 = "Hello";
$var2 = 42;
$var3 = null;
$var4 = [1, 2, 3];

echo "Type checking:\n";
echo "gettype(\$var1): " . gettype($var1) . "\n";
echo "gettype(\$var2): " . gettype($var2) . "\n";
echo "gettype(\$var3): " . gettype($var3) . "\n";
echo "gettype(\$var4): " . gettype($var4) . "\n";

// isset() for null coalescing pattern
echo "\nisset() for default values:\n";
$x = null;
if (isset($x)) {
    echo "\$x is set: " . $x . "\n";
} else {
    echo "\$x is not set, using default\n";
}

$y = "actual value";
if (isset($y)) {
    echo "\$y is set: " . $y . "\n";
} else {
    echo "\$y is not set, using default\n";
}

// Using isset with arrays for safe access
$user = [
    "name" => "John",
    "email" => null
];

echo "\nArray access with isset():\n";
if (isset($user['email'])) {
    echo "Email: " . $user['email'] . "\n";
} else {
    echo "Email: not provided\n";
}

if (isset($user['phone'])) {
    echo "Phone: " . $user['phone'] . "\n";
} else {
    echo "Phone: not provided\n";
}

