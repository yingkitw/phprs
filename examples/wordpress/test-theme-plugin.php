<?php
/**
 * Test script for WordPress theme and plugin loading
 * 
 * This script demonstrates:
 * - Plugin loading and activation hooks
 * - Theme loading and setup hooks
 * - Action and filter hooks
 * - Session handling
 */

// Load WordPress
require_once 'wp-load.php';
require_once 'wp-settings.php';

echo "=== WordPress Theme and Plugin Test ===\n\n";

// Test 1: Check if plugins are loaded
echo "Test 1: Plugin Loading\n";
echo "- Plugins loaded action fired: " . (did_action('plugins_loaded') > 0 ? 'Yes' : 'No') . "\n";
echo "- Init action fired: " . (did_action('init') > 0 ? 'Yes' : 'No') . "\n\n";

// Test 2: Check if theme is loaded
echo "Test 2: Theme Loading\n";
echo "- Theme setup action fired: " . (did_action('after_setup_theme') > 0 ? 'Yes' : 'No') . "\n";
echo "- Current theme: " . get_template() . "\n";
echo "- Theme directory: " . get_template_directory() . "\n\n";

// Test 3: Test action hooks
echo "Test 3: Action Hooks\n";
$test_action_fired = false;
add_action('test_action', function() {
    global $test_action_fired;
    $test_action_fired = true;
    echo "- Custom action executed!\n";
});
do_action('test_action');
echo "- Action hook works: " . ($test_action_fired ? 'Yes' : 'No') . "\n\n";

// Test 4: Test filter hooks
echo "Test 4: Filter Hooks\n";
add_filter('test_filter', function($value) {
    return $value . ' (filtered)';
});
$filtered = apply_filters('test_filter', 'Original value');
echo "- Filter result: " . $filtered . "\n";
echo "- Filter hook works: " . (strpos($filtered, 'filtered') !== false ? 'Yes' : 'No') . "\n\n";

// Test 5: Test theme support
echo "Test 5: Theme Support\n";
echo "- Post thumbnails: " . (current_theme_supports('post-thumbnails') ? 'Supported' : 'Not supported') . "\n";
echo "- Custom logo: " . (current_theme_supports('custom-logo') ? 'Supported' : 'Not supported') . "\n\n";

// Test 6: Test session handling
echo "Test 6: Session Handling\n";
wp_session_start();
wp_session_set('test_key', 'test_value');
$session_value = wp_session_get('test_key');
echo "- Session set/get works: " . ($session_value === 'test_value' ? 'Yes' : 'No') . "\n";
wp_session_delete('test_key');
$deleted_value = wp_session_get('test_key');
echo "- Session delete works: " . ($deleted_value === null ? 'Yes' : 'No') . "\n\n";

// Test 7: Test database options
echo "Test 7: Database Options\n";
$site_url = get_option('siteurl');
echo "- Site URL: " . $site_url . "\n";
update_option('test_option', 'test_value');
$test_option = get_option('test_option');
echo "- Option set/get works: " . ($test_option === 'test_value' ? 'Yes' : 'No') . "\n\n";

// Test 8: Test wpdb
echo "Test 8: Database Layer\n";
global $wpdb;
echo "- wpdb initialized: " . (isset($wpdb) ? 'Yes' : 'No') . "\n";
echo "- Database name: " . $wpdb->dbname . "\n";
echo "- Table prefix: " . $wpdb->prefix . "\n\n";

echo "=== All Tests Complete ===\n";
