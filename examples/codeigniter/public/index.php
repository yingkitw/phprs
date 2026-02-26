<?php
// Minimal CodeIgniter 4-style bootstrap (phprs)
// Run: cargo run -p phprs-cli -- run examples/codeigniter/public/index.php
define('FCPATH', dirname(__FILE__) . '/');
require dirname(__FILE__) . '/../app/Config/Paths.php';
require dirname(__FILE__) . '/../system/bootstrap.php';
