<?php
// WordPress core functions stub for phprs

// Get an option from the database
function get_option($option, $default = false) {
    global $wpdb;
    if (isset($wpdb)) {
        return $wpdb->get_option($option, $default);
    }
    return $default;
}

// Update an option in the database
function update_option($option, $value) {
    global $wpdb;
    if (isset($wpdb)) {
        $wpdb->set_option($option, $value);
        return true;
    }
    return false;
}

// Add a new option to the database
function add_option($option, $value) {
    global $wpdb;
    if (isset($wpdb)) {
        $wpdb->set_option($option, $value);
        return true;
    }
    return false;
}

// Delete an option from the database
function delete_option($option) {
    global $wpdb;
    if (isset($wpdb)) {
        $wpdb->set_option($option, null);
        return true;
    }
    return false;
}

// Get the site URL
function get_site_url() {
    return get_option('siteurl', 'http://localhost');
}

// Get the home URL
function get_home_url() {
    return get_option('home', 'http://localhost');
}

// Get the blog name
function get_bloginfo($show = '') {
    if ($show === 'name') {
        return get_option('blogname', 'WordPress');
    }
    if ($show === 'description') {
        return get_option('blogdescription', '');
    }
    if ($show === 'url') {
        return get_site_url();
    }
    return '';
}

// Sanitize a string
function sanitize_text_field($str) {
    return trim($str);
}

// Escape HTML
function esc_html($text) {
    return htmlspecialchars($text);
}

// Escape attributes
function esc_attr($text) {
    return htmlspecialchars($text);
}

// Escape URL
function esc_url($url) {
    return $url;
}

// Check if current user can perform an action (stub - always returns false)
function current_user_can($capability) {
    return false;
}

// Check if user is logged in (stub - always returns false)
function is_user_logged_in() {
    return false;
}

// WordPress version check
function wp_version_check() {
    return true;
}

// Load plugin API functions (hooks are already stubbed in builtins.rs)
function add_action($hook, $callback, $priority = 10, $accepted_args = 1) {
    return true;
}

function add_filter($hook, $callback, $priority = 10, $accepted_args = 1) {
    return true;
}

function remove_action($hook, $callback, $priority = 10) {
    return true;
}

function remove_filter($hook, $callback, $priority = 10) {
    return true;
}

function has_action($hook, $callback = false) {
    return false;
}

function has_filter($hook, $callback = false) {
    return false;
}
