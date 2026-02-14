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
- [x] VM execution (64 opcodes)
- [x] Built-in functions (40+ functions)

### Tools
- [x] Unified CLI (`bin/phprs`) with `run`, `serve`, `pkg` subcommands
- [x] Web playground (`phprs serve`)
- [x] Test suite
- [x] Comprehensive examples

### Language Features (Phase 2)
- [x] Namespaces
- [x] Traits
- [x] Attributes (PHP 8.0)
- [x] Match expressions (PHP 8.0)
- [x] Generators (yield → array accumulation)

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

## Statistics

- **35 source files** in engine/
- **12 source files** in php/
- **197 tests** passing
- **64 opcodes** implemented
- **40+ built-in functions**

