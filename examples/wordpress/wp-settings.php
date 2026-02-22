<?php
// Minimal wp-settings: core loaded (stub for phprs)

// Get the WordPress root directory
$wp_root = dirname(__FILE__);

// Load wpdb class
require_once $wp_root . '/wp-includes/wp-db.php';

// Initialize database connection (stub)
global $wpdb, $table_prefix;
$wpdb = new wpdb(DB_USER, DB_PASSWORD, DB_NAME, DB_HOST);

// Load core functions
require_once $wp_root . '/wp-includes/functions.php';

// Load theme and plugin support
require_once $wp_root . '/wp-includes/plugin.php';
require_once $wp_root . '/wp-includes/theme.php';

// Load plugins
wp_load_plugins();

// Load theme
wp_load_theme();

// Fire init action
do_action('init');
