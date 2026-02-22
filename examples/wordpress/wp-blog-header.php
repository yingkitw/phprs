<?php
// Load WordPress and run main request
require 'wp-load.php';
require 'wp-settings.php';
echo "Hello from WordPress (phprs)!\n";
echo "ABSPATH = " . ABSPATH . "\n";
echo "WP version = " . (defined('WP_VERSION') ? WP_VERSION : 'unknown') . "\n";
do_action('init');
