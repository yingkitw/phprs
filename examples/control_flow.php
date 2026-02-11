<?php
// Control Flow Example
// Demonstrates PHP control flow structures

// If-else
$value = 10;
if ($value > 5) {
    echo "Value is greater than 5\n";
} else {
    echo "Value is not greater than 5\n";
}

// Switch
$day = "Monday";
switch ($day) {
    case "Monday":
        echo "Start of the week\n";
        break;
    case "Friday":
        echo "End of the week\n";
        break;
    default:
        echo "Midweek\n";
}

// For loop
echo "\nFor loop:\n";
for ($i = 0; $i < 5; $i++) {
    echo "  Iteration $i\n";
}

// While loop
echo "\nWhile loop:\n";
$j = 0;
while ($j < 3) {
    echo "  Count: $j\n";
    $j++;
}

// Foreach loop
echo "\nForeach loop:\n";
$items = ['apple', 'banana', 'cherry'];
foreach ($items as $item) {
    echo "  Item: $item\n";
}

