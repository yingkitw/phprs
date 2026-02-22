<?php
// Load WordPress and run main request
require 'wp-load.php';
require 'wp-settings.php';

echo "Hello from WordPress (phprs)!\n";
echo "ABSPATH = " . ABSPATH . "\n";
echo "WP version = " . (defined('WP_VERSION') ? WP_VERSION : 'unknown') . "\n";
echo "DB Name = " . DB_NAME . "\n";
echo "Table prefix = " . $table_prefix . "\n";

// Test wpdb
global $wpdb;
if (isset($wpdb)) {
    echo "wpdb initialized: " . $wpdb->dbname . "\n";
}

// Test options
$site_url = get_option('siteurl');
echo "Site URL = " . $site_url . "\n";

$blog_name = get_bloginfo('name');
echo "Blog name = " . $blog_name . "\n";

do_action('init');
