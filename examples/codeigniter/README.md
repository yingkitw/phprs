# CodeIgniter 4-style example (phprs)

Minimal CodeIgniter 4 bootstrap that runs on the phprs engine.

## Run

From the **project root**:

```bash
cargo run -p phprs-cli -- run examples/codeigniter/public/index.php
```

## CI

Covered by `example_codeigniter_public_index_runs` in `tests/examples_runtime.rs` (compile + VM + stdout assertions).

## Layout

- `public/index.php` — entry point; defines FCPATH, requires Paths and bootstrap
- `app/Config/Paths.php` — defines SYSTEM_PATH, APP_PATH, WRITEPATH
- `system/bootstrap.php` — loads Constants, Autoload; echoes bootstrap status
- `system/Config/Constants.php` — ENVIRONMENT, CI_VERSION
- `system/Config/Autoload.php` — autoloader stub
- `system/Router.php` — minimal router used by the demo

This is a **demo**, not full CodeIgniter 4 parity.
