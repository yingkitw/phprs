<?php
// wpdb class stub for phprs - WordPress database abstraction layer
// This is a minimal implementation for testing WordPress bootstrap

class wpdb {
    public $prefix;
    public $dbname;
    public $dbuser;
    public $dbhost;
    public $charset;
    public $collate;
    
    // In-memory storage for options (stub)
    private $options = array();
    
    public function __construct($dbuser, $dbpassword, $dbname, $dbhost) {
        $this->dbuser = $dbuser;
        $this->dbname = $dbname;
        $this->dbhost = $dbhost;
        $this->prefix = isset($GLOBALS['table_prefix']) ? $GLOBALS['table_prefix'] : 'wp_';
        $this->charset = defined('DB_CHARSET') ? DB_CHARSET : 'utf8mb4';
        $this->collate = defined('DB_COLLATE') ? DB_COLLATE : '';
        
        // Initialize with some default options
        $this->options['siteurl'] = 'http://localhost';
        $this->options['home'] = 'http://localhost';
        $this->options['blogname'] = 'WordPress on phprs';
        $this->options['blogdescription'] = 'Just another WordPress site';
    }
    
    // Execute a query (stub - returns true)
    public function query($query) {
        return true;
    }
    
    // Get results from a query (stub - returns empty array)
    public function get_results($query) {
        // Parse simple SELECT queries for options table
        if (strpos($query, 'wp_options') !== false && strpos($query, 'SELECT') !== false) {
            return array();
        }
        return array();
    }
    
    // Get a single row (stub)
    public function get_row($query) {
        return null;
    }
    
    // Get a single variable (stub)
    public function get_var($query) {
        // Handle option queries
        if (strpos($query, 'wp_options') !== false && strpos($query, 'option_value') !== false) {
            // Extract option name from query (very basic parsing)
            if (preg_match("/option_name = '([^']+)'/", $query, $matches)) {
                $option_name = $matches[1];
                if (isset($this->options[$option_name])) {
                    return $this->options[$option_name];
                }
            }
        }
        return null;
    }
    
    // Insert a row (stub)
    public function insert($table, $data, $format = null) {
        if ($table === $this->prefix . 'options' && isset($data['option_name'])) {
            $this->options[$data['option_name']] = $data['option_value'];
            return true;
        }
        return true;
    }
    
    // Update rows (stub)
    public function update($table, $data, $where, $format = null, $where_format = null) {
        if ($table === $this->prefix . 'options' && isset($where['option_name'])) {
            $this->options[$where['option_name']] = $data['option_value'];
            return 1;
        }
        return 1;
    }
    
    // Delete rows (stub)
    public function delete($table, $where, $where_format = null) {
        if ($table === $this->prefix . 'options' && isset($where['option_name'])) {
            unset($this->options[$where['option_name']]);
            return 1;
        }
        return 1;
    }
    
    // Prepare a query (stub - returns query as-is)
    public function prepare($query) {
        return $query;
    }
    
    // Get option from in-memory store
    public function get_option($option_name, $default = false) {
        if (isset($this->options[$option_name])) {
            return $this->options[$option_name];
        }
        return $default;
    }
    
    // Set option in in-memory store
    public function set_option($option_name, $option_value) {
        $this->options[$option_name] = $option_value;
    }
}
