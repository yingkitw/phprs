# WordPress-style example (phprs)

Minimal WordPress-like bootstrap intended for development and include-path testing.

## Run

From the **project root**:

```bash
cargo run -p phprs-cli -- run examples/wordpress/index.php
```

## Status

- **`array()` constructor** compiles (enables `wp-includes/wp-db.php`).
- **Full bootstrap** still fails on later includes (e.g. `plugin.php` uses assignment patterns the compiler does not support yet).
- **Not in the root example matrix** — run manually while the nested tree is being brought up.

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
