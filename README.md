# phprs

**Modernizing PHP with Rust** — a memory-safe PHP interpreter and toolchain written in Rust. Performance is a goal; rigorous, published comparisons against PHP are not claimed here yet.

[![Rust](https://img.shields.io/badge/rust-2024-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)]()
[![Tests](https://img.shields.io/badge/tests-485%2B%20passing-brightgreen.svg)]()

## Why phprs? The Rust Advantage

PHP powers much of the web; many production runtimes are implemented in C and C++. **phprs** is an experiment in implementing PHP semantics in Rust so we can lean on the borrow checker and modern tooling. It aims to be:

- **Safer by construction (Rust)**: Memory errors that plague C/C++ code are largely ruled out in safe Rust; the interpreter still has correctness and parity work ahead.
- **A performance-minded design**: Opcode dispatch, JIT hooks, and LLVM for the host binary — without promising a given speedup over Zend until we publish reproducible benchmarks.
- **Concurrency-friendly host code**: Rust’s type system helps avoid data races in the engine itself; PHP’s shared mutable runtime model is still evolving in phprs.
- **Test-backed**: 485+ workspace tests; every root `examples/*.php` runs in `tests/examples_runtime.rs`; Rust demos compile via `build_rust_examples`.

**phprs** brings PHP into the future by:

### 🛡️ **Memory Safety - The Rust Guarantee**
**Traditional PHP (C-based) Problems:**
- ❌ Memory leaks from manual allocation/deallocation
- ❌ Buffer overflows leading to security exploits
- ❌ Use-after-free vulnerabilities
- ❌ Dangling pointers causing crashes
- ❌ Segmentation faults in production

**What Rust gives the codebase:**
- Strong **memory safety** guarantees for safe code (no dangling pointers from the borrow checker’s rules).
- **Bounds-checked** access patterns by default, reducing classic buffer overruns.
- **`Option` and `Result`** instead of nullable pointers everywhere.
- **`unsafe` is explicit** and should stay rare and reviewed.

This does **not** automatically make every PHP script “more secure” end-to-end — it raises the bar for the **interpreter implementation** itself. Application security still depends on how you deploy and what you run.

### ⚡ **Performance — goals, not guarantees**
Rust is a **good host language** for a VM: LLVM, predictable allocation patterns, and room to optimize hot paths. **phprs** includes pieces like direct opcode dispatch and JIT-oriented hooks, but **we do not publish head-to-head PHP 8.x numbers here** — workloads and completeness differ too much for a fair slogan.

If you want the engineering direction (dispatch, JIT ideas, etc.), see [PERFORMANCE.md](PERFORMANCE.md). Treat it as **design notes**, not a audited benchmark report, until we link reproducible methodology and results.

### 🔒 **Thread Safety - Fearless Concurrency**
**Traditional PHP Limitations:**
- ❌ No true multi-threading support
- ❌ Process-based concurrency (high memory overhead)
- ❌ Race conditions in extensions
- ❌ Global state issues

**Rust (host code):**
- The borrow checker and `Send` / `Sync` rules catch many concurrency mistakes **at compile time** in the Rust parts of the project.
- Primitives like `Arc`, mutexes, and `OnceLock` are the usual tools for shared engine state.

**PHP scripts** under phprs still run through a single runtime model; don’t assume “multi-threaded PHP” parity with extensions or Zend here.

### 🌐 **Framework Support**
Progressive compatibility with popular PHP frameworks and CMSs (stubs and demos in `examples/`):
- **WordPress** 🚧 - Hooks, wpdb stubs, plugin/theme loading (`examples/wordpress/`) — bootstrap blocked on `array()` syntax in nested stubs; not production-ready
- **CodeIgniter 4** ✅ - Minimal bootstrap + routed controller demo (`examples/codeigniter/`, `tests/examples_runtime.rs`)
- **Drupal** ✅ - Minimal kernel/bootstrap stub (`examples/drupal/`, `tests/examples_runtime.rs`)
- **Laravel** 📋 - Routing, Eloquent, Blade (planned)
- **Symfony** 📋 - HTTP kernel, DI (planned)

### 🛠️ **Modern Ecosystem - Rust Crate Integration**
**Leveraging crates.io:**
- **HTTP**: `reqwest` for HTTP/HTTPS from the host
- **Regex**: the `regex` crate (different tradeoffs vs PCRE — not a drop-in performance claim)
- **Crypto / hashing**: common Rust crates for checksums and tooling
- **PDO / JSON / regex / HTTP**: implemented or stubbed to varying degrees — see `builtin_capability_tests.rs` and tests for what is real today
- **Sessions**: `session_start`, `session_destroy`, `session_id`, `session_name` — JSON file storage; `phprs serve` sends `PHPSESSID` cookie
- **Package manager**: Composer-oriented workflows with `semver` for version parsing
- **Dev server**: `phprs serve` for local tries

There is a **huge ecosystem** of maintained Rust libraries; phprs only uses a small slice.

### 🎯 **Standalone & Embeddable - Rust's Portability**
**Deployment:**
- **Single binary** (typical Rust workflow): fewer moving parts than a full PHP build with many extensions.
- Image size and static linking depend on **how you package** the CLI — compare measurements for your own Dockerfile, don’t trust a slogan.
- **Cross-compilation** is possible in principle; CI mainly exercises tier-1 targets you care about.
- **Library crate**: embed from Rust; FFI to other languages is possible but not the focus of this README.
- **WASM / every platform**: aspirational until documented.

Rust helps with **predictable builds**; it doesn’t magically shrink every deployment.

## Quick Start

### Installation

```bash
# Clone the repository
git clone https://github.com/yingkitw/phprs.git
cd phprs

# Build library + CLI (workspace)
cargo build --release

# CLI binary: target/release/phprs (from `phprs-cli` workspace member)
```

### Usage - It's That Simple!

**Run any PHP file** - Zero configuration needed:
```bash
# Run a PHP script
phprs run script.php

# Or use cargo during development
cargo run -p phprs-cli -- run script.php
```

**Start development server**:
```bash
# Built-in web server on port 3080
phprs serve

# Custom port
phprs serve --port 8080
```

**Package management**:
```bash
# Initialize composer.json
phprs pkg init

# Install dependencies
phprs pkg install

# Add a package
phprs pkg require vendor/package
```

### Running code written for PHP

phprs targets **PHP language compatibility**, but many extensions, ini settings, and edge cases differ from Zend PHP. **Try your script and fix gaps** — we don’t promise bit-for-bit behavior yet.

```php
<?php
// Example that may run once the engine supports the features you use
class User {
    public function __construct(
        private string $name,
        private string $email
    ) {}
    
    public function greet(): string {
        return "Hello, {$this->name}!";
    }
}

$user = new User('John', 'john@example.com');
echo $user->greet();
```

```bash
cargo run -p phprs-cli -- run your-app.php
```

**WordPress / Laravel / Symfony**: not production migration targets today — use the **`examples/`** trees and [TODO.md](TODO.md) to see what is implemented. Large apps should expect **porting work**.

## Examples

### Basic PHP Script
```php
<?php
// examples/01_hello_world.php
echo "Hello from phprs!\n";

// Many PHP 8 features work; unsupported ones fail at compile or runtime
$numbers = [1, 2, 3, 4, 5];
$squared = array_map(fn($n) => $n ** 2, $numbers);
print_r($squared);
```

### Regular Expressions
```php
<?php
// Email validation (Rust regex engine — not full PCRE)
$email = "user@example.com";
if (preg_match('/^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$/', $email)) {
    echo "Valid email!\n";
}
$text = "Hello World";
echo preg_replace('/World/', 'phprs', $text);
```

### HTTP Streams
```php
<?php
// Fetch data from APIs
$json = file_get_contents('https://api.example.com/data');
$data = json_decode($json, true);

// Works with any HTTP/HTTPS URL
$html = file_get_contents('https://example.com');
```

### Database with PDO
```php
<?php
// Connect to database
$pdo = new PDO('mysql:host=localhost;dbname=myapp', 'user', 'pass');

// Prepared statements (SQL injection safe)
$stmt = $pdo->prepare('SELECT * FROM users WHERE email = :email');
$stmt->bindParam(':email', $email);
$stmt->execute();

$user = $stmt->fetch();
```

### Session-style state (simulated)
```php
<?php
// session_start() is not a builtin yet — use $_SESSION as a variable
$_SESSION = [];
$_SESSION['user_id'] = 123;
$_SESSION['username'] = 'john_doe';
if (isset($_SESSION['user_id'])) {
    echo "Welcome back, " . $_SESSION['username'];
}
```

### WordPress-shaped plugin (demo stubs)
```php
<?php
// Runs only in the examples/wordpress tree with phprs stubs — not stock WordPress core
add_action('init', function() {
    echo "init hook\n";
});
```

### Integration smoke script
See `examples/integration-test.php` for a short runnable script that exercises PDO stubs, regex, password helpers, and JSON under phprs.

**More Examples** (all root `examples/*.php` covered by `cargo test --test examples_runtime`):
- `examples/stdlib_callbacks.php` - Callback builtins (`array_map`/`array_filter`/`call_user_func`), new array/string/math helpers, and the Reflection API
- `examples/strings_and_serialize.php` - String helpers, printf precision, base conversion, fuzzy comparison, and `serialize`/`unserialize`
- `examples/control_flow.php` - if/switch, **for**, **while**, **foreach** (see `tests/examples_runtime.rs`)
- `examples/mbstring.php` - Multibyte string helpers (`mb_*` subset)
- `examples/match_expression.php` - `match` expressions
- `examples/regex-examples.php` - Regex patterns
- `examples/http-stream-examples.php` - HTTP streams
- `examples/session-examples.php` - Simulated `$_SESSION` patterns (no `session_*` builtins)
- `examples/pdo-examples.php` - PDO usage
- `examples/integration-test.php` - Combined feature script
- `examples/wordpress/` - WordPress integration
- `examples/codeigniter/public/index.php` - CodeIgniter-style bootstrap demo
- `examples/drupal/index.php` - Drupal-style kernel stub

## Project Structure

```
phprs/
├── src/
│   ├── engine/          # Core PHP engine
│   │   ├── types.rs     # Type system (PhpValue, PhpType, Val)
│   │   ├── compile/     # Lexer, parser, AST compilation
│   │   ├── vm/          # Virtual machine, opcodes, execution
│   │   ├── jit.rs       # JIT compiler
│   │   └── operators.rs # PHP operators implementation
│   └── php/             # PHP runtime & standard library
│       ├── regex.rs     # preg_* via Rust regex
│       ├── http_stream.rs # HTTP/HTTPS streams
│       ├── pdo.rs       # Database abstraction
│       ├── streams.rs   # Stream wrappers
│       └── filesystem.rs # File operations
├── bin/phprs/           # CLI application
├── examples/            # Curated demos (WordPress, CI, Drupal, language features)
│   ├── wordpress/       # WordPress integration
│   ├── regex-examples.php
│   ├── pdo-examples.php
│   └── integration-test.php
└── tests/               # Comprehensive test suite
```

## API Usage

```rust
use phprs::engine::compile::{compile_string, compile_string_with_functions};
use phprs::engine::vm::{execute_ex, ExecuteData};
use std::sync::Arc;

// Compile a snippet (no user functions)
let op_array = compile_string("<?php echo 'Hello'; ?>", "inline.php")?;

// Scripts with `function` definitions need the function table:
let (op_array, fn_table) =
    compile_string_with_functions("<?php function f() { return 1; } echo f(); ?>", "t.php")?;

let mut exec_data = ExecuteData::new();
exec_data.function_table = Some(Arc::new(fn_table));
let _ = execute_ex(&mut exec_data, &op_array);
```

## Framework Compatibility

### WordPress (partial / `examples/` focus)
There is a **WordPress-shaped demo** under `examples/wordpress/` (stubs, hooks, wpdb-ish pieces, sample plugin/theme). It is useful for development and tests, **not** a claim that arbitrary WordPress sites run unchanged in production on phprs.

- Hooks, plugin/theme loading, and wpdb-related code paths are **incomplete** compared to PHP + Zend + extensions.
- Treat real deployments as **unsupported** until you validate your stack yourself.

```bash
# Demo tree only — not a full WP core checkout
cargo run -p phprs-cli -- run examples/wordpress/index.php
```

### 📋 Laravel (Planned)
- Routing and middleware
- Eloquent ORM
- Blade templating
- Artisan CLI
- Service container

### 📋 Symfony (Planned)
- HTTP kernel
- Dependency injection
- Twig templating
- Console component

### ✅ CodeIgniter 4 (demo)
- Minimal app layout, autoload stubs, router, and sample controller under `examples/codeigniter/`
- Full framework parity is not a goal of the demo; it exercises includes and routing-style code paths

## Features & Capabilities

### ✅ Core PHP Engine
- **Types**: PHP type system (int, float, string, array, object, null, bool) — growing toward full parity
- **Operators**: Core arithmetic, logical, comparison, and string operators (see `examples/operators.php`)
- **Control Flow**: if/else, switch, `match`, **for**, **foreach** (value-only), **while**; legacy **`array()`** and short **`[]`** literals
- **Functions**: User-defined functions, closures, arrow functions, variadic params (`...$args`), named arguments (`func(param: value)`)
- **Classes**: OOP with inheritance, traits, interfaces, namespaces, static properties/methods (`ClassName::$prop`, `ClassName::method()`)
- **Late Static Binding**: `static::` keyword resolved at runtime via `called_class`
- **Magic Methods**: `__get`, `__set`, `__call`, `__callStatic` fallbacks in VM handlers
- **Anonymous Classes**: `new class { ... }` with optional extends/implements
- **Enums**: Pure enums and backed enums (`enum Color: string { case Red = 'red'; }`)
- **Union / Intersection Types**: `int|string`, `Countable&ArrayAccess` parsing in params and return types
- **Reflection API**: `ReflectionClass`, `ReflectionMethod`, `ReflectionProperty`, `ReflectionFunction`, `ReflectionParameter` (getName/getParameters/getNumberOfParameters/isBuiltin/hasMethod/hasProperty)
- **Error Handling**: try/catch/finally, exceptions

### ✅ Standard Library (195+ functions, expanding)
**String Functions**: `strlen`, `substr`, `str_replace`, `trim`, `strtolower`, `strtoupper`, `ucfirst`, `ucwords`, `lcfirst`, `str_repeat`, `str_pad`, `str_split`, `strrev`, `str_contains`, `str_starts_with`, `str_ends_with`, `strtr`, `str_ireplace`, `nl2br`, `chunk_split`, `addslashes`, `stripslashes`, `quotemeta`, `strip_tags`, `htmlspecialchars`/`htmlspecialchars_decode`, `wordwrap`, `number_format`, `sprintf`/`vsprintf`, `substr_count`, `substr_replace`, `strpbrk`, `substr_compare`, plus a growing **mbstring** subset (`mb_strlen`, `mb_substr`, … — see `src/php/mbstring.rs`, `examples/mbstring.php`)

**Callback helpers**: `call_user_func`, `call_user_func_array` (invoke builtins **and** user functions; see `src/engine/vm/callable.rs`)

**Math / type helpers**: `abs`, `ceil`, `floor`, `round`, `sqrt`, `pow`, `max`, `min`, `intdiv`, `fmod`, `hypot`, `is_nan`, `is_infinite`, `is_finite`, `is_numeric`, `is_callable`, `boolval`, base conversion (`decbin`/`decoct`/`dechex`/`bindec`/`octdec`/`hexdec`/`base_convert`), `deg2rad`, `rad2deg`, `similar_text`, `levenshtein`, `soundex`

**Serialization**: `serialize()` / `unserialize()` (scalars, arrays, plain objects)

**URL / query**: `parse_url`, `http_build_query`, and related helpers (`src/php/url.rs`)

**Array Functions**: `array_map`, `array_filter`, `array_reduce`, `array_walk`, `array_merge`, `array_keys`, `array_values`, `array_combine`, `array_flip`, `array_search`, `array_unique`, `array_column`, `array_sum`, `array_product`, `array_chunk`, `array_diff`, `array_intersect`, `array_count_values`, `array_fill`, `array_pad`, `range`, `count`, `in_array`, `array_key_exists`

**Regular Expressions**: `preg_match`, `preg_match_all`, `preg_replace`, `preg_split` (Rust `regex` + `fancy-regex` for look-around; not full PCRE)

**File System**: `file_get_contents`, `file_put_contents`, `file_exists`, `dirname`, `basename`

**HTTP Streams**: `file_get_contents('http://...')` - Full HTTP/HTTPS support

**Database (PDO)**: `new PDO()`, `query()`, `prepare()`, `execute()`, `fetch()`, `fetchAll()`

**Sessions**: Not implemented as PHP extension; demo patterns in `session-examples.php` use `$_SESSION` as a variable

**JSON**: `json_encode`, `json_decode`

**Type Checking**: `isset`, `empty`, `is_array`, `is_string`, `is_int`, `is_null`

**Output**: `echo`, `print`, `var_dump`, `print_r`

### ✅ Advanced Features
- **JIT / optimizer hooks**: present to varying degrees; see code and tests for what’s wired today
- **Bytecode / op arrays**: compiled once per script in the current model
- **Memory**: Rust ownership for engine structures; PHP values still use runtime-managed lifecycles
- **Async I/O**: Tokio in the host for HTTP client / server paths where used

### ✅ Development Tools
- **Built-in Web Server**: `phprs serve`
- **Package Manager**: Composer-compatible
- **REPL**: Interactive PHP shell (planned)
- **Debugger**: Step-through debugging (planned)
- **Profiler**: Performance analysis (planned)

## Performance — expectations

**phprs is not marketed with a “Nx faster than PHP” number.** A fair comparison needs the same features enabled, representative workloads, and pinned versions. We’d rather under-promise and add measured results later than publish invented comparison tables.

**Why Rust is still a sensible implementation language:**
- Mature **LLVM** backend for the host binary.
- **Control over allocation** and hot paths in the VM without a GC for the Rust parts.
- Room to grow **JIT** and other optimizations incrementally.

**Inside the project today (high level):**
- Opcode execution via a **direct dispatch table** (see VM sources).
- **JIT-related** code paths and optimizer scaffolding — completeness varies; read `src/engine/jit.rs` and tests.
- [PERFORMANCE.md](PERFORMANCE.md) discusses **intent and architecture**; when we have reproducible benchmarks, they should live next to methodology (machine, OS, PHP build flags, phprs commit).

If you need predictable numbers for a decision, **measure your own scripts** on both runtimes or open an issue asking for a benchmark harness — we welcome contributions there.

## Getting Started

### Prerequisites
- Rust 1.75+ (2024 edition)
- Cargo (comes with Rust)

### Build from Source
```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/yingkitw/phprs.git
cd phprs
cargo build --release

# Install globally (optional)
cargo install --path bin/phprs
```

### Run Your First Script
```bash
# Create a PHP file
echo '<?php echo "Hello from phprs!\n";' > hello.php

# Run it
phprs run hello.php
```

### Try the Examples
```bash
# Run example scripts
phprs run examples/01_hello_world.php
phprs run examples/regex-examples.php
phprs run examples/pdo-examples.php

# WordPress example
phprs run examples/wordpress/index.php

# Integration test (all features)
phprs run examples/integration-test.php

# Run all tests
cd examples
chmod +x run-all-tests.sh
./run-all-tests.sh
```

## Testing

```bash
# Library + CLI + integration tests (recommended)
cargo test --workspace

# Library only
cargo test --lib

# PHP example compile checks
cargo test --test php_examples

# End-to-end: all root examples/*.php + curated framework demos
cargo test --test examples_runtime

# Rust examples compile
cargo test --test build_rust_examples

# Builtin capability coverage (library)
cargo test --lib builtin_capability

# Smoke a script manually
cargo run -p phprs-cli -- run examples/control_flow.php
cargo run -p phprs-cli -- run examples/test-streams-regex-pdo.php
cargo run -p phprs-cli -- run examples/wordpress/test-theme-plugin.php
```

The workspace runs **clean** (no warnings) on `cargo test --workspace` and `cargo build --workspace`.

## Documentation

### Core Documentation
- **[SPEC.md](SPEC.md)** - Project specification and scope
- **[ARCHITECTURE.md](ARCHITECTURE.md)** - Module structure and execution flow
- **[TODO.md](TODO.md)** - Migration roadmap and statistics (195+ built-in functions, 16 PHP runtime modules)
- **[PERFORMANCE.md](PERFORMANCE.md)** - Evidence policy and VM optimization notes (not benchmark marketing)

### Feature Documentation
- **[examples/STREAMS-REGEX-PDO-README.md](examples/STREAMS-REGEX-PDO-README.md)** - Stream wrappers, regex, sessions, PDO
- **[examples/wordpress/THEME-PLUGIN-README.md](examples/wordpress/THEME-PLUGIN-README.md)** - WordPress integration guide
- **[examples/TEST-GUIDE.md](examples/TEST-GUIDE.md)** - Comprehensive testing guide
- **[examples/TESTING-SUMMARY.md](examples/TESTING-SUMMARY.md)** - Example and test coverage notes

### Quick Links
- **[examples/](examples/)** - Curated PHP demos (language features, WordPress, CodeIgniter, Drupal stubs)
- **[AGENTS.md](AGENTS.md)** - Development guidelines
- **[Cargo.toml](Cargo.toml)** - Dependencies and build configuration

## Roadmap

### ✅ Completed (v0.1.x)
- Core PHP engine with 73 opcodes (added FetchStaticProp, DoStaticCall, CloneObj, SendValNamed, BindGlobal, SendVarRef)
- 195+ built-in functions (string, array, math, regex, hash, datetime, URL, mbstring, callbacks, serialize)
- Regular expressions (`preg_*` via Rust `regex`)
- HTTP/HTTPS stream wrappers
- PDO database abstraction
- Session handling (PHP extension)
- WordPress demo stubs (hooks, plugins, themes — not full core)
- Package manager (Composer-compatible)
- JIT / optimizer scaffolding (see sources; not a complete production JIT story)
- Compiled op arrays (per-run bytecode; caching story is incremental)
- Static properties/methods, late static binding, magic methods, anonymous classes
- Variadic functions, named arguments, union/intersection types, enums (PHP 8.1)

### 🚧 In Progress (v0.2.x)
- Laravel framework support
- Symfony framework support
- Advanced JIT optimizations
- Debugger and profiler
- REPL (interactive shell)

### 📋 Planned (v0.3.x+)
- Deeper CodeIgniter / Drupal parity (beyond `examples/` demos)
- Native extensions API
- WebAssembly compilation
- Distributed caching (Redis, Memcached)
- Real database drivers (MySQL, PostgreSQL)
- HTTP/2 and HTTP/3 support

See [TODO.md](TODO.md) for detailed roadmap.

## Contributing

We welcome contributions! Here's how to get started:

### Development Setup
```bash
# Fork and clone
git clone https://github.com/YOUR_USERNAME/phprs.git
cd phprs

# Create a branch
git checkout -b feature/my-feature

# Make changes and test
cargo test
cargo run -p phprs-cli -- run examples/test-streams-regex-pdo.php

# Submit PR
git push origin feature/my-feature
```

### Areas for Contribution
- 🐛 **Bug Fixes**: Report or fix issues
- ✨ **Features**: Implement new PHP functions or features
- 📚 **Documentation**: Improve docs and examples
- 🧪 **Testing**: Add test cases
- 🎨 **Framework Support**: Add support for more frameworks
- ⚡ **Performance**: Optimize hot paths

### Guidelines
- Follow Rust 2024 edition best practices
- Add tests for new features
- Update documentation
- Keep code DRY and maintainable
- See [AGENTS.md](AGENTS.md) for detailed guidelines

## Community & Support

- **Issues**: [GitHub Issues](https://github.com/yingkitw/phprs/issues)
- **Discussions**: [GitHub Discussions](https://github.com/yingkitw/phprs/discussions)
- **Documentation**: [docs/](docs/)

## License

Apache License 2.0 - See [LICENSE](LICENSE) for details.

## Acknowledgments

- PHP Team - For the original PHP implementation
- Rust Community - For the amazing language and ecosystem
- Contributors - Everyone who has contributed to phprs

---

**Built with ❤️ using Rust** | **Modernizing PHP for the future**
