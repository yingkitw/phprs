<?php
// Filesystem Operations Example
// Demonstrates PHP filesystem functions

// Check if current directory exists
echo "Does '.' exist? " . (file_exists('.') ? 'yes' : 'no') . "\n";
echo "Is '.' a directory? " . (is_dir('.') ? 'yes' : 'no') . "\n";
echo "Is '.' a file? " . (is_file('.') ? 'yes' : 'no') . "\n";

// Check if a file exists
$test_file = __FILE__;
echo "\nDoes '$test_file' exist? " . (file_exists($test_file) ? 'yes' : 'no') . "\n";
echo "Is '$test_file' a directory? " . (is_dir($test_file) ? 'yes' : 'no') . "\n";
echo "Is '$test_file' a file? " . (is_file($test_file) ? 'yes' : 'no') . "\n";

// Get file size
if (file_exists($test_file)) {
    $size = filesize($test_file);
    echo "Filesize of '$test_file': $size bytes\n";
}

// Get file contents
if (file_exists($test_file)) {
    $contents = file_get_contents($test_file);
    echo "Contents length: " . strlen($contents) . " bytes\n";
}

// Scan directory
$entries = scandir('.');
echo "\nEntries in current directory:\n";
foreach ($entries as $entry) {
    if ($entry !== '.' && $entry !== '..') {
        echo "  - $entry\n";
    }
}

