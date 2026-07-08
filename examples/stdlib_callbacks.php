<?php
// examples/stdlib_callbacks.php
// Demonstrates callback-driven builtins, new array helpers, and the Reflection
// API surface that phprs implements. Run with:
//   cargo run -p phprs-cli -- run examples/stdlib_callbacks.php

// --- Callback-driven array functions with a user function ---
function double(int $n): int {
    return $n + $n;
}

$nums = [1, 2, 3, 4];
$doubled = array_map('double', $nums);
echo "map: " . implode(",", $doubled) . "\n";

$sum = array_reduce($nums, 'double', 0);
echo "reduce: " . $sum . "\n";

// Default filter drops falsy values (0 and "" and "0").
$mixed = [0, 1, 2, 0, 3];
echo "filter: " . implode(",", array_filter($mixed)) . "\n";

// call_user_func / call_user_func_array
echo "cuf: " . call_user_func('strtoupper', 'abc') . "\n";
echo "cufa: " . call_user_func_array('implode', ['-', ['a', 'b', 'c']]) . "\n";

// --- New array helpers ---
echo "combine: " . implode("/", array_combine(['a', 'b'], [1, 2])) . "\n";
$flipped = array_flip(['x' => 'one', 'y' => 'two']);
echo "flip: " . $flipped['one'] . "\n";
echo "search: " . array_search('green', ['red', 'green', 'blue']) . "\n";
echo "unique: " . implode(",", array_unique(['a', 'b', 'a', 'c'])) . "\n";
echo "sum: " . array_sum([1, 2, 3, 4]) . "\n";
echo "product: " . array_product([1, 2, 3, 4]) . "\n";
echo "diff: " . implode(",", array_diff(['a', 'b', 'c'], ['b'])) . "\n";
echo "intersect: " . implode(",", array_intersect(['a', 'b', 'c'], ['b', 'c'])) . "\n";
echo "range: " . implode(",", range(1, 5)) . "\n";
echo "fill: " . implode(",", array_fill(0, 3, 'x')) . "\n";
echo "pad: " . implode(",", array_pad([1, 2], 4, 0)) . "\n";
$counts = array_count_values(['a', 'b', 'a']);
echo "counts: " . $counts['a'] . "\n";

// --- String helpers ---
echo "substr_count: " . substr_count('aabbccaa', 'aa') . "\n";
echo "substr_replace: " . substr_replace('Hello World', 'PHP', 0, 5) . "\n";
echo "strpbrk: " . strpbrk('hello', 'oe') . "\n";

// --- Math helpers ---
echo "intdiv: " . intdiv(17, 5) . "\n";
echo "fmod: " . fmod(17, 5) . "\n";
echo "hypot: " . hypot(3, 4) . "\n";
echo "is_numeric('123'): " . (is_numeric('123') ? 'yes' : 'no') . "\n";

// --- Reflection API ---
function described($alpha, $beta) {
    return $alpha;
}

$rf = new ReflectionFunction('described');
echo "reflect_fn: " . $rf->getName() . " nparams=" . $rf->getNumberOfParameters() . "\n";
$rp = new ReflectionParameter('described', 'beta');
echo "reflect_param: " . $rp->getName() . " pos=" . $rp->getPosition() . "\n";

echo "STDLIB OK\n";
