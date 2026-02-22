<?php
// WordPress Plugin API - Hooks and Filters

// Global storage for hooks and filters
global $wp_filter, $wp_actions, $wp_current_filter;
$wp_filter = array();
$wp_actions = array();
$wp_current_filter = array();

// Add an action hook
function add_action($hook_name, $callback, $priority = 10, $accepted_args = 1) {
    return add_filter($hook_name, $callback, $priority, $accepted_args);
}

// Add a filter hook
function add_filter($hook_name, $callback, $priority = 10, $accepted_args = 1) {
    global $wp_filter;
    
    if (!isset($wp_filter[$hook_name])) {
        $wp_filter[$hook_name] = array();
    }
    
    if (!isset($wp_filter[$hook_name][$priority])) {
        $wp_filter[$hook_name][$priority] = array();
    }
    
    $wp_filter[$hook_name][$priority][] = array(
        'function' => $callback,
        'accepted_args' => $accepted_args
    );
    
    return true;
}

// Execute action hooks
function do_action($hook_name) {
    global $wp_filter, $wp_actions, $wp_current_filter;
    
    if (!isset($wp_actions[$hook_name])) {
        $wp_actions[$hook_name] = 1;
    } else {
        $wp_actions[$hook_name]++;
    }
    
    $wp_current_filter[] = $hook_name;
    
    $args = array();
    if (func_num_args() > 1) {
        $args = func_get_args();
        array_shift($args);
    }
    
    if (isset($wp_filter[$hook_name])) {
        ksort($wp_filter[$hook_name]);
        foreach ($wp_filter[$hook_name] as $priority => $callbacks) {
            foreach ($callbacks as $callback_data) {
                call_user_func_array($callback_data['function'], $args);
            }
        }
    }
    
    array_pop($wp_current_filter);
}

// Apply filter hooks
function apply_filters($hook_name, $value) {
    global $wp_filter, $wp_current_filter;
    
    $wp_current_filter[] = $hook_name;
    
    $args = array();
    if (func_num_args() > 2) {
        $args = func_get_args();
        array_shift($args);
    } else {
        $args = array($value);
    }
    
    if (isset($wp_filter[$hook_name])) {
        ksort($wp_filter[$hook_name]);
        foreach ($wp_filter[$hook_name] as $priority => $callbacks) {
            foreach ($callbacks as $callback_data) {
                $value = call_user_func_array($callback_data['function'], $args);
                $args[0] = $value;
            }
        }
    }
    
    array_pop($wp_current_filter);
    return $value;
}

// Remove an action hook
function remove_action($hook_name, $callback, $priority = 10) {
    return remove_filter($hook_name, $callback, $priority);
}

// Remove a filter hook
function remove_filter($hook_name, $callback, $priority = 10) {
    global $wp_filter;
    
    if (!isset($wp_filter[$hook_name][$priority])) {
        return false;
    }
    
    foreach ($wp_filter[$hook_name][$priority] as $key => $callback_data) {
        if ($callback_data['function'] === $callback) {
            unset($wp_filter[$hook_name][$priority][$key]);
            return true;
        }
    }
    
    return false;
}

// Check if action has been done
function did_action($hook_name) {
    global $wp_actions;
    return isset($wp_actions[$hook_name]) ? $wp_actions[$hook_name] : 0;
}

// Check if filter/action exists
function has_filter($hook_name, $callback = false) {
    global $wp_filter;
    
    if (!isset($wp_filter[$hook_name])) {
        return false;
    }
    
    if ($callback === false) {
        return true;
    }
    
    foreach ($wp_filter[$hook_name] as $priority => $callbacks) {
        foreach ($callbacks as $callback_data) {
            if ($callback_data['function'] === $callback) {
                return $priority;
            }
        }
    }
    
    return false;
}

// Alias for has_filter
function has_action($hook_name, $callback = false) {
    return has_filter($hook_name, $callback);
}

// Get number of function arguments (stub)
function func_num_args() {
    return 0;
}

// Get function arguments (stub)
function func_get_args() {
    return array();
}

// Call user function with array of arguments (stub)
function call_user_func_array($callback, $args) {
    // In a real implementation, this would call the callback with args
    // For now, just return null
    return null;
}

// Plugin activation/deactivation hooks
function register_activation_hook($file, $callback) {
    add_action('activate_' . plugin_basename($file), $callback);
}

function register_deactivation_hook($file, $callback) {
    add_action('deactivate_' . plugin_basename($file), $callback);
}

// Get plugin basename
function plugin_basename($file) {
    $file = str_replace('\\', '/', $file);
    $plugin_dir = str_replace('\\', '/', dirname(dirname(__FILE__))) . '/wp-content/plugins/';
    return str_replace($plugin_dir, '', $file);
}

// Load active plugins
function wp_load_plugins() {
    global $wp_plugin_paths;
    
    $plugin_dir = dirname(dirname(__FILE__)) . '/wp-content/plugins/';
    
    if (!file_exists($plugin_dir)) {
        return;
    }
    
    // For this example, we'll manually load known plugins
    // In real WordPress, this reads from the database
    $active_plugins = array(
        'example-plugin/example-plugin.php'
    );
    
    foreach ($active_plugins as $plugin) {
        $plugin_file = $plugin_dir . $plugin;
        if (file_exists($plugin_file)) {
            include_once $plugin_file;
        }
    }
    
    do_action('plugins_loaded');
}
