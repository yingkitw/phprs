<?php
// Multibyte string functions test

echo "Testing mb_strlen:\n";
echo "ASCII: " . mb_strlen("hello world") . "\n";
echo "Unicode: " . mb_strlen("hello 世界") . "\n";
echo "Emoji: " . mb_strlen("Hello 👋 World") . "\n\n";

echo "Testing mb_substr:\n";
echo "Substring: " . mb_substr("hello world", 6) . "\n";
echo "Unicode substring: " . mb_substr("hello 世界", 6) . "\n";
echo "With length: " . mb_substr("hello world", 0, 5) . "\n\n";

echo "Testing mb_strtolower and mb_strtoupper:\n";
echo "Lower: " . mb_strtolower("HELLO WORLD") . "\n";
echo "Upper: " . mb_strtoupper("hello world") . "\n";
echo "Unicode lower: " . mb_strtolower("HELLO 世界") . "\n";
echo "Unicode upper: " . mb_strtoupper("hello 世界") . "\n\n";

echo "Testing mb_strpos:\n";
echo "Position: " . mb_strpos("hello world", "world") . "\n";
echo "Not found: ";
$pos = mb_strpos("hello world", "xyz");
echo ($pos === false ? "false" : $pos) . "\n\n";

echo "Testing mb_strrpos:\n";
echo "Last position: " . mb_strrpos("hello world world", "world") . "\n\n";

echo "Testing mb_substr_count:\n";
echo "Count: " . mb_substr_count("hello world world", "world") . "\n\n";

echo "Testing mb_strwidth:\n";
echo "Width: " . mb_strwidth("hello") . "\n";
echo "Unicode width: " . mb_strwidth("hello 世界") . "\n\n";

echo "Testing mb_strimwidth:\n";
echo "Truncated: " . mb_strimwidth("hello world", 0, 5, "...") . "\n";
echo "Unicode truncated: " . mb_strimwidth("hello 世界", 0, 7, "...") . "\n";
