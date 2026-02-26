# CodeIgniter 4-style example (phprs)

Minimal CodeIgniter 4 bootstrap that runs on the phprs engine.

## Run

From the **project root**:

```bash
cargo run -p phprs-cli -- run examples/codeigniter/public/index.php
```

## Layout

- `public/index.php` — entry point; defines FCPATH, requires Paths and bootstrap
- `app/Config/Paths.php` — defines SYSTEM_PATH, APP_PATH, WRITEPATH
- `system/bootstrap.php` — loads Constants, Autoload; echoes bootstrap status
- `system/Config/Constants.php` — ENVIRONMENT, CI_VERSION
- `system/Config/Autoload.php` — autoloader stub
