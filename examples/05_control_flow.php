<?php
// Control Flow Example
// Demonstrates if/else and loops

// If-else statement
echo "If-else:\n";
$value = 10;
if ($value > 5) {
    echo '  Value is greater than 5';
    echo "\n";
} else {
    echo '  Value is not greater than 5';
    echo "\n";
}

// For loop
echo "\nFor loop:\n";
$i = 0;
for ($i = 0; $i < 5; $i++) {
    echo '  Iteration ';
    echo $i;
    echo "\n";
}

// While loop
echo "\nWhile loop:\n";
$j = 0;
while ($j < 3) {
    echo '  Count: ';
    echo $j;
    echo "\n";
    $j++;
}

echo "\nDone!\n";
