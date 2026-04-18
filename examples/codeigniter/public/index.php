<?php
// Minimal CodeIgniter 4-style bootstrap (phprs)
// Run: cargo run -p phprs-cli -- run examples/codeigniter/public/index.php
// Use relative requires (dirname/__FILE__ can be unreliable on some entry paths)
define('FCPATH', './');
if (!defined('CI_REQUEST_URI')) {
    define('CI_REQUEST_URI', '/');
}
require '../app/Config/Paths.php';
require '../system/bootstrap.php';
