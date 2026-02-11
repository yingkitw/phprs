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
- [x] Statement parsing (echo, assign, return, include)
- [x] Control flow (if/else, while, for, foreach)
- [x] Function compilation and calls
- [x] Class compilation (properties, methods, constructors)
- [x] VM execution (63 opcodes)
- [x] Built-in functions (40+ functions)

### Tools
- [x] CLI interpreter (`bin/php`)
- [x] Web playground (`bin/php-server`)
- [x] Test suite (252 tests passing)
- [x] Comprehensive examples

## In Progress 🚧

### Language Features
- [ ] Closures / anonymous functions
- [ ] Type declarations
- [ ] Namespaces
- [ ] Traits

### Package Manager
- [x] CLI framework
- [x] Composer.json parsing
- [x] Packagist API client
- [ ] Autoloader generation
- [ ] Dependency resolution
- [ ] Package installation

## Planned 📋

### Language Features
- [ ] Generators
- [ ] Attributes
- [ ] Match expressions

### Standard Library
- [ ] Stream wrappers (HTTP, FTP)
- [ ] Regular expressions (preg_match, preg_replace)
- [ ] Session handling
- [ ] PDO/database layer

## Roadmap to Frameworks

### CodeIgniter 4
- [ ] Bootstrap (index.php → system/bootstrap.php)
- [ ] Autoloading
- [ ] Routing
- [ ] Controllers

### Drupal
- [ ] Bootstrap (index.php → core/lib/Drupal.php)
- [ ] Kernel initialization
- [ ] Module system

## Statistics

- **36 source files** in engine/
- **12 source files** in php/
- **252 tests** passing
- **63 opcodes** implemented
- **40+ built-in functions**

