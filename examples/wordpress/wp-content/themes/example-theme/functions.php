<?php
/**
 * Example Theme Functions
 * 
 * Theme setup and functionality for the example theme
 */

// Theme setup
add_action('after_setup_theme', 'example_theme_setup');

function example_theme_setup() {
    // Add theme support for various features
    add_theme_support('title-tag');
    add_theme_support('post-thumbnails');
    add_theme_support('html5', array('search-form', 'comment-form', 'comment-list', 'gallery', 'caption'));
    add_theme_support('custom-logo');
    add_theme_support('custom-header');
    add_theme_support('custom-background');
    
    // Register navigation menus
    register_nav_menus(array(
        'primary' => 'Primary Menu',
        'footer' => 'Footer Menu'
    ));
}

// Register widget areas
add_action('widgets_init', 'example_theme_widgets_init');

function example_theme_widgets_init() {
    register_sidebar(array(
        'name' => 'Sidebar',
        'id' => 'sidebar-1',
        'description' => 'Main sidebar widget area',
        'before_widget' => '<div class="widget">',
        'after_widget' => '</div>',
        'before_title' => '<h3 class="widget-title">',
        'after_title' => '</h3>'
    ));
    
    register_sidebar(array(
        'name' => 'Footer',
        'id' => 'footer-1',
        'description' => 'Footer widget area',
        'before_widget' => '<div class="footer-widget">',
        'after_widget' => '</div>',
        'before_title' => '<h4 class="footer-widget-title">',
        'after_title' => '</h4>'
    ));
}

// Enqueue scripts and styles
add_action('wp_enqueue_scripts', 'example_theme_scripts');

function example_theme_scripts() {
    wp_enqueue_style('example-theme-style', get_stylesheet_directory_uri() . '/style.css');
    wp_enqueue_script('example-theme-script', get_template_directory_uri() . '/js/main.js', array(), '1.0.0', true);
}

// Custom post type example
add_action('init', 'example_theme_custom_post_types');

function example_theme_custom_post_types() {
    // Register custom post type (stub)
}

// Custom taxonomy example
add_action('init', 'example_theme_custom_taxonomies');

function example_theme_custom_taxonomies() {
    // Register custom taxonomy (stub)
}

// Customizer settings
add_action('customize_register', 'example_theme_customize_register');

function example_theme_customize_register($wp_customize) {
    // Add customizer settings (stub)
}

// Filter example - modify excerpt length
add_filter('excerpt_length', 'example_theme_excerpt_length');

function example_theme_excerpt_length($length) {
    return 30;
}

// Filter example - modify excerpt more text
add_filter('excerpt_more', 'example_theme_excerpt_more');

function example_theme_excerpt_more($more) {
    return '...';
}

// Custom template tags
function example_theme_posted_on() {
    echo 'Posted on: ' . get_the_date();
}

function example_theme_posted_by() {
    echo 'By: ' . get_the_author();
}

// Breadcrumb function
function example_theme_breadcrumbs() {
    echo '<nav class="breadcrumbs">';
    echo '<a href="' . esc_url(get_home_url()) . '">Home</a> &raquo; ';
    echo get_the_title();
    echo '</nav>';
}

// Social media links
function example_theme_social_links() {
    $social_links = array(
        'facebook' => 'https://facebook.com/example',
        'twitter' => 'https://twitter.com/example',
        'instagram' => 'https://instagram.com/example'
    );
    
    echo '<div class="social-links">';
    foreach ($social_links as $platform => $url) {
        echo '<a href="' . esc_url($url) . '" class="social-' . esc_attr($platform) . '">' . ucfirst($platform) . '</a>';
    }
    echo '</div>';
}
