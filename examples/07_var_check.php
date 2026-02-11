<?php
// String Functions Example
// Demonstrates strlen() and string operations

$text = 'Hello World';
echo 'String: ';
echo $text;
echo "\n";

$result = strlen($text);
echo 'Length: ';
echo $result;
echo "\n";

$empty = '';
$result = strlen($empty);
echo 'Empty string length: ';
echo $result;
echo "\n";

$php = 'PHP';
$result = strlen($php);
echo 'strlen of PHP: ';
echo $result;
echo "\n";
