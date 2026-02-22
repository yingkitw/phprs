# WordPress Theme and Plugin Support in phprs

This directory demonstrates a complete WordPress-style theme and plugin system implemented in phprs.

## Features Implemented

### Plugin System
- **Plugin Loading**: Automatic loading of plugins from `wp-content/plugins/`
- **Activation Hooks**: `register_activation_hook()` and `register_deactivation_hook()`
- **Action Hooks**: Full `add_action()`, `do_action()`, `remove_action()` with priority support
- **Filter Hooks**: Full `add_filter()`, `apply_filters()`, `remove_filter()` with priority support
- **Hook Introspection**: `has_action()`, `has_filter()`, `did_action()`

### Theme System
- **Theme Loading**: Automatic loading of theme `functions.php`
- **Theme Support**: `add_theme_support()`, `remove_theme_support()`, `current_theme_supports()`
- **Navigation Menus**: `register_nav_menu()`, `register_nav_menus()`
- **Sidebars/Widgets**: `register_sidebar()`, `register_widget()`
- **Template Functions**: `get_header()`, `get_footer()`, `get_sidebar()`, `get_template_part()`
- **Asset Enqueuing**: `wp_enqueue_script()`, `wp_enqueue_style()` (stubs)

### Session Handling
- **Session Functions**: `wp_session_start()`, `wp_session_get()`, `wp_session_set()`, `wp_session_delete()`
- **PHP Session Stubs**: `session_start()`, `session_destroy()`, `session_id()`, etc.

### Database Integration
- **wpdb Class**: Full in-memory database abstraction
- **Options API**: `get_option()`, `update_option()`, `add_option()`, `delete_option()`

## Directory Structure

```
examples/wordpress/
├── wp-content/
│   ├── plugins/
│   │   └── example-plugin/
│   │       └── example-plugin.php    # Example plugin with hooks
│   └── themes/
│       └── example-theme/
│           ├── functions.php          # Theme setup and hooks
│           └── style.css              # Theme stylesheet
├── wp-includes/
│   ├── wp-db.php                      # Database abstraction layer
│   ├── functions.php                  # Core WordPress functions
│   ├── plugin.php                     # Plugin API and hooks system
│   ├── theme.php                      # Theme API
│   └── session.php                    # Session handling
├── wp-config.php                      # WordPress configuration
├── wp-settings.php                    # Core loading script
├── wp-load.php                        # Configuration loader
├── wp-blog-header.php                 # Main request handler
├── index.php                          # Entry point
└── test-theme-plugin.php              # Comprehensive test script
```

## Running the Examples

### Basic WordPress Bootstrap
```bash
cargo run -p phprs-cli -- run examples/wordpress/index.php
```

### Theme and Plugin Test Suite
```bash
cargo run -p phprs-cli -- run examples/wordpress/test-theme-plugin.php
```

## Example Plugin

The example plugin (`wp-content/plugins/example-plugin/example-plugin.php`) demonstrates:

1. **Plugin Header**: Standard WordPress plugin metadata
2. **Activation/Deactivation Hooks**: Setup and cleanup on plugin state changes
3. **Action Hooks**: Responding to WordPress events (`plugins_loaded`, `admin_menu`, `wp_footer`)
4. **Filter Hooks**: Modifying content (`the_content` filter)
5. **Shortcodes**: Custom `[example]` shortcode
6. **AJAX Handlers**: Example AJAX endpoint

### Plugin Code Example
```php
// Add filter to modify content
add_filter('the_content', 'example_plugin_filter_content');

function example_plugin_filter_content($content) {
    return '[Example Plugin] ' . $content;
}

// Add action to footer
add_action('wp_footer', 'example_plugin_footer');

function example_plugin_footer() {
    echo '<!-- Example Plugin Active -->';
}
```

## Example Theme

The example theme (`wp-content/themes/example-theme/`) demonstrates:

1. **Theme Setup**: Registering theme features and support
2. **Navigation Menus**: Multiple menu locations
3. **Widget Areas**: Sidebar and footer widget areas
4. **Script/Style Enqueuing**: Loading theme assets
5. **Custom Functions**: Template tags and helper functions
6. **Filters**: Modifying WordPress behavior (excerpt length, etc.)

### Theme Code Example
```php
// Theme setup
add_action('after_setup_theme', 'example_theme_setup');

function example_theme_setup() {
    add_theme_support('post-thumbnails');
    add_theme_support('custom-logo');
    
    register_nav_menus(array(
        'primary' => 'Primary Menu',
        'footer' => 'Footer Menu'
    ));
}

// Enqueue assets
add_action('wp_enqueue_scripts', 'example_theme_scripts');

function example_theme_scripts() {
    wp_enqueue_style('example-theme-style', get_stylesheet_directory_uri() . '/style.css');
}
```

## Test Script Output

The `test-theme-plugin.php` script validates:

1. ✓ Plugin loading and `plugins_loaded` action
2. ✓ Theme loading and `after_setup_theme` action
3. ✓ Custom action hooks execution
4. ✓ Filter hooks with value modification
5. ✓ Theme support features
6. ✓ Session handling (set/get/delete)
7. ✓ Database options API
8. ✓ wpdb initialization

## Hooks System Implementation

The hooks system supports:
- **Priority-based execution**: Hooks execute in priority order (default: 10)
- **Multiple callbacks**: Multiple functions can hook into the same action/filter
- **Hook removal**: `remove_action()` and `remove_filter()`
- **Hook introspection**: Check if hooks exist or have been fired
- **Filter chaining**: Filters can modify values through multiple callbacks

### Hook Execution Flow
```
1. add_action('init', 'my_function', 10)
2. add_action('init', 'another_function', 5)  // Executes first (lower priority)
3. do_action('init')
   → Executes 'another_function' (priority 5)
   → Executes 'my_function' (priority 10)
```

## Built-in Functions Added

For WordPress compatibility, the following built-in functions were added to phprs:

- `isset()`, `empty()`, `unset()` - Variable handling
- `htmlspecialchars()`, `htmlentities()` - HTML escaping
- `esc_html()`, `esc_attr()`, `esc_url()` - WordPress escaping
- `preg_match()`, `preg_replace()` - Regex (stubs)
- `shortcode_atts()` - Shortcode attribute merging
- `array_merge()` - Array merging
- `ucfirst()` - String capitalization
- `str_replace()` - String replacement
- `uniqid()`, `time()`, `dechex()` - Utility functions

## Session Handling

Session data is stored in-memory using global variables:

```php
wp_session_start();
wp_session_set('user_id', 123);
$user_id = wp_session_get('user_id');
wp_session_delete('user_id');
wp_session_destroy();
```

## Known Limitations

1. **Constant Concatenation**: Direct constant concatenation (e.g., `ABSPATH . 'file.php'`) has issues. Use `constant()` function or variables as workaround.
2. **Regex**: `preg_match()` and `preg_replace()` are stubs and don't perform actual regex operations.
3. **Database**: wpdb uses in-memory storage, not a real database connection.
4. **Sessions**: Session data is not persisted between script executions.
5. **AJAX**: AJAX handlers are stubs and don't process actual HTTP requests.

## Future Enhancements

- Real regex support with `preg_*` functions
- Persistent session storage
- Database connection to MySQL/PostgreSQL
- HTTP request handling for AJAX
- Template rendering engine
- Full shortcode parsing
- Widget rendering
- Customizer API implementation

## Testing

Run the comprehensive test suite:
```bash
cargo run -p phprs-cli -- run examples/wordpress/test-theme-plugin.php
```

Expected output shows all tests passing with "Yes" confirmations for each feature.

## Contributing

When adding new WordPress features:
1. Add function stubs to appropriate `wp-includes/*.php` files
2. Implement built-in functions in `src/engine/vm/builtins.rs` if needed
3. Add test cases to `test-theme-plugin.php`
4. Update this README with new features
5. Follow WordPress coding standards and naming conventions
