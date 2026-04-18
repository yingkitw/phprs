<?php
// Minimal Drupal-style bootstrap (phprs)
// Run: cargo run -p phprs-cli -- run examples/drupal/index.php
require_once 'core/includes/bootstrap.inc.php';
require_once 'core/lib/Drupal.php';
require_once 'core/lib/DrupalKernel.php';
require_once 'core/lib/Drupal/Core/Extension/ModuleHandler.php';

echo "Drupal bootstrap complete\n";
echo "DRUPAL_ROOT = " . DRUPAL_ROOT . "\n";
$kernel = new DrupalKernel();
$kernel->boot();
