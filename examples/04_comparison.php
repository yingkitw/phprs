<?php
// Comparison Operators Example
// Demonstrates comparing values

$a = 10;
$b = 20;
$c = 10;

echo 'a = ';
echo $a;
echo ', b = ';
echo $b;
echo ', c = ';
echo $c;
echo "\n\n";

// Equal
echo 'a == c: ';
if ($a == $c) {
    echo 'true';
} else {
    echo 'false';
}
echo "\n";

// Not equal
echo 'a != b: ';
if ($a != $b) {
    echo 'true';
} else {
    echo 'false';
}
echo "\n";

// Greater than
echo 'b > a: ';
if ($b > $a) {
    echo 'true';
} else {
    echo 'false';
}
echo "\n";

// Less than
echo 'a < b: ';
if ($a < $b) {
    echo 'true';
} else {
    echo 'false';
}
echo "\n";

// Greater than or equal
echo 'a >= c: ';
if ($a >= $c) {
    echo 'true';
} else {
    echo 'false';
}
echo "\n";

// Less than or equal
echo 'a <= b: ';
if ($a <= $b) {
    echo 'true';
} else {
    echo 'false';
}
echo "\n";
