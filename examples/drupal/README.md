# Drupal-style example (phprs)

Minimal Drupal bootstrap that runs on the phprs engine.

## Run

From the **project root**:

```bash
cargo run -p phprs-cli -- run examples/drupal/index.php
```

## Layout

- `index.php` — entry point; requires bootstrap.inc and Drupal.php
- `core/includes/bootstrap.inc.php` — defines DRUPAL_ROOT, DRUPAL_BOOTSTRAP_CONFIGURATION
- `core/lib/Drupal.php` — kernel stub; echoes load status
