# WordPress-style example (phprs)

Minimal WordPress-like bootstrap that runs on the phprs engine.

## Run

From the **project root**:

```bash
cargo run -p phprs-cli -- run examples/wordpress/index.php
```

## Layout

- `index.php` — entry point; requires `wp-blog-header.php`
- `wp-blog-header.php` — requires `wp-load.php`, then `wp-settings.php`; prints greeting, ABSPATH, WP version, database info; calls `do_action('init')`
- `wp-load.php` — requires `wp-config.php` only if `file_exists('wp-config.php')` (relative to script dir); else echoes and `exit(1)`
- `wp-config.php` — defines `ABSPATH`, `WP_DEBUG`, `WP_DEBUG_DISPLAY`, `WP_VERSION`, database constants (DB_NAME, DB_USER, DB_PASSWORD, DB_HOST, DB_CHARSET, DB_COLLATE), and `$table_prefix`
- `wp-settings.php` — loads wpdb class and core functions
- `wp-includes/wp-db.php` — wpdb class with in-memory storage (query, get_results, insert, update, delete methods)
- `wp-includes/functions.php` — WordPress core functions (get_option, update_option, get_bloginfo, sanitize_text_field, esc_html, etc.)

## Engine support used

- `require` / `include` with relative path resolution (current script directory); caller state restored after include
- `define()`, `defined()`, `constant()`; bare-identifier constant lookup
- Magic constants: `__DIR__`, `__FILE__` (set per script)
- `dirname()`, `file_exists()`, `file_get_contents()` with script-relative path resolution
- `exit()` / `die()` (optional message or status code)
- `do_action()`, `apply_filters()` (stubs)
- `isset()`, `empty()`, `unset()` for variable handling
- `htmlspecialchars()`, `htmlentities()` for HTML escaping
- `preg_match()`, `preg_replace()` (stubs)
- Classes and objects (wpdb class)
- Global variables (`$wpdb`, `$table_prefix`)
- `if ( condition ) { ... } else { ... }` (fixed so `)` after condition is not consumed twice)
