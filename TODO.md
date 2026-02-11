# phprs Migration TODO

## Completed ✅

### Core Engine
- [x] Type system (Zval, strings, arrays, objects)
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
- [x] Lexer (tokenizer)
- [x] Expression parsing (arithmetic, comparison, logical, bitwise)
- [x] Statement parsing (echo, assign, return, include)
- [x] Control flow (if/else, while, for, foreach)
- [x] Function compilation and calls
- [x] Class compilation (properties, methods, constructors)
- [x] VM execution (52 opcodes)
- [x] Built-in functions (40+ functions)

### Tools
- [x] CLI interpreter (`bin/php`)
- [x] Web playground (`bin/php-server`)
- [x] Test suite (248 tests passing)
- [x] Comprehensive examples

## In Progress 🚧

### Package Manager
- [x] CLI framework
- [x] Composer.json parsing
- [x] Packagist API client
- [ ] Autoloader generation
- [ ] Dependency resolution
- [ ] Package installation

## Planned 📋

### Language Features
- [ ] Ternary operator (`?:`)
- [ ] Null coalescing (`??`)
- [ ] Namespaces
- [ ] Traits
- [ ] Generators
- [ ] Closures
- [ ] Type declarations
- [ ] Attributes

### Standard Library
- [ ] Stream wrappers (HTTP, FTP)
- [ ] Stream filters
- [ ] Regular expressions (preg_match, preg_replace)
- [ ] Session handling
- [ ] PDO/database layer

### SAPI
- [ ] FastCGI
- [ ] Apache/Nginx modules

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
- **248 tests** passing
- **52 opcodes** implemented
- **40+ built-in functions**
