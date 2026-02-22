<?php
// Load wp-config.php (defines ABSPATH, DB constants, etc.)
if (file_exists('wp-config.php')) {
    require 'wp-config.php';
} else {
    echo "wp-config.php not found.\n";
    exit(1);
}
