# Testing Summary

Overview of how examples and tests relate in phprs.

## Workspace tests

Roughly **376+** tests across:

- **296** library unit tests (`cargo test --lib`)
- Integration crates: `examples_runtime` (14), `php_examples` (16), `comprehensive_tests` (25), `edge_cases` (13), `error_handling` (4), `integration_tests` (4), `example_verification` (7), `build_rust_examples` (1)

## Example entrypoints

### Root PHP (`examples/*.php`) — 21 files

**Enforced by** `examples_root_php_scripts_all_run` in `tests/examples_runtime.rs`:

- Auto-discovers every `*.php` directly under `examples/`
- Requires `PhpResult::Success` and non-empty stdout

### Rust (`examples/rust/`) — 9 files

**Enforced by** `build_rust_examples` (`cargo build --examples`).

### Framework trees (nested)

| Entry | Automated test |
|-------|----------------|
| `codeigniter/public/index.php` | `example_codeigniter_public_index_runs` |
| `drupal/index.php` | `example_drupal_index_runs` |
| `wordpress/index.php` | Manual — `array()` in wp-db stub blocks compile today |
| Nested includes under CI/Drupal/WP | Indirectly via entrypoint tests |

## Feature demo scripts

| Script | Focus | Engine reality |
|--------|--------|----------------|
| `regex-examples.php` | `preg_*` | look-around via fancy-regex |
| `http-stream-examples.php` | HTTP streams | `reqwest` + blocking wrapper |
| `session-examples.php` | Session **patterns** | No `session_*` builtins; simulated `$_SESSION` |
| `pdo-examples.php` | PDO API shape | In-memory stub |
| `integration-test.php` | Combined smoke | Runnable under phprs |
| `test-streams-regex-pdo.php` | Checklist | Runnable under phprs |

## Builtin coverage

`src/engine/vm/builtin_capability_tests.rs` exercises a large fraction of `execute_builtin_function` (regex, hash, math, datetime, mbstring, output, WordPress hooks, file ops, etc.). This is the authoritative list of **implemented** builtins — not the marketing bullets in older docs.

## What “100%” does not mean here

- Sessions are **not** the PHP session extension.
- PDO is **not** a real database driver.
- Regex is **not** full PCRE.
- WordPress demo is **not** production WordPress.

## Running everything

```bash
cargo test --workspace
cargo test --test examples_runtime
```

## Docs

- [TEST-GUIDE.md](TEST-GUIDE.md)
- [STREAMS-REGEX-PDO-README.md](STREAMS-REGEX-PDO-README.md)
- [../tests/README.md](../tests/README.md)
- [../PERFORMANCE.md](../PERFORMANCE.md) — no invented PHP speedup tables
