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
    private $options;

    public function __construct($dbuser, $dbpassword, $dbname, $dbhost) {
        $this->dbuser = $dbuser;
        $this->dbname = $dbname;
        $this->dbhost = $dbhost;
        $this->prefix = isset($GLOBALS['table_prefix']) ? $GLOBALS['table_prefix'] : 'wp_';
        $this->charset = defined('DB_CHARSET') ? DB_CHARSET : 'utf8mb4';
        $this->collate = defined('DB_COLLATE') ? DB_COLLATE : '';

        $this->options = array(
            'siteurl' => 'http://localhost',
            'home' => 'http://localhost',
            'blogname' => 'WordPress on phprs',
            'blogdescription' => 'Just another WordPress site'
        );
    }

    public function query($query) {
        return true;
    }

    public function get_results($query) {
        if (strpos($query, 'wp_options') !== false && strpos($query, 'SELECT') !== false) {
            return array();
        }
        return array();
    }

    public function get_row($query) {
        return null;
    }

    public function get_var($query) {
        if (strpos($query, 'wp_options') !== false && strpos($query, 'option_value') !== false) {
            if (preg_match("/option_name = '([^']+)'/", $query, $matches)) {
                $option_name = $matches[1];
                return $this->get_option($option_name, null);
            }
        }
        return null;
    }

    public function insert($table, $data, $format = null) {
        if ($table === $this->prefix . 'options' && isset($data['option_name'])) {
            $name = $data['option_name'];
            $value = $data['option_value'];
            $this->set_option($name, $value);
            return true;
        }
        return true;
    }

    public function update($table, $data, $where, $format = null, $where_format = null) {
        if ($table === $this->prefix . 'options' && isset($where['option_name'])) {
            $name = $where['option_name'];
            $value = $data['option_value'];
            $this->set_option($name, $value);
            return 1;
        }
        return 1;
    }

    public function delete($table, $where, $where_format = null) {
        if ($table === $this->prefix . 'options' && isset($where['option_name'])) {
            $name = $where['option_name'];
            $this->set_option($name, null);
            return 1;
        }
        return 1;
    }

    public function prepare($query) {
        return $query;
    }

    public function get_option($option_name, $default = false) {
        $opts = $this->options;
        if (isset($opts[$option_name])) {
            $val = $opts[$option_name];
            if ($val === null) {
                return $default;
            }
            return $val;
        }
        return $default;
    }

    public function set_option($option_name, $option_value) {
        $opts = $this->options;
        $opts[$option_name] = $option_value;
        $this->options = $opts;
    }
}
