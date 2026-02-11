<?php
// String functions: strlen(), strpos(), strtolower(), strtoupper()

// strlen() - get string length
$text = "Hello World";
$result = strlen($text);
echo "strlen(\"Hello World\"): " . $result . "\n";

// strpos() - find position of substring
$result = strpos("Hello World", "World");
echo "strpos(\"Hello World\", \"World\"): " . $result . "\n";

$result = strpos("Hello World", "xyz");
echo "strpos(\"Hello World\", \"xyz\"): " . $result . "\n";

// strtolower() - convert to lowercase
$result = strtolower("HELLO WORLD");
echo "strtolower(\"HELLO WORLD\"): " . $result . "\n";

// strtoupper() - convert to uppercase
$result = strtoupper("hello world");
echo "strtoupper(\"hello world\"): " . $result . "\n";

// trim() - remove whitespace
$result = trim("  hello  ");
echo "trim(\"  hello  \"): \"" . $result . "\"\n";

// str_replace() - replace substrings
$result = str_replace("World", "PHP", "Hello World");
echo "str_replace(\"World\", \"PHP\", \"Hello World\"): " . $result . "\n";
