# phprs Migration TODO

## Completed ✅

### Core Engine
- [x] Type system (Val, strings, arrays, objects)
- [x] String handling (DJBX33A hashing)
- [x] Hash tables (dynamic resizing)
- [x] Memory allocation (persistent/non-persistent)
- [x] Garbage collection (tri-color marking)
- [x] Operators and type conversion

### PHP Runtime
- [x] Runtime functions
- [x] INI configuration
- [x] Variable handling
- [x] Stream system (file streams)
- [x] SAPI layer (CLI)
- [x] Output buffering
- [x] Global state
- [x] Filesystem operations
- [x] Extension framework

### Compiler & VM
- [x] Lexer (tokenizer with `?`, `??`, `?->`, `::` support)
- [x] Expression parsing (arithmetic, comparison, logical, bitwise)
- [x] Ternary operator (`?:` and short `?:`)
- [x] Null coalescing (`??`)
- [x] Closures / anonymous functions (`function() use () { ... }`)
- [x] Callable variables (`$fn()`)
- [x] Type declarations (parameter types, return types, nullable)
- [x] Function return values (`return expr;`)
- [x] Statement parsing (echo, assign, return, include)
- [x] Control flow (if/else, while, for, foreach)
- [x] Function compilation and calls
- [x] Class compilation (properties, methods, constructors)
- [x] VM execution (63 opcodes, dispatch table)
- [x] Built-in functions (40+ functions)

### Tools
- [x] Unified CLI (`bin/phprs`) with `run`, `serve`, `pkg` subcommands
- [x] Web playground (`phprs serve`)
- [x] Test suite
- [x] Comprehensive examples

### Performance Optimizations
- [x] JIT compilation system (for hot functions)
- [x] Function optimizer (inlining, call optimization)
- [x] Opcode cache with optimization passes
- [x] Thread-safe global state using OnceLock and RwLock
- [x] Fixed mutable static reference issues for Rust 2024 compliance

### Language Features (Phase 2)
- [x] Namespaces
- [x] Traits
- [x] Attributes (PHP 8.0)
- [x] Match expressions (PHP 8.0)
- [x] Generators (yield → array accumulation)

### WordPress example support
- [x] define(), defined(), constant(); bare-identifier constant lookup
- [x] __DIR__ and __FILE__ magic constants (per-script)
- [x] require/include relative to current script dir; caller state restored after include
- [x] dirname(), exit(), die(); do_action(), apply_filters() stubs

### Package Manager
- [x] CLI framework
- [x] Composer.json parsing
- [x] Packagist API client
- [x] Autoloader generation (PSR-4)
- [x] Dependency resolution (transitive, semver)
- [x] Package installation

## Planned 📋

### Standard Library
- [x] Stream wrappers (HTTP, FTP)
  - [x] HTTP/HTTPS stream wrapper with reqwest
  - [x] file_get_contents() HTTP support
  - [x] FTP stream wrapper (stub)
  - [x] Custom stream contexts (basic)
- [x] Regular expressions (preg_match, preg_replace)
  - [x] preg_match() with capture groups
  - [x] preg_match_all() for multiple matches
  - [x] preg_replace() with pattern replacement
  - [x] preg_split() for pattern-based splitting
  - [x] PCRE flag support (i, m, s, x)
  - [x] Regex compilation and caching
- [x] Session handling
  - [x] session_start(), session_destroy()
  - [x] session_id(), session_name(), session_regenerate_id()
  - [x] $_SESSION superglobal support
  - [x] In-memory session storage
  - [x] Session persistence (file-based available)
- [x] PDO/database layer
  - [x] PDO class with connection management
  - [x] Query execution (query(), exec())
  - [x] Prepared statements (prepare(), execute())
  - [x] Parameter binding (bindParam())
  - [x] Transactions (beginTransaction(), commit(), rollback())
  - [x] Fetch operations (fetch(), fetchAll())
  - [x] Error handling (errorInfo())
  - [x] Multiple driver support (MySQL, PostgreSQL, SQLite stubs)

### Framework Roadmap

#### CodeIgniter 4
- [ ] Bootstrap (index.php → system/bootstrap.php)
- [ ] Autoloading
- [ ] Routing
- [ ] Controllers

#### Drupal
- [ ] Bootstrap (index.php → core/lib/Drupal.php)
- [ ] Kernel initialization
- [ ] Module system

#### WordPress
- [x] Bootstrap (index.php → wp-blog-header.php → wp-load.php → wp-config.php → wp-settings.php)
- [x] wp-config-style constants (ABSPATH, WP_DEBUG; define/defined/constant, __DIR__, __FILE__)
- [x] Relative include resolution; include restores caller state
- [x] Minimal example in examples/wordpress (runnable)
- [x] do_action / apply_filters (full implementation with priority support)
- [x] wp-config.php parsing (DB_*, table prefix)
- [x] Database layer for wpdb (in-memory stub with query/get_results/insert/update/delete)
- [x] Core loading (wp-includes: wpdb class, core functions)
- [x] WordPress core functions (get_option, update_option, get_bloginfo, sanitize_text_field, esc_html, etc.)
- [x] Theme and plugin loading with hooks system
  - [x] Plugin API (add_action, add_filter, remove_action, remove_filter, has_action, has_filter)
  - [x] Plugin loading (wp_load_plugins, register_activation_hook, register_deactivation_hook)
  - [x] Theme API (add_theme_support, register_nav_menus, register_sidebar, get_template_part)
  - [x] Theme loading (wp_load_theme, after_setup_theme hook)
  - [x] Session handling (wp_session_start, wp_session_get, wp_session_set, session stubs)
  - [x] Example plugin with activation hooks and filters
  - [x] Example theme with functions.php and theme setup
  - [x] Comprehensive test script (test-theme-plugin.php)

## Statistics

- **Engine**: types, string, hash, alloc, gc, operators, array_ops, lexer, compile, vm, jit, function_optimizer, opcode_cache, benchmark, perf, perf_alloc, facade, errors, exception
- **PHP runtime**: 15 source files in php/ (added regex, http_stream, pdo)
- **63 opcodes** (dispatch table, dispatch_handlers)
- **70+ built-in functions** (including isset, empty, htmlspecialchars, preg_*, shortcode_atts, array_merge, ucfirst, etc.)
- **Thread-safe** JIT and optimizer (Arc, OnceLock, RwLock)
- **Standard Library**:
  - Full regex support with `regex` crate (preg_match, preg_match_all, preg_replace, preg_split)
  - HTTP/HTTPS stream wrappers with `reqwest`
  - Session handling with in-memory and file persistence
  - PDO database abstraction layer
  - FTP stream wrapper (stub)
- **WordPress support**: 
  - wpdb class with in-memory storage
  - Complete hooks system (actions and filters with priority support)
  - Plugin API and loading system
  - Theme API and loading system
  - Session handling
  - 40+ WordPress-specific functions
  - Example plugin and theme with comprehensive test suite

## Documentation

- [SPEC.md](SPEC.md) - Project specification and scope
- [PERFORMANCE.md](PERFORMANCE.md) - Performance optimizations and benchmarks vs PHP 8

