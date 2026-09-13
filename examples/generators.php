<?php
// Generators Example
// Demonstrates PHP generator functions with yield

function fibonacci() {
    yield 0;
    yield 1;
    yield 1;
    yield 2;
    yield 3;
    yield 5;
    yield 8;
}

$fib = fibonacci();
$fib->rewind();
while ($fib->valid()) {
    echo $fib->current() . " ";
    $fib->next();
}
echo "\n";

function range_gen() {
    yield 10;
    yield 20;
    yield 30;
}

$r = range_gen();
$r->rewind();
while ($r->valid()) {
    echo $r->current() . " ";
    $r->next();
}
echo "\n";
