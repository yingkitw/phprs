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
- [ ] Stream wrappers (HTTP, FTP)
- [ ] Regular expressions (preg_match, preg_replace)
- [ ] Session handling
- [ ] PDO/database layer

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
- [x] do_action / apply_filters stubs (no-op)
- [ ] wp-config.php parsing (DB_*, table prefix) with real DB
- [ ] Database layer for wpdb (depends on PDO/MySQLi or compatible stub)
- [ ] Core loading (wp-includes: functions, class-wp.php, pluggable, options)
- [ ] Theme and plugin loading (after standard library: sessions, regex as needed)

## Statistics

- **Engine**: types, string, hash, alloc, gc, operators, array_ops, lexer, compile, vm, jit, function_optimizer, opcode_cache, benchmark, perf, perf_alloc, facade, errors, exception
- **PHP runtime**: 12 source files in php/
- **63 opcodes** (dispatch table, dispatch_handlers)
- **40+ built-in functions**
- **Thread-safe** JIT and optimizer (Arc, OnceLock, RwLock)

## Documentation

- [spec.md](spec.md) - Project specification and scope
- [PERFORMANCE.md](PERFORMANCE.md) - Performance optimizations and benchmarks vs PHP 8

