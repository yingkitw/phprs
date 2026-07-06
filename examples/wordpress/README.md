# WordPress-style example (phprs)

Minimal WordPress-like bootstrap intended for development and include-path testing.

## Run

From the **project root**:

```bash
cargo run -p phprs-cli -- run examples/wordpress/index.php
```

## Status

- **Include resolution**: Relative paths are resolved like PHP — first relative to the process **current working directory**, then relative to the **including script’s directory** (see `resolve_include_path` in `src/engine/vm/dispatch_handlers.rs`).
- **Known blocker**: `wp-includes/wp-db.php` uses legacy `array()` syntax; the phprs compiler currently expects short array `[]` in many contexts. The bootstrap may fail at `require` of `wp-db.php` until that syntax is supported or the stub is rewritten.
- **Not in the root example matrix**: Only `examples/*.php` at the repository root are auto-tested; nested `wordpress/` entrypoints are run manually.

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
