<?php
// WordPress Theme API

// Get current theme directory
function get_template_directory() {
    return dirname(dirname(__FILE__)) . '/wp-content/themes/' . get_template();
}

// Get current theme name (stub)
function get_template() {
    return 'example-theme';
}

// Get stylesheet directory (for child themes)
function get_stylesheet_directory() {
    return get_template_directory();
}

// Get theme file URI
function get_template_directory_uri() {
    return 'http://localhost/wp-content/themes/' . get_template();
}

// Load theme functions.php
function wp_load_theme() {
    $theme_dir = get_template_directory();
    $functions_file = $theme_dir . '/functions.php';
    
    if (file_exists($functions_file)) {
        include_once $functions_file;
    }
    
    do_action('after_setup_theme');
}

// Theme support functions
global $wp_theme_features;
$wp_theme_features = array();

function add_theme_support($feature) {
    global $wp_theme_features;
    $wp_theme_features[$feature] = true;
}

function remove_theme_support($feature) {
    global $wp_theme_features;
    unset($wp_theme_features[$feature]);
}

function current_theme_supports($feature) {
    global $wp_theme_features;
    return isset($wp_theme_features[$feature]);
}

// Register navigation menu
function register_nav_menu($location, $description) {
    register_nav_menus(array($location => $description));
}

function register_nav_menus($locations) {
    global $wp_registered_nav_menus;
    if (!isset($wp_registered_nav_menus)) {
        $wp_registered_nav_menus = array();
    }
    $wp_registered_nav_menus = array_merge($wp_registered_nav_menus, $locations);
}

// Widget functions (stubs)
function register_sidebar($args) {
    return true;
}

function register_widget($widget_class) {
    return true;
}

// Template functions
function get_header($name = null) {
    do_action('get_header', $name);
    $template = get_template_directory() . '/header.php';
    if (file_exists($template)) {
        include $template;
    }
}

function get_footer($name = null) {
    do_action('get_footer', $name);
    $template = get_template_directory() . '/footer.php';
    if (file_exists($template)) {
        include $template;
    }
}

function get_sidebar($name = null) {
    do_action('get_sidebar', $name);
    $template = get_template_directory() . '/sidebar.php';
    if (file_exists($template)) {
        include $template;
    }
}

// Template part loading
function get_template_part($slug, $name = null) {
    $templates = array();
    if ($name) {
        $templates[] = $slug . '-' . $name . '.php';
    }
    $templates[] = $slug . '.php';
    
    $template_dir = get_template_directory();
    foreach ($templates as $template_name) {
        $template_file = $template_dir . '/' . $template_name;
        if (file_exists($template_file)) {
            include $template_file;
            return;
        }
    }
}

// Enqueue scripts and styles (stubs)
function wp_enqueue_script($handle, $src = '', $deps = array(), $ver = false, $in_footer = false) {
    return true;
}

function wp_enqueue_style($handle, $src = '', $deps = array(), $ver = false, $media = 'all') {
    return true;
}

function wp_head() {
    do_action('wp_head');
}

function wp_footer() {
    do_action('wp_footer');
}
