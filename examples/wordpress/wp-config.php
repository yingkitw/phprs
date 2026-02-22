<?php
// Minimal wp-config: define constants and database settings
define('ABSPATH', dirname(__FILE__) . '/');
define('WP_DEBUG', false);
define('WP_DEBUG_DISPLAY', false);
define('WP_VERSION', '6.4-phprs');

// Database configuration (stub - no real connection)
define('DB_NAME', 'wordpress_phprs');
define('DB_USER', 'root');
define('DB_PASSWORD', '');
define('DB_HOST', 'localhost');
define('DB_CHARSET', 'utf8mb4');
define('DB_COLLATE', '');

// Database table prefix
$table_prefix = 'wp_';
