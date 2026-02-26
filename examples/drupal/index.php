<?php
// Minimal Drupal-style bootstrap (phprs)
// Run: cargo run -p phprs-cli -- run examples/drupal/index.php
require_once __DIR__ . '/core/includes/bootstrap.inc.php';
require_once __DIR__ . '/core/lib/Drupal.php';

echo "Drupal bootstrap complete\n";
