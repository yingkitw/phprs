<?php
// Minimal CodeIgniter 4 system bootstrap (phprs)
require 'Config/Constants.php';
require 'Config/Autoload.php';
echo "CodeIgniter 4 bootstrap loaded\n";
echo "FCPATH = " . FCPATH . "\n";
echo "ENVIRONMENT = " . ENVIRONMENT . "\n";
require 'Router.php';
$router = new Router();
$router->dispatch(CI_REQUEST_URI);
