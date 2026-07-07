<?php
require_once dirname(__FILE__) . '/../../examples/wordpress/wp-includes/plugin.php';

function hello_init() {
    echo "FIRED\n";
}

add_action('init', 'hello_init');
echo "filter isset=" . (isset($wp_filter['init']) ? 'yes' : 'no') . "\n";
if (isset($wp_filter['init'])) {
    echo "hooks=" . count($wp_filter['init'][10]) . "\n";
}
do_action('init');
echo "done\n";
