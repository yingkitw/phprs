<?php
// Operators Example
// Demonstrates PHP operators

$a = 10;
$b = 3;

// Arithmetic operations
echo "a + b = " . ($a + $b) . "\n";
echo "a - b = " . ($a - $b) . "\n";
echo "a * b = " . ($a * $b) . "\n";
echo "a / b = " . ($a / $b) . "\n";
echo "a % b = " . ($a % $b) . "\n";

// String interpolation
$name = "PHP-RS";
echo "Hello $name!\n";

// String concatenation
echo "Result: " . ($a * $b + 2) . "\n";

// Built-in functions
echo "strlen: " . strlen("hello") . "\n";
echo strtoupper("operators work") . "\n";

// --- Compound assignment operators ---
echo "--- Compound assignment ---\n";
$total = 100;
$total += 50;   // 150
$total -= 30;   // 120
$total *= 2;    // 240
$total /= 4;    // 60
$total %= 7;    // 4
echo "total: $total\n";

$label = "count";
$label .= "-";
$label .= "down";
$dim = array('k' => 10);
$dim['k'] += 5;
echo "label: $label, dim: " . $dim['k'] . "\n";

// --- Bitwise operators ---
echo "--- Bitwise ---\n";
$a = 12; $b = 10;
echo "and: ", $a & $b, "\n";     // 8
echo "or: ", $a | $b, "\n";      // 14
echo "xor: ", $a ^ $b, "\n";     // 6
echo "not: ", ~$a, "\n";         // -13
echo "shl: ", 1 << 6, "\n";      // 64
echo "shr: ", 256 >> 4, "\n";    // 16

$flags = 0b1100;                 // binary literal
$mask = 0x0f;                    // hex literal
echo "flags&mask: ", $flags & $mask, "\n"; // 12

$perm = 6;
$perm |= 1;    // 7
$perm &= ~2;   // 5
$perm <<= 2;   // 20
echo "perm: $perm\n";
