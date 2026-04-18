<?php
// Minimal DrupalKernel stub (phprs) — mirrors Drupal 8+ bootstrap flow
class DrupalKernel {
    function boot() {
        echo "DrupalKernel::boot()\n";
        $handler = new ModuleHandler();
        $handler->loadAll();
    }
}
