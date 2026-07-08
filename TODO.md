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
- [x] VM execution (73 opcodes, dispatch table)
- [x] Built-in functions (160+ functions — see Statistics)
- [x] Legacy `array()` constructor syntax (`array()`, `array('k' => v)`, indexed elements)
- [x] Foreach with key => value (`foreach ($a as $k => $v)`)
- [x] Chained array dimension assignment (`$a['b']['c'] = $v`)
- [x] Array append assignment (`$arr[] = $v`)
- [x] CLI/serve function table wiring (`compile_file_with_functions`, include merge)
- [x] `global $var` statement (BindGlobal opcode; script globals in user functions)
- [x] Chained object property dimension assignment (`$obj->prop['k'] = v`, `$this->data['a']['b'] = v`)
- [x] Class property defaults with constant expressions (`public $data = array()`)

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
- [x] require/include: cwd-first then script-dir resolution; caller state restored after include
- [x] dirname(), exit(), die(); do_action(), apply_filters() stubs

### Testing & examples
- [x] `tests/examples_runtime.rs` — auto matrix for all root `examples/*.php`
- [x] `tests/build_rust_examples.rs` — `examples/rust/*.rs` compile
- [x] `src/engine/vm/builtin_capability_tests.rs` — broad builtin coverage
- [x] [PERFORMANCE.md](PERFORMANCE.md) evidence policy (phprs-only benchmarks; no fake PHP baselines)
- [x] Example scripts adjusted for phprs compiler limits (regex lookahead, session simulation, foreach value-only)
- [x] Look-ahead/look-behind regex via `fancy-regex` fallback in `preg_*`
- [x] Session builtins + file-backed storage for `phprs serve`
- [x] First-class callable syntax (`strlen(...)`, static/instance method forms)

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
- [x] Session **demo patterns** (`examples/session-examples.php` — plain `$_SESSION` array; not Zend session extension)
  - [x] `session_start()`, `session_destroy()`, `session_id()` as engine builtins
  - [x] File-backed / request-scoped session storage in `phprs serve`
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
- [x] Bootstrap (public/index.php → app/Config/Paths.php → system/bootstrap.php)
- [x] Config/Paths (SYSTEM_PATH, APP_PATH, WRITEPATH)
- [x] Constants and Autoload stubs
- [ ] Routing (future)
- [ ] Controllers (future)

#### Drupal
- [x] Bootstrap (index.php → core/includes/bootstrap.inc.php → core/lib/Drupal.php)
- [x] DRUPAL_ROOT and bootstrap constants
- [x] Drupal.php kernel stub
- [ ] Full DrupalKernel (future)
- [ ] Module system (future)

#### WordPress
- [x] Bootstrap (index.php → wp-blog-header.php → wp-load.php → wp-config.php → wp-settings.php)
- [x] wp-config-style constants (ABSPATH, WP_DEBUG; define/defined/constant, __DIR__, __FILE__)
- [x] Relative include resolution; include restores caller state
- [x] Minimal example in examples/wordpress (full bootstrap runs: index.php → wp-settings → plugins/theme hooks)
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
  - [x] WordPress demo session stubs (`wp_session_*` in `examples/wordpress/` only)
  - [x] Example plugin with activation hooks and filters
  - [x] Example theme with functions.php and theme setup
  - [x] `call_user_func_array` invokes string callbacks; `ksort` / `add_shortcode` stubs in plugin.php

## Statistics

### Implementation stats (current)
- **Engine**: types, string, hash, alloc, gc, operators, compile, vm, jit, benchmark, …
- **PHP runtime**: modules under `src/php/` (regex, http_stream, pdo stub, math, hash, datetime, mbstring, …)
- **Framework examples**: WordPress-shaped (partial), CodeIgniter 4 demo (CI-tested), Drupal demo (CI-tested)
- **73 opcodes** (dispatch table)
- **195+ built-in functions** — see `builtin_capability_tests.rs` for exercised surface
- **485+ workspace tests** (`cargo test --workspace`)
- **23 root PHP examples** — all run via `examples_root_php_scripts_all_run`
- **Known gaps** (verified during testing — tracked, not blocking):
  - **Exceptions are not wired into the VM dispatch.** `throw`, `try`/`catch`/`finally` opcodes (`Throw`, `TryCatchBegin/End`, `CatchBegin/End`, `FinallyBegin/End`) are **no-ops** in the real dispatch table — `src/engine/vm/handlers.rs` has the logic but is unused dead code, and `execute_ex` only dispatches ~50 of the 73 opcodes. The `exception.rs` state machine (`ExceptionState`, `TryCatchBlock`) is never triggered, so `throw new X()` silently continues and `catch` blocks never run. Unit tests pass because they exercise `ExceptionState` directly. **This is the highest-priority correctness gap.**
  - Other un-dispatched opcodes (no-ops today): bitwise (`Sl`, `Sr`, `BwOr/And/Xor/Not`), `BoolXor`, `AssignOp` (compound `+=` etc. via opcode), `AssignObj`, `TypeCheck`, `Unset`, `IsSet`, `Empty`, `Count`, `Keys`, `Values`, `ArrayDiff`. Several are covered by their builtin equivalents (`isset`/`empty`/`count`/`unset` work as function calls), but the opcode-level forms do nothing.
  - `func()['key']` / `(func())['key']` — subscripting a function-call result directly returns the whole array instead of the element. **Workaround:** assign the return value to a variable first (`$x = func(); $x['key']`).
  - Numeric-string array keys are not normalized to integers (PHP stores `'1'` as integer key `1`); lookups by the other form may miss.
  - `Val::clone()` is shallow for arrays/objects (creates an empty/default copy); engine code must use `clone_val` (deep) — a frequent source of subtle bugs for contributors.

### Standard library (honest)
- Regex via Rust `regex` + `fancy-regex` for look-around (`preg_*`); not full PCRE
- HTTP GET via `file_get_contents` + `reqwest`
- PDO **stub** (in-memory)
- Session builtins (`session_start`, `session_destroy`, `session_id`, `session_name`) with JSON file storage; cookie support in `phprs serve`
- Math functions (20+): abs, ceil, floor, round, sqrt, pow, trig functions, max, min, rand
- Hash functions: md5, sha1, sha256, sha512, base64_encode, base64_decode
- DateTime functions: time, date, strtotime, mktime, microtime
- FTP stream wrapper (stub)

- WordPress-shaped demo: hooks/filters stubs, wpdb in-memory — **not** full core

### Standard library additions (recent)
- **Callback-driven builtins** (`src/engine/vm/callable.rs`): `array_map`, `array_filter`, `array_reduce`, `array_walk`, `call_user_func`, `call_user_func_array` invoke builtins **and** user functions (safe VM re-entry mirroring `DoFCall`)
- **Array helpers**: `array_combine`, `array_flip`, `array_search`, `array_unique`, `array_column`, `array_sum`, `array_product`, `array_chunk`, `array_diff`, `array_intersect`, `array_count_values`, `array_fill`, `array_pad`, `range`
- **String helpers**: `substr_count`, `substr_replace`, `strpbrk`, `substr_compare`, plus previously-listed-but-unimplemented functions now real: `str_repeat`, `ucwords`, `lcfirst`, `str_split`, `strrev`, `str_contains`, `str_starts_with`, `str_ends_with`, `strtr` (char + assoc), `str_ireplace`, `nl2br`, `chunk_split`, `addslashes`, `stripslashes`, `quotemeta`, `strip_tags`, `htmlspecialchars_decode`, `wordwrap`, `number_format`
- **printf family**: richer `sprintf` (`%s`/`%d`/`%f` with precision/`%x`/`%e`/`%%`) and `vsprintf`
- **Math/type helpers**: `intdiv`, `fmod`, `hypot`, `is_nan`, `is_infinite`, `is_finite`, improved `is_numeric` (numeric strings), `is_callable`, `boolval`, base conversion (`decbin`/`decoct`/`dechex`/`bindec`/`octdec`/`hexdec`/`base_convert`), `deg2rad`, `rad2deg`
- **Fuzzy string comparison**: `similar_text`, `levenshtein`, `soundex`, simplified `metaphone`
- **Serialization**: `serialize()` / `unserialize()` (`src/php/serialize.rs`) — scalars, arrays, plain objects via properties; `__serialize`/`__unserialize` method hooks still pending (require method invocation from builtins)
- **Bug fix**: void builtins returning `null` (e.g. `var_dump`, `echo`, `unset`) no longer emit spurious "Call to undefined function" warnings — `DoFCall` now treats a known builtin returning `None` as a successful void call

## Rust host advantages (engineering, not product guarantees)

Rust is used for the **interpreter implementation** because of memory safety in safe code, strong tooling (`cargo test`, clippy), and LLVM for the host binary. That does **not** automatically make every PHP workload faster or more secure end-to-end — measure your scripts or read [PERFORMANCE.md](PERFORMANCE.md).

## Code Quality Improvements (Completed)

### Rust 2024 Compliance ✅
- [x] Fix unsafe blocks in unsafe functions (alloc.rs, gc.rs)
- [x] Remove unused imports and dead code
- [x] Fix visibility issues with ExecResult type
- [x] Add missing safety comments for unsafe operations

### Memory Management ✅
- [x] Improve realloc implementation with proper size tracking
- [x] Optimize allocation patterns
- [ ] Add memory leak detection (future improvement)

### Code Cleanup ✅
- [x] Remove unreachable patterns and unused variables
- [x] Fix documentation for macro invocations
- [x] Standardize error handling patterns

#### Summary of Changes:
- Fixed all 88 compilation warnings, now builds cleanly with zero warnings
- Improved realloc implementation with size tracking for better performance
- Enhanced safety documentation for all unsafe operations
- Proper visibility fixes for public API consistency
- Removed dead code while preserving intentionally unused functions with #[allow(dead_code)]

## New Capabilities (Brainstormed) 🚀

### Core Language Features
- [x] **Static properties and methods** - Full static member support with `ClassName::$prop`, `ClassName::method()`, `static::`
- [x] **Late static binding** (`static::` keyword, runtime resolution via `called_class`)
- [x] **Magic methods** - `__get`, `__set`, `__call`, `__callStatic`
- [x] **Magic methods** (remaining) - `__toString`, `__invoke`, `__clone`
- [x] **Magic methods** - `__isset` (called from FetchObjProp before __get)
- [x] **Magic methods** (partial) - `__debugInfo` (object dumping in var_dump; method invocation requires ExecuteData in builtins)
- [ ] **Magic methods** (pending) - `__unset` (requires compiler support for `unset($obj->prop)`), `__serialize`, `__unserialize`
- [x] **Anonymous classes** - `new class { ... }` with optional extends/implements
- [x] **Variadic functions** - `...$args` parameter unpacking in VM
- [x] **Named arguments** (PHP 8.0) - `func(param: value)` via `SendValNamed` opcode
- [x] **Union types** (PHP 8.0) - `int|string` parsing in params and return types
- [x] **Intersection types** (PHP 8.1) - `Countable&ArrayAccess` parsing
- [x] **Readonly properties** (PHP 8.1) - `T_READONLY` keyword + class body parsing
- [x] **Enums** (PHP 8.1) - Pure and backed enums (`enum Color: string { case Red = 'red'; }`)
- [x] **First-class callable syntax** (PHP 8.1) - `strlen(...)`, `Class::method(...)`, `$obj->method(...)`
- [x] **Foreach with key** — `foreach ($a as $k => $v)`
- [x] **Array append / chained dim assign** — `$arr[] = $x`, `$a['b']['c'] = $v`
- [x] **User-defined functions in CLI scripts** — top-level `function foo()` callable from same file
- [ ] **Fibers** (PHP 8.1) - Lightweight concurrency
- [x] **Never type** (PHP 8.1) - recognized in type hints
- [ ] **Final class constants** (PHP 8.1)
- [x] **New in initializers** (PHP 8.1) - `new` in property defaults and param defaults

### Standard Library Extensions
- [x] **DateTime/DateTimeImmutable** - Basic date/time manipulation
  - [x] `date()`, `strtotime()`, `mktime()`, `time()`, `microtime()`
  - [ ] `DateTime::createFromFormat()`, `DateTime::diff()`
  - [ ] Timezone support
- [x] **Math functions** - `abs()`, `ceil()`, `floor()`, `round()`, `sqrt()`, `pow()`, `exp()`, `log()`, `log10()`, `sin()`, `cos()`, `tan()`, `asin()`, `acos()`, `atan()`, `atan2()`, `pi()`, `max()`, `min()`, `rand()`
- [x] **Hash functions** - `md5()`, `sha1()`, `hash()`, `base64_encode()`, `base64_decode()`
  - [x] `hash_hmac()`, `password_hash()`, `password_verify()`
- [x] **URL functions** - `parse_url()`, `http_build_query()`, `urlencode()`, `urldecode()`, `rawurlencode()`, `rawurldecode()`, `parse_str()`, `get_headers()`
- [x] **Multibyte string** - `mb_strlen()`, `mb_substr()`, `mb_strtolower()`, `mb_strtoupper()`, `mb_convert_encoding()`
  - [x] mb_strlen() with Unicode grapheme cluster support
  - [x] mb_substr() with proper Unicode handling
  - [x] mb_strtolower() and mb_strtoupper()
  - [x] mb_strpos() and mb_strrpos()
  - [x] mb_convert_encoding() (basic UTF-8 support)
  - [x] mb_substr_count()
  - [x] mb_strwidth() and mb_strimwidth()
- [x] **Introspection functions** - `class_exists()`, `interface_exists()`, `trait_exists()`, `method_exists()`, `property_exists()`, `function_exists()`, `get_class()`, `get_parent_class()`, `gettype()`
- [ ] **XML parsing** - SimpleXML, XMLReader, XMLWriter
- [x] **CSV handling** - `fgetcsv()`, `fputcsv()`, `str_getcsv()`
- [x] **Compression** - `gzcompress()`, `gzuncompress()`, `gzencode()`, `gzdecode()`, `gzdeflate()`, `gzinflate()`
- [ ] **Image processing** - GD library basics (create, resize, crop, filters)
- [ ] **Mail functions** - `mail()` with SMTP support
- [ ] **Crypt functions** - `openssl_encrypt()`, `openssl_decrypt()`
  - [x] `random_bytes()`, `random_int()`

### Advanced Features
- [x] **Reflection API** (basic) - `ReflectionClass`, `ReflectionMethod`, `ReflectionProperty` with `getName()`, `getMethods()`, `getProperties()`, `hasMethod()`, `hasProperty()`, `getParentClass()`, `getDeclaringClass()`
- [x] **Reflection API** (extended) - `ReflectionFunction` (`getName`, `getParameters`, `getNumberOfParameters`, `isBuiltin`, `isUserDefined`), `ReflectionParameter` (`getName`, `getPosition`, `getDeclaringFunction`), and `ReflectionMethod::getParameters()` / `getNumberOfParameters()`
- [ ] **Reflection API** (remaining) - `ReflectionExtension`, typed-parameter reflection, full attribute reflection
- [ ] **SPL (Standard PHP Library)**
  - [ ] Iterators (ArrayIterator, DirectoryIterator, RecursiveDirectoryIterator)
  - [ ] Data structures (SplStack, SplQueue, SplHeap, SplPriorityQueue)
  - [ ] Exceptions (SPL exception hierarchy)
  - [ ] File handling (SplFileObject, SplFileInfo)
- [x] **Autoloading** - `spl_autoload_register()`, `spl_autoload_unregister()`, `spl_autoload_functions()` (runtime registration); PSR-4 autoloader generation (package manager)
- [x] **Error handling improvements**
  - [x] Custom error handlers (`set_error_handler()`)
  - [x] Exception handlers (`set_exception_handler()`)
  - [x] Shutdown functions (`register_shutdown_function()`)
- [x] **Output buffering enhancements**
  - [x] `ob_start()` with callbacks (builtin callback invocation on `ob_end_flush`/`ob_get_flush`)
  - [x] `ob_get_clean()`, `ob_get_flush()`, `ob_get_level()`
  - [x] Multiple buffer levels

### Performance & Optimization
- [ ] **LLVM-based JIT** - Replace custom JIT with LLVM for better optimization
- [ ] **Opcode optimization passes**
  - [ ] Constant propagation
  - [ ] Dead code elimination
  - [ ] Loop unrolling
  - [ ] Tail call optimization
- [ ] **Memory pool improvements** - Better allocation strategies
- [x] **String interning** - `src/engine/string_intern.rs` (`StringInterner` / global `intern()`); dedup-by-content with pointer-equality handles. Building block; not yet forced into every hot path
- [ ] **Copy-on-write arrays** - Optimize array copying
- [ ] **Lazy evaluation** - Defer computation until needed
- [ ] **Parallel execution** - Multi-threaded script execution
- [ ] **Profiling tools** - Built-in profiler with flame graphs

### Framework Support
- [ ] **Laravel** (High Priority)
  - [ ] Routing system (Route facade, controller routing)
  - [ ] Eloquent ORM (Model, Query Builder, relationships)
  - [ ] Blade templating engine
  - [ ] Service container and dependency injection
  - [ ] Middleware support
  - [ ] Artisan CLI commands
  - [ ] Database migrations
  - [ ] Validation
  - [ ] Authentication scaffolding
- [ ] **Symfony**
  - [ ] HTTP Foundation (Request, Response)
  - [ ] HTTP Kernel
  - [ ] Routing component
  - [ ] Dependency injection container
  - [ ] Twig templating
  - [ ] Console component
  - [ ] Event dispatcher
- [ ] **Slim Framework** - Lightweight microframework support
- [ ] **Lumen** - Laravel micro-framework
- [ ] **Yii2** - Full-stack framework support

### Database & Caching
- [ ] **Real database drivers**
  - [ ] MySQL/MariaDB native driver
  - [ ] PostgreSQL native driver  
  - [ ] SQLite native driver
  - [ ] Connection pooling
  - [ ] Prepared statement caching
- [ ] **Redis support**
  - [ ] Redis client
  - [ ] Session storage backend
  - [ ] Cache backend
  - [ ] Pub/Sub support
- [ ] **Memcached support**
  - [ ] Memcached client
  - [ ] Cache backend
- [ ] **ORM improvements**
  - [ ] Query builder enhancements
  - [ ] Relationship loading strategies
  - [ ] Database migrations

### Web & Networking
- [ ] **HTTP/2 support** - Native HTTP/2 client and server
- [ ] **HTTP/3 (QUIC)** - Experimental HTTP/3 support
- [ ] **WebSocket support** - WebSocket client and server
- [ ] **GraphQL** - GraphQL query execution
- [ ] **gRPC** - gRPC client and server
- [ ] **SOAP client** - SOAP web services
- [ ] **cURL wrapper** - Full cURL API compatibility
- [ ] **Email parsing** - MIME message parsing
- [ ] **OAuth2** - OAuth2 client implementation

### Developer Tools
- [ ] **REPL (Interactive Shell)** - Full-featured PHP REPL
  - [ ] Syntax highlighting
  - [ ] Auto-completion
  - [ ] History support
  - [ ] Multi-line editing
- [ ] **Debugger** - Step-through debugging
  - [ ] Breakpoints
  - [ ] Variable inspection
  - [ ] Call stack navigation
  - [ ] Watch expressions
  - [ ] DBGp protocol support (Xdebug compatible)
- [ ] **Profiler** - Performance profiling
  - [ ] Function call timing
  - [ ] Memory usage tracking
  - [ ] Flame graph generation
  - [ ] Cachegrind output format
- [ ] **Static analyzer** - Code quality analysis
  - [ ] Type checking
  - [ ] Dead code detection
  - [ ] Complexity metrics
  - [ ] Security vulnerability scanning
- [ ] **Code formatter** - PSR-12 compliant formatter
- [ ] **Documentation generator** - PHPDoc to HTML/Markdown
- [ ] **Test runner** - PHPUnit compatible test runner
- [ ] **Code coverage** - Test coverage analysis

### Build & Deployment
- [ ] **Phar support** - Create and execute Phar archives
- [ ] **WebAssembly compilation** - Compile PHP to WASM
- [ ] **Native binary compilation** - AOT compilation to native code
- [ ] **Docker integration** - Optimized Docker images
- [ ] **Kubernetes support** - K8s deployment helpers
- [ ] **Serverless adapters** - AWS Lambda, Google Cloud Functions
- [ ] **Hot reload** - Development server with hot reload
- [ ] **Asset bundling** - Built-in asset pipeline

### Security
- [ ] **Sandbox mode** - Restricted execution environment
- [ ] **Security policies** - Fine-grained permission control
- [ ] **Input validation** - Built-in validation library
- [ ] **CSRF protection** - Token generation and validation
- [ ] **XSS prevention** - Auto-escaping templates
- [ ] **SQL injection prevention** - Query parameterization enforcement
- [ ] **Rate limiting** - Built-in rate limiter
- [ ] **Content Security Policy** - CSP header generation

### Testing & Quality
- [ ] **Unit testing framework** - Built-in test framework
- [ ] **Integration testing** - HTTP testing helpers
- [ ] **Mocking library** - Mock objects and stubs
- [ ] **Assertion library** - Rich assertion methods
- [ ] **Snapshot testing** - Visual regression testing
- [ ] **Property-based testing** - QuickCheck-style testing
- [ ] **Mutation testing** - Test quality analysis

### Ecosystem Integration
- [ ] **npm/yarn integration** - JavaScript package management
- [ ] **Composer v2 features** - Full Composer v2 support
- [ ] **Git integration** - Built-in Git operations
- [ ] **CI/CD helpers** - GitHub Actions, GitLab CI integration
- [ ] **Monitoring integration** - Prometheus, Grafana metrics
- [ ] **Logging standards** - PSR-3 logger interface
- [ ] **APM integration** - New Relic, DataDog support

### Experimental Features
- [ ] **FFI (Foreign Function Interface)** - Call C libraries directly
- [ ] **JIT to GPU** - Offload computation to GPU
- [ ] **Distributed computing** - Multi-node execution
- [ ] **Machine learning** - Basic ML primitives
- [ ] **Blockchain integration** - Smart contract execution
- [ ] **Quantum computing** - Quantum algorithm simulation

## Documentation

- [SPEC.md](SPEC.md) - Project specification and scope
- [PERFORMANCE.md](PERFORMANCE.md) - Evidence policy and optimization notes (not benchmark marketing)

