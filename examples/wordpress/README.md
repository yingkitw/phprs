# WordPress-style example (phprs)

Minimal WordPress-like bootstrap intended for development and include-path testing.

## Run

From the **project root**:

```bash
cargo run -p phprs-cli -- run examples/wordpress/index.php
```

## Status

- **`array()` constructor** compiles (enables `wp-includes/wp-db.php`).
- **Full bootstrap runs**: `index.php` → `wp-blog-header.php` → `wp-load.php` → `wp-settings.php` → plugins/theme hooks complete successfully.
- **Covered by a test**: `example_wordpress_index_runs` in `tests/examples_runtime.rs` (the nested tree is outside the root `examples/*.php` matrix, which only scans top-level files).
- **`test-theme-plugin.php`** is manual — run it with `cargo run -p phprs-cli -- run examples/wordpress/test-theme-plugin.php`.
- Demo stubs only — **not** production WordPress core.

## Layout

- `index.php` — entry point; requires `wp-blog-header.php`
- `wp-blog-header.php` — requires `wp-load.php`, then `wp-settings.php`; prints greeting and config; calls `do_action('init')`
- `wp-load.php` — requires `wp-config.php` when present
- `wp-config.php` — defines `ABSPATH`, DB constants, `$table_prefix`
- `wp-settings.php` — loads wpdb and core includes
- `wp-includes/wp-db.php` — wpdb stub (in-memory)
- `wp-includes/functions.php`, `plugin.php`, `theme.php` — core stubs

## Engine features used

- `require` / `include` / `require_once` with cwd-first path resolution
- `define()`, `defined()`, `constant()`; `__DIR__`, `__FILE__`
- `dirname()`, `file_exists()`, `file_get_contents()` (script-relative where applicable)
- `do_action()`, `apply_filters()` (stubs with priority)
- Classes, globals (`$wpdb`, `$table_prefix`)

See [THEME-PLUGIN-README.md](THEME-PLUGIN-README.md) for plugin/theme layout (same compiler constraints apply).
