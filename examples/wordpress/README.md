# WordPress-style example (phprs)

Minimal WordPress-like bootstrap that runs on the phprs engine.

## Run

From the **project root**:

```bash
cargo run -p phprs-cli -- run examples/wordpress/index.php
```

## Layout

- `index.php` — entry point; requires `wp-blog-header.php`
- `wp-blog-header.php` — requires `wp-load.php`, then `wp-settings.php`; prints greeting, ABSPATH, WP version; calls `do_action('init')`
- `wp-load.php` — requires `wp-config.php` only if `file_exists('wp-config.php')` (relative to script dir); else echoes and `exit(1)`
- `wp-config.php` — defines `ABSPATH` (via `dirname(__FILE__) . '/'`), `WP_DEBUG`, `WP_DEBUG_DISPLAY`, `WP_VERSION`
- `wp-settings.php` — stub for “core” load

## Engine support used

- `require` / `include` with relative path resolution (current script directory); caller state restored after include
- `define()`, `defined()`, `constant()`; bare-identifier constant lookup
- Magic constants: `__DIR__`, `__FILE__` (set per script)
- `dirname()`, `file_exists()`, `file_get_contents()` with script-relative path resolution
- `exit()` / `die()` (optional message or status code)
- `do_action()`, `apply_filters()` (stubs)
- `if ( condition ) { ... } else { ... }` (fixed so `)` after condition is not consumed twice)
