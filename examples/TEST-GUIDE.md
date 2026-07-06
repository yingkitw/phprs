# Test Guide for Stream, Regex, PDO, and Example Demos

How to run phprs example scripts and what CI enforces.

## Quick start (CI-equivalent)

```bash
# All root examples/*.php — auto-discovered matrix
cargo test --test examples_runtime examples_root_php_scripts_all_run

# Full workspace
cargo test --workspace
```

Manual runs:

```bash
cargo run -p phprs-cli -- run examples/regex-examples.php
cargo run -p phprs-cli -- run examples/http-stream-examples.php
cargo run -p phprs-cli -- run examples/session-examples.php
cargo run -p phprs-cli -- run examples/pdo-examples.php
cargo run -p phprs-cli -- run examples/integration-test.php
cargo run -p phprs-cli -- run examples/test-streams-regex-pdo.php
```

Optional shell runner (if present):

```bash
cd examples && chmod +x run-all-tests.sh && ./run-all-tests.sh
```

## Test files

### regex-examples.php
- 15 practical `preg_*` scenarios
- Uses **Rust `regex`** — no look-ahead/look-behind; password demo uses multiple checks instead of `(?=…)`
- No `foreach ($arr as $key => $value)` — use value-only foreach or separate variables

### http-stream-examples.php
- Documents HTTP/HTTPS `file_get_contents` patterns
- Live network calls depend on connectivity; demos are mostly illustrative

### session-examples.php
- **Does not call `session_start()`** — PHP session extension is not implemented in the engine
- Demonstrates patterns with `$_SESSION` as a **plain array** (assign `$_SESSION = []` first)
- Avoid `$arr[] = …`, `$x += $y['z']`, and chained `$a['b']['c'] = …` (compiler limits)

### pdo-examples.php
- Exercises **PDO stub** (`src/php/pdo.rs`) — no real MySQL/PostgreSQL connections

### integration-test.php
- Short runnable smoke script (PDO, regex, password, json)

### test-streams-regex-pdo.php
- Checklist output for streams, regex, PDO-shaped APIs

## Rust / workspace tests

| Suite | Purpose |
|-------|---------|
| `tests/examples_runtime.rs` | Root PHP examples + CI/Drupal entrypoints |
| `tests/build_rust_examples.rs` | `examples/rust/*.rs` compile |
| `src/engine/vm/builtin_capability_tests.rs` | Builtin surface area |
| `tests/php_examples.rs` | Compile-only checks |

## Debugging

```bash
RUST_LOG=debug cargo run -p phprs-cli -- run examples/your-script.php
```

## Contributing

1. Add or update `examples/<name>.php` at the **root** of `examples/` when possible.
2. Ensure `cargo test --test examples_runtime examples_root_php_scripts_all_run` passes.
3. Update this guide and [TESTING-SUMMARY.md](TESTING-SUMMARY.md) if behavior or limitations change.

See also [STREAMS-REGEX-PDO-README.md](STREAMS-REGEX-PDO-README.md) and [../tests/README.md](../tests/README.md).
