<?php
// Error Handling Example
// Demonstrates PHP error handling system

// Set error handler
set_error_handler(function($errno, $errstr, $errfile, $errline) {
    echo "[Custom Handler] Error $errno: $errstr in $errfile on line $errline\n";
    return true;
});

// Trigger errors
echo "Triggering a warning:\n";
trigger_error("This is a custom warning!", E_USER_WARNING);

echo "\nTriggering a notice:\n";
trigger_error("This is a custom notice!", E_USER_NOTICE);

// Restore default error handler
restore_error_handler();

echo "\nTriggering a default error (should print to stderr):\n";
trigger_error("This is a default error!", E_USER_ERROR);

