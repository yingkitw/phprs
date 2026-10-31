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
- [x] Control flow (if/else, while, for, foreach; `break`/`continue` with explicit levels — `break 2`, `continue 2` — since 2026-10-14; `switch` with fallthrough/first-match-wins/position-independent `default`, and `do-while`, same date)
- [x] Function compilation and calls
- [x] Class compilation (properties, methods, constructors)
- [x] VM execution (77 opcodes, dispatch table)
- [x] Built-in functions (279+ functions — see Statistics)
- [x] Legacy `array()` constructor syntax (`array()`, `array('k' => v)`, indexed elements)
- [x] Foreach with key => value (`foreach ($a as $k => $v)`)
- [x] Chained array dimension assignment (`$a['b']['c'] = $v`)
- [x] Array append assignment (`$arr[] = $v`)
- [x] CLI/serve function table wiring (`compile_file_with_functions`, include merge)
- [x] `global $var` statement (BindGlobal opcode; script globals in user functions)
- [x] Chained object property dimension assignment (`$obj->prop['k'] = v`, `$this->data['a']['b'] = v`)
- [x] Class property defaults with constant expressions (`public $data = array()`)
- [x] try/catch/finally exception dispatch (single op-array scope; cross-function propagation now supported via `propagate_after_call` in `src/engine/vm/exception_dispatch.rs`)
- [x] Cross-function exception propagation — `throw` in a callee (function, method, static method, included file, `call_user_func` callback) with no local catch is catchable by `try` regions in the caller's frame. `finally` blocks run on normal completion, after catch, and during exception unwinding (pending exceptions re-dispatched after `FinallyEnd`).
- [x] Built-in `Exception`/`Error` family: `new Exception("msg")` stores message/code on the object so `$e->getMessage()` and `$e->getCode()` work; rethrow from a local catch propagates to the outer `try`.
- [x] `__unset` magic method (`unset($obj->prop)` calls `__unset($name)` when the property is undefined; defined properties are removed without invoking `__unset`); covered by `tests/php8x_features.rs::test_magic_unset_*`.
- [x] Direct subscripting of function-call results (`func()['key']`, `(func())['key']`)
- [x] Compound assignment operators (`+=`, `-=`, `*=`, `/=`, `%=`, `.=`) on variables, array dims (`$a['k'] .= x`, chained), and object properties (`$obj->p += x`); `??=` on variables — compiler desugaring
- [x] Bitwise operators (`&`, `|`, `^`, `~`, `<<`, `>>`) with PHP 8 precedence (`|` > `^` > `&`, shifts tighter than comparison); bitwise compound assigns (`&=`, `|=`, `^=`, `<<=`, `>>=`)
- [x] Unary `+`/`-` in expressions (`$x = -5;`, `-~$n`)
- [x] `xor` keyword operator (dispatched via `BoolXor`; note: `=` binds looser than `xor` in Zend — parenthesize when mixing)
- [x] **Power operator `**`** (right-associative, binds tighter than `*`; int result for non-negative int exponents)
- [x] **Spaceship `<=>`** (PHP 7) — new `Spaceship` opcode returning -1/0/1
- [x] Integer literal bases — `0x`/`0X` hex, `0b` binary, `0o` octal, legacy `0777` octal, `_` separators (`1_000_000`)
- [x] `echo $a, $b, $c;` (comma-separated arguments)
- [x] `(string)null === ""` — `zval_get_string` checks type before payload; missing array keys read as null
- [x] Numeric-string array key normalization — `$a["1"]` and `$a[1]` address the same slot (`hash_add_or_update`/`hash_find`/`hash_del` central; non-canonical `"01"`, `"-0"`, overflow stay strings)
- [x] Integer arithmetic type preservation — `int-int`, `int*int` stay int; `int/int` yields int when evenly divisible (`10/2 === 5`, `10/4 === 2.5`); float keys with integral values truncate to int keys

### Tools
- [x] Unified CLI (`bin/phprs`) with `run`, `serve`, `pkg` subcommands
- [x] Web playground (`phprs serve`)
- [x] Test suite
- [x] Comprehensive examples

### Performance Optimizations
- [x] JIT compilation system (for hot functions) — ⚠️ **scaffolding only**; not wired into the compile or VM execution path (reachable from `src/engine/benchmark.rs`)
- [x] Function optimizer (inlining, call optimization) — ⚠️ **scaffolding only**; not wired into the compile or VM execution path
- [x] Opcode cache with optimization passes — ⚠️ the cache's `OptimizationHints` are advisory; the real op-stream transformation lives in `src/engine/opt.rs`
- [x] Thread-safe global state using OnceLock and RwLock
- [x] Fixed mutable static reference issues for Rust 2024 compliance

### Language Features (Phase 2)
- [x] Namespaces
- [x] Traits
- [x] Attributes (PHP 8.0) — **parsed and discarded**; no attribute storage or reflection support yet
- [x] Match expressions (PHP 8.0)
- [x] Generators (yield → resumable Generator objects with rewind/valid/current/key/next/send/getReturn)

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
- [x] Example scripts adjusted for phprs limits where needed (look-ahead regex — later unlocked natively via `fancy-regex`; session demos — later backed by real `session_*` builtins)
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

### Standard Library
- [x] Stream wrappers (HTTP)
  - [x] HTTP/HTTPS stream wrapper with reqwest
  - [x] file_get_contents() HTTP support
  - [ ] FTP stream wrapper — **not implemented**
  - [ ] Custom stream contexts — **not implemented** (`stream_context_create()` does not exist)
- [x] Regular expressions (preg_match, preg_replace)
  - [x] preg_match() with capture groups
  - [x] preg_match_all() for multiple matches
  - [x] preg_replace() with pattern replacement
  - [x] preg_split() for pattern-based splitting
  - [x] PCRE flag support (i, m, s, x)
  - [x] Regex compilation and caching
- [x] Session builtins (`session_start()`, `session_destroy()`, `session_id()`, `session_name()`)
  - [x] File-backed / request-scoped session storage in `phprs serve` (JSON files, `PHPSESSID` cookie)
  - Not the full Zend session extension — see SPEC.md limitations
- [x] PDO/database layer
  - [x] PDO class with connection management
  - [x] Query execution (query(), exec())
  - [x] Prepared statements (prepare(), execute())
  - [x] Parameter binding (bindParam())
  - [x] Transactions (beginTransaction(), commit(), rollback())
  - [x] Fetch operations (fetch(), fetchAll())
  - [x] Error handling (errorInfo())
  - [x] Multiple driver support (MySQL, PostgreSQL stubs; **SQLite: real driver via rusqlite**)
- [x] **SQLite PDO driver** (real database access via `rusqlite` with bundled SQLite; `sqlite::memory:` and file paths supported)
  - [x] Connection management with global registry (atomic ID counter)
  - [x] `query()`, `exec()`, `prepare()`, `execute()` with bound parameters
  - [x] `fetch()`, `fetchAll()`, `rowCount()`, `columnCount()`, `lastInsertId()`
  - [x] Transactions: `beginTransaction()`, `commit()`, `rollBack()`
- [x] **SimpleXML support** (`simplexml_load_string`, `simplexml_load_file`, `SimpleXMLElement` class)
  - [x] Lightweight recursive-descent XML parser (no external dependency)
  - [x] Element/attribute/text access, nested children, multiple same-name children
  - [x] `asXML()`, `getName()`, `__toString()`, `count()` methods
  - [x] `html_entity_decode()`, `utf8_encode()`, `utf8_decode()`
- [x] **cURL functions** (`curl_init`, `curl_setopt`, `curl_exec`, `curl_getinfo`, `curl_close`, `curl_version`, `curl_strerror`)
- [x] **OpenSSL functions** (`openssl_digest`, `openssl_get_md_methods`, `openssl_cipher_iv_length`, `openssl_error_string`, `openssl_random_pseudo_bytes`, `openssl_encrypt`, `openssl_decrypt`)
- [x] **ctype functions** (`ctype_alnum`, `ctype_alpha`, `ctype_digit`, `ctype_lower`, `ctype_upper`, `ctype_space`, `ctype_xdigit`, `ctype_punct`, `ctype_print`, `ctype_graph`, `ctype_cntrl`)
- [x] **filter functions** (`filter_var` with integer filter ids for `VALIDATE_INT/FLOAT/BOOL/URL/EMAIL/IP/REGEXP`, `SANITIZE_STRING/NUMBER_INT/NUMBER_FLOAT/ENCODED`). ⚠️ The `FILTER_*` **constants are not defined** — pass the numeric id, otherwise the bare identifier resolves to its own name as a string. `FILTER_SANITIZE_STRING` does not strip tags.
- [x] **Additional string/array functions** (`array_key_first`, `array_key_last`, `array_is_list`, `str_word_count`, `strcoll`, `quoted_printable_decode`, `levenshtein`, `similar_text`)

## Planned 📋

### Framework Roadmap

#### CodeIgniter 4
- [x] Bootstrap (public/index.php → app/Config/Paths.php → system/bootstrap.php)
- [x] Config/Paths (SYSTEM_PATH, APP_PATH, WRITEPATH)
- [x] Constants and Autoload stubs
- [x] Routing (demo `examples/codeigniter/system/Router.php` + `app/Config/Routes.php`)
- [x] Controllers (demo `examples/codeigniter/app/Controllers/Home.php`)

#### Drupal
- [x] Bootstrap (index.php → core/includes/bootstrap.inc.php → core/lib/Drupal.php)
- [x] DRUPAL_ROOT and bootstrap constants
- [x] Drupal.php kernel stub
- [x] Full DrupalKernel (demo `examples/drupal/core/lib/DrupalKernel.php` with `boot()`)
- [x] Module system (demo `examples/drupal/core/lib/Drupal/Core/Extension/ModuleHandler.php`)

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
- **PHP runtime**: 25 modules under `src/php/` (regex, http_stream, pdo, sqlite, math, hash, datetime, mbstring, …)
- **Framework examples**: WordPress-shaped (partial), CodeIgniter 4 demo (covered by `tests/examples_runtime.rs`), Drupal demo (covered by `tests/examples_runtime.rs`)
- **77 opcodes** (76 have dispatch handlers; `AssignOp` is a reserved, never-emitted slot)
- **279+ built-in functions** — see `builtin_capability_tests.rs` for exercised surface
- **688 workspace tests** (`cargo test --workspace` — 688 passing, 1 ignored; see `tests/exception_propagation.rs`, `tests/string_interpolation.rs`, `tests/php8x_features.rs` for the new surface)
- **`break` / `continue` implemented**: compile to `Jmp` with compile-time-resolved targets (loop head for while/foreach, increment position for for-loops — the increment still runs, per PHP). Levels (`break 2`, `continue 2`) resolve against a per-`CompileContext` loop stack; named functions/closures compile with a fresh context so a stray `break` inside a function is a compile error. Misplacement (`break` outside a loop, `break 0`) is a compile error, matching Zend. Covered by `tests/loop_control.rs` (16 cases).
- **`switch` and `do-while` implemented**: `switch` compiles to an interleaved test/fallthrough `IsEqual`+`JmpNZ`/`Jmp` layout — subject evaluated once, first match wins, fallthrough without `break`, `default` is last resort regardless of source position, consecutive labels share a body. `switch` is a breakable level on the loop stack (`continue 1` in a switch acts like `break`). `do-while` reuses the loop stack with the condition as the continue target. No new opcodes. Covered by `tests/switch_statement.rs` (15 cases).
- **Post-compile optimizer passes implemented** (`src/engine/opt.rs`): constant folding + dead code elimination, applied once per compilation inside `compile_string_with_functions`. Covered by `src/engine/opt.rs` unit tests + `tests/opt_passes.rs` (11 cases).
- **Remaining newly-discovered engine gaps** (pre-existing, not caused by the new features):
  - [x] Function-call arguments stop parsing at the relational level — **fixed**: call args / ctor args / array elements / match arms now use `parse_ternary_expr_with_initial` (full precedence through ternary). Covered by `tests/call_arg_expressions.rs` and `examples/call_arg_expressions.php`.
  - Concat `.` shares precedence with `+`/`-` (PHP 7 style, `additive_loop`); PHP 8.0 made arithmetic bind tighter than concat.
  - `continue` inside a `switch` has PHP's "equivalent to break" warning behavior implemented silently (no warning emitted).
- **26 root PHP examples** — all run via `examples_root_php_scripts_all_run`
- **Known gaps** (verified during testing — tracked, not blocking):
  - 76 of the 77 opcodes have dispatch handlers. The previously no-op opcodes (`AssignObj`, `TypeCheck`, `IsSet`, `Empty`, `Count`, `Keys`, `Values`, `ArrayDiff`) are wired up, and a new `Yield` opcode (76) implements generator suspension. `AssignOp` (28) remains a reserved slot with no handler; it is never emitted.
  - Exception dispatch now supports cross-function propagation (throw in a callee caught by a caller) via `propagate_after_call`; `finally` blocks now run on normal completion, after catch, and during exception unwinding (pending exceptions re-dispatched after `FinallyEnd`). Covered by `tests/exception_propagation.rs` (18 cases, including 5 `finally` tests).
  - `__unset` implemented; `__serialize`/`__unserialize` implemented via `invoke_magic_method`/`invoke_magic_method_with_this` helpers in `callable.rs`.
  - `Val::clone()` is shallow for arrays/objects (creates an empty/default copy); engine code must use `clone_val` (deep) — a frequent source of subtle bugs for contributors.

### Standard library (honest)
- Regex via Rust `regex` + `fancy-regex` for look-around (`preg_*`); not full PCRE
- HTTP GET via `file_get_contents` + `reqwest`
- PDO: **SQLite driver is real** (`src/php/sqlite.rs` via bundled `rusqlite`). Other DSNs (`mysql:`, `pgsql:`) parse but do not connect. `bindParam`/`bindValue` are accepted but currently no-ops — use positional `execute([...])` placeholders.
- Session builtins (`session_start`, `session_destroy`, `session_id`, `session_name`) with JSON file storage; cookie support in `phprs serve`
- Math functions (20+): abs, ceil, floor, round, sqrt, pow, trig functions, max, min, rand
- Hash functions: md5, sha1, sha256, sha512, base64_encode, base64_decode
- DateTime functions: time, date, strtotime, mktime, microtime
- FTP stream wrapper — **not implemented** (no FTP code exists in the tree)

- WordPress-shaped demo: hooks/filters stubs, wpdb in-memory — **not** full core

### Standard library additions (recent)
- **Callback-driven builtins** (`src/engine/vm/callable.rs`): `array_map`, `array_filter`, `array_reduce`, `array_walk`, `call_user_func`, `call_user_func_array` invoke builtins **and** user functions (safe VM re-entry mirroring `DoFCall`)
- **Array helpers**: `array_combine`, `array_flip`, `array_search`, `array_unique`, `array_column`, `array_sum`, `array_product`, `array_chunk`, `array_diff`, `array_intersect`, `array_count_values`, `array_fill`, `array_pad`, `range`
- **String helpers**: `substr_count`, `substr_replace`, `strpbrk`, `substr_compare`, plus previously-listed-but-unimplemented functions now real: `str_repeat`, `ucwords`, `lcfirst`, `str_split`, `strrev`, `str_contains`, `str_starts_with`, `str_ends_with`, `strtr` (char + assoc), `str_ireplace`, `nl2br`, `chunk_split`, `addslashes`, `stripslashes`, `quotemeta`, `strip_tags`, `htmlspecialchars_decode`, `wordwrap`, `number_format`
- **printf family**: richer `sprintf` (`%s`/`%d`/`%f` with precision/`%x`/`%e`/`%%`) and `vsprintf`
- **Math/type helpers**: `intdiv`, `fmod`, `hypot`, `is_nan`, `is_infinite`, `is_finite`, improved `is_numeric` (numeric strings), `is_callable`, `boolval`, base conversion (`decbin`/`decoct`/`dechex`/`bindec`/`octdec`/`hexdec`/`base_convert`), `deg2rad`, `rad2deg`
- **Fuzzy string comparison**: `similar_text`, `levenshtein`, `soundex`, simplified `metaphone`
- **Serialization**: `serialize()` / `unserialize()` (`src/php/serialize.rs`) — scalars, arrays, plain objects via properties; `__serialize`/`__unserialize` magic methods supported via `invoke_magic_method`/`invoke_magic_method_with_this` helpers in `callable.rs`
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
- [x] **Magic methods** - `__unset` implemented (UnsetObjProp opcode + `unset($obj->prop)` compiler path); `__serialize`/`__unserialize` implemented via `invoke_magic_method`/`invoke_magic_method_with_this` helpers in `callable.rs`
- [x] **Anonymous classes** - `new class { ... }` with optional extends/implements
- [x] **Variadic functions** - `...$args` parameter unpacking in VM
- [x] **Named arguments** (PHP 8.0) - `func(param: value)` via `SendValNamed` opcode
- [x] **Union types** (PHP 8.0) - `int|string` parsing in params and return types
- [x] **Intersection types** (PHP 8.1) - `Countable&ArrayAccess` parsing
- [x] **Readonly properties** (PHP 8.1) - `T_READONLY` keyword + class body parsing. ⚠️ Parsing only — the `readonly` flag is **not enforced** at runtime (reassignment still succeeds).
- [x] **Enums** (PHP 8.1) - Pure and backed enums (`enum Color: string { case Red = 'red'; }`)
- [x] **First-class callable syntax** (PHP 8.1) - `strlen(...)`, `Class::method(...)`, `$obj->method(...)`
- [x] **Foreach with key** — `foreach ($a as $k => $v)`
- [x] **Complex string interpolation** (PHP double-quote syntax) — `"$name"`, simple-syntax accessors (`"$arr[key]"`, `"$arr[0]"`, `"$arr[$k]"`, `"$obj->prop"`), and complex `"{$arr['key']}"`, `"{$obj->prop}"`, `"{$obj->method()}"` (see `tests/string_interpolation.rs`)
- [x] **Array append / chained dim assign** — `$arr[] = $x`, `$a['b']['c'] = $v`
- [x] **User-defined functions in CLI scripts** — top-level `function foo()` callable from same file
- [x] **Fibers** (PHP 8.1) - Lightweight concurrency — implemented as built-in class with suspend/resume via `execute_ex_resume`
- [x] **Never type** (PHP 8.1) - recognized in type hints
- [x] **Final class constants** (PHP 8.1) - `final public const X = ...` (and `public final const X`); child override of a final constant is a compile error
- [x] **Inherited class constant / static property lookup** - `execute_fetch_static_prop` walks `parent_name` chain; `Child::PARENT_CONST` and multi-level inheritance work
- [x] **New in initializers** (PHP 8.1) - `new` in property defaults and param defaults. ⚠️ The object is constructed but the **constructor is not invoked** in these positions, and `new` in a param default yields `null`.

### Standard Library Extensions
- [x] **DateTime/DateTimeImmutable** - Basic date/time manipulation
  - [x] `date()`, `strtotime()`, `mktime()`, `time()`, `microtime()`
  - [x] `DateTime::createFromFormat()` — parses a datetime string using a format specifier (Y, y, m, n, d, j, H, G, i, s, a, A, g); returns a `DateTime` object
  - [x] `DateTime::diff()` — returns a `DateInterval` object with `y`, `m`, `d`, `h`, `i`, `s`, `days`, `invert` properties; calendar-aware difference with borrowing
  - [x] Timezone support — `DateTimeZone` built-in class; constructor accepts `DateTimeZone` as second arg; `getTimezone()`/`setTimezone()` methods; format specifiers `e`, `T`, `O`, `P`, `Z`; named zones (UTC, America/New_York, Asia/Tokyo, etc.) and numeric offsets (+0530, -08:00); DST-aware offsets via `timezone_offset_at(name, timestamp)` for US/EU/Australia zones
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
- [x] **XML parsing** - SimpleXML (simplexml_load_string, simplexml_load_file, SimpleXMLElement class) — XMLReader, XMLWriter, DOM still pending
- [x] **CSV handling** - `fgetcsv()`, `fputcsv()`, `str_getcsv()`
- [x] **Compression** - `gzcompress()`, `gzuncompress()`, `gzencode()`, `gzdecode()`, `gzdeflate()`, `gzinflate()`
- [ ] **Image processing** - GD library basics (create, resize, crop, filters)
- [ ] **Mail functions** - `mail()` with SMTP support
- [x] **Crypt functions** — `openssl_encrypt()`/`openssl_decrypt()` are implemented via `src/php/openssl.rs` (`openssl_encrypt`, `openssl_decrypt` builtins in `builtins.rs`)
  - [x] `random_bytes()`, `random_int()`

### Advanced Features
- [x] **Reflection API** (basic) - `ReflectionClass`, `ReflectionMethod`, `ReflectionProperty` with `getName()`, `getMethods()`, `getProperties()`, `hasMethod()`, `hasProperty()`, `getParentClass()`, `getDeclaringClass()`
- [x] **Reflection API** (extended) - `ReflectionFunction` (`getName`, `getParameters`, `getNumberOfParameters`, `isBuiltin`, `isUserDefined`), `ReflectionParameter` (`getName`, `getPosition`, `getDeclaringFunction`), and `ReflectionMethod::getParameters()` / `getNumberOfParameters()`
- [ ] **Reflection API** (remaining) - `ReflectionExtension`, typed-parameter reflection, full attribute reflection
- [x] **SPL (Standard PHP Library)**
  - [x] ArrayIterator (construction, current/key/next/rewind/valid, count, ArrayAccess methods)
  - [x] Data structures (SplStack, SplQueue, SplHeap/SplMaxHeap/SplMinHeap, SplPriorityQueue — push/pop/enqueue/dequeue/top/bottom/count/isEmpty/insert/extract)
  - [x] DirectoryIterator (construction, valid/current/getFilename/getPathname/getPath/key/next/rewind)
  - [x] Exceptions (SPL exception hierarchy — RuntimeException, LogicException, InvalidArgumentException, BadMethodCallException, OutOfBoundsException, OverflowException, UnderflowException, UnexpectedValueException, RangeException, DomainException, LengthException, BadFunctionCallException)
  - [x] File handling (SplFileInfo — getPath/getFilename/getPathname/getRealPath/getSize/isFile/isDir/isReadable/isWritable; SplFileObject — fgets/fread/fwrite/feof/rewind)
- [x] **Autoloading** - `spl_autoload_register()`, `spl_autoload_unregister()`, `spl_autoload_functions()` (runtime registration); PSR-4 autoloader generation (package manager). ⚠️ Registered autoloaders are **never invoked** by class resolution.
- [x] **Error handling improvements**
  - [x] Custom error handlers (`set_error_handler()`) — ⚠️ the callback is **recorded but never invoked**
  - [x] Exception handlers (`set_exception_handler()`) — ⚠️ the callback is **recorded but never invoked**
  - [x] Shutdown functions (`register_shutdown_function()`) — ⚠️ the callback is **recorded but never invoked**
- [x] **Output buffering enhancements**
  - [x] `ob_start()` with callbacks (builtin callback invocation on `ob_end_flush`/`ob_get_flush`) — ⚠️ only **builtin** callbacks resolve; user-defined PHP functions are silently ignored
  - [x] `ob_get_clean()`, `ob_get_flush()`, `ob_get_level()`
  - [x] Multiple buffer levels

### Performance & Optimization
- [ ] **LLVM-based JIT** - Replace custom JIT with LLVM for better optimization
- [ ] **Opcode optimization passes** — constant folding and DCE are done; loop unrolling and TCO remain
  - [x] **Constant propagation / constant folding** (`src/engine/opt.rs`) — pure ops over literal operands are evaluated at compile time by running the real dispatch handlers on a scratch `ExecuteData` (zero semantic duplication), rewritten op-count-neutrally to `QmAssign <literal> -> <result slot>`; folded temp values propagate into later foldable ops within a basic block (`2 + 3 * 4` → one literal). `ConstantAst` operands (bare identifiers, may be `define()`d later) and division/modulo by zero are never folded. Covers script bodies, functions, closures, and class methods.
  - [x] **Dead code elimination** (`src/engine/opt.rs`) — ops unreachable from entry or any jump target are removed; `extended_value` targets on `Jmp`/`JmpZ`/`JmpNZ`/`JmpNullZ`/`FeFetch`/`TryCatchBegin` are renumbered; exception structural markers are always kept (runtime derives catch/finally bodies from their positions).
  - [ ] Loop unrolling
  - [ ] Tail call optimization
- [ ] **Memory pool improvements** - Better allocation strategies
- [x] **String interning** - `src/engine/string_intern.rs` (`StringInterner` / global `intern()`); dedup-by-content with pointer-equality handles. ⚠️ The interner is **not yet called from the engine** — it is currently unreferenced outside its own module.
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
  - [x] SQLite native driver (via rusqlite with bundled SQLite)
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
- [x] **cURL wrapper** - Basic cURL API (curl_init, curl_setopt, curl_exec, curl_getinfo, curl_close, curl_version, curl_strerror) — full compatibility still pending
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

