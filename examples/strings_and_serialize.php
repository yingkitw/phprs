<?php
// examples/strings_and_serialize.php
// Demonstrates the PHP string helpers, base conversion, printf precision,
// fuzzy string comparison, and serialize/unserialize that phprs implements.
// Run with: cargo run -p phprs-cli -- run examples/strings_and_serialize.php

// --- String helpers ---
echo str_repeat("ab", 3) . "\n";
echo ucwords("hello world") . "\n";
echo lcfirst("HelloWorld") . "\n";
echo strrev("hello") . "\n";
echo "contains: " . (str_contains("hello world", "world") ? "yes" : "no") . "\n";
echo "starts: " . (str_starts_with("hello", "he") ? "yes" : "no") . "\n";
echo "ends: " . (str_ends_with("hello", "lo") ? "yes" : "no") . "\n";
echo strtr("Hello", "el", "ip") . "\n";          // Hippo
echo str_ireplace("WORLD", "php", "hello world") . "\n"; // hello php
echo addslashes("it's \"great\"") . "\n";
echo stripslashes("it\\'s") . "\n";
echo strip_tags("<b>hi</b>") . "\n";
echo htmlspecialchars_decode("a &amp; b") . "\n";
echo wordwrap("The quick brown fox", 10, "/", true) . "\n";
echo number_format(1234567.891, 2) . "\n";

// --- printf family with precision ---
echo sprintf("name=%s age=%d pi=%.2f hex=%x", "Bob", 42, 3.14159, 255) . "\n";
echo vsprintf("%s-%s-%s", ["a", "b", "c"]) . "\n";

// --- Base conversion + trig ---
echo decbin(10) . " " . dechex(255) . " " . decoct(8) . "\n";
echo bindec("1010") . " " . hexdec("ff") . " " . octdec("10") . "\n";
echo base_convert("ff", 16, 2) . "\n";

// --- Fuzzy comparison ---
echo "similar: " . similar_text("abcde", "abfde") . "\n";
echo "levenshtein: " . levenshtein("kitten", "sitting") . "\n";
echo "soundex: " . soundex("Robert") . " " . soundex("Rupert") . "\n";

// --- serialize / unserialize ---
$data = ["name" => "Alice", "tags" => ["a", "b"], "count" => 3];
$blob = serialize($data);
echo "serialized: " . $blob . "\n";
$back = unserialize($blob);
echo "name=" . $back["name"] . " count=" . $back["count"] . " tag0=" . $back["tags"][0] . "\n";

echo "STRINGS OK\n";
