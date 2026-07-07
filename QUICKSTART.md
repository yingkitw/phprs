# phprs Quick Start Guide

Get up and running with phprs in a few minutes.

## Installation

### Build from source (recommended)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh   # if needed
git clone https://github.com/yingkitw/phprs.git
cd phprs
cargo build --release
# Binary: target/release/phprs
```

### Install with Cargo (optional)

```bash
cargo install --path bin/phprs
```

## Your first script

```bash
echo '<?php echo "Hello from phprs!\n";' > hello.php
cargo run -p phprs-cli -- run hello.php
```

## Common commands

```bash
phprs run script.php
phprs serve                  # dev server (default port 3080)
phprs serve --port 8080
phprs pkg init               # Composer-style project stub
```

## Try bundled examples

```bash
phprs run examples/01_hello_world.php
phprs run examples/regex-examples.php
phprs run examples/pdo-examples.php
phprs run examples/session-examples.php   # simulated $_SESSION, not session_start()
phprs run examples/integration-test.php

# CI-equivalent: all root examples/*.php
cargo test --test examples_runtime examples_root_php_scripts_all_run
```

Framework demos:

```bash
phprs run examples/codeigniter/public/index.php   # tested in CI
phprs run examples/drupal/index.php               # tested in CI
phprs run examples/wordpress/index.php            # may fail on wp-db array() syntax
```

## Expectations

- **Not a drop-in Zend replacement** — try your script, read errors, check [TODO.md](TODO.md).
- **Sessions** — `session_start()` is not an engine builtin; see `examples/session-examples.php`.
- **PDO** — stub/in-memory API for demos, not a real DB driver.
- **Regex** — Rust `regex` + `fancy-regex` for look-around; not full PCRE.
- **Performance** — use `cargo build --release`; see [PERFORMANCE.md](PERFORMANCE.md) (no invented PHP speedup tables).

## Development workflow

```bash
cargo test --workspace
cargo run -p phprs-cli -- run examples/test-streams-regex-pdo.php
RUST_LOG=debug cargo run -p phprs-cli -- run your-script.php
```

## Next steps

- [README.md](README.md) — features and structure
- [examples/README.md](examples/README.md) — all demos
- [ARCHITECTURE.md](ARCHITECTURE.md) — how the engine fits together
- [PERFORMANCE.md](PERFORMANCE.md) — measurement policy
