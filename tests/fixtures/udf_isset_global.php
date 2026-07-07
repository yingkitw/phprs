<?php
$wp_filter = array();
function test_isset($hook_name) {
    global $wp_filter;
    if (!isset($wp_filter[$hook_name])) {
        $wp_filter[$hook_name] = array();
    }
    echo "ok\n";
}
echo "call\n";
test_isset('init');
echo "done\n";
