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
echo $fib[0];
echo "\n";
echo $fib[3];
echo "\n";
echo $fib[6];
echo "\n";

function range_gen() {
    yield 10;
    yield 20;
    yield 30;
}

$r = range_gen();
echo $r[1];
echo "\n";
