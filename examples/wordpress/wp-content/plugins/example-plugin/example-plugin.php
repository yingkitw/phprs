<?php
/**
 * Plugin Name: Example Plugin
 * Plugin URI: https://example.com/plugins/example-plugin
 * Description: A simple example plugin demonstrating WordPress plugin API in phprs
 * Version: 1.0.0
 * Author: phprs Team
 * Author URI: https://github.com/yingkitw/phprs
 * License: Apache-2.0
 */

// Prevent direct access
if (!defined('ABSPATH') && !defined('WP_VERSION')) {
    exit;
}

// Plugin activation hook
register_activation_hook(__FILE__, 'example_plugin_activate');

function example_plugin_activate() {
    // Set default options
    add_option('example_plugin_version', '1.0.0');
    add_option('example_plugin_enabled', true);
}

// Plugin deactivation hook
register_deactivation_hook(__FILE__, 'example_plugin_deactivate');

function example_plugin_deactivate() {
    // Cleanup if needed
    delete_option('example_plugin_enabled');
}

// Initialize plugin
add_action('plugins_loaded', 'example_plugin_init');

function example_plugin_init() {
    // Plugin initialization code
    add_filter('the_content', 'example_plugin_filter_content');
    add_action('wp_footer', 'example_plugin_footer');
}

// Filter content
function example_plugin_filter_content($content) {
    return '[Example Plugin] ' . $content;
}

// Add footer content
function example_plugin_footer() {
    echo '<!-- Example Plugin Active -->';
}

// Admin menu (stub)
add_action('admin_menu', 'example_plugin_admin_menu');

function example_plugin_admin_menu() {
    // In real WordPress, this would add admin menu items
}

// Shortcode example
add_shortcode('example', 'example_plugin_shortcode');

function example_plugin_shortcode($atts) {
    $atts = shortcode_atts(array(
        'message' => 'Hello from Example Plugin!'
    ), $atts);
    
    return '<div class="example-plugin">' . esc_html($atts['message']) . '</div>';
}

// AJAX handler example (stub)
add_action('wp_ajax_example_action', 'example_plugin_ajax_handler');
add_action('wp_ajax_nopriv_example_action', 'example_plugin_ajax_handler');

function example_plugin_ajax_handler() {
    // AJAX handling code
    echo json_encode(array('success' => true, 'message' => 'AJAX works!'));
    exit;
}
