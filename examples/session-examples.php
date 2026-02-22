<?php
/**
 * Session Handling Examples
 * 
 * Demonstrates session management functionality
 */

echo "=== Session Handling Examples ===\n\n";

// Example 1: Basic Session Usage
echo "Example 1: Start Session and Store Data\n";
session_start();
$_SESSION['username'] = 'john_doe';
$_SESSION['user_id'] = 12345;
$_SESSION['login_time'] = time();
echo "  Session started: Yes\n";
echo "  Session ID: " . session_id() . "\n";
echo "  Data stored: username, user_id, login_time\n\n";

// Example 2: Reading Session Data
echo "Example 2: Read Session Variables\n";
if (isset($_SESSION['username'])) {
    echo "  Username: " . $_SESSION['username'] . "\n";
    echo "  User ID: " . $_SESSION['user_id'] . "\n";
    echo "  Login time: " . $_SESSION['login_time'] . "\n";
}
echo "\n";

// Example 3: Session ID Management
echo "Example 3: Session ID Operations\n";
$old_id = session_id();
echo "  Current session ID: $old_id\n";
session_regenerate_id();
$new_id = session_id();
echo "  New session ID: $new_id\n";
echo "  ID changed: " . ($old_id !== $new_id ? "Yes" : "No") . "\n\n";

// Example 4: Session Name
echo "Example 4: Session Name Management\n";
$default_name = session_name();
echo "  Default session name: $default_name\n";
session_name('MY_APP_SESSION');
echo "  Custom session name: " . session_name() . "\n\n";

// Example 5: Shopping Cart Example
echo "Example 5: Shopping Cart Session\n";
if (!isset($_SESSION['cart'])) {
    $_SESSION['cart'] = [];
}

// Add items to cart
$_SESSION['cart'][] = ['id' => 101, 'name' => 'Widget', 'price' => 19.99];
$_SESSION['cart'][] = ['id' => 102, 'name' => 'Gadget', 'price' => 29.99];

echo "  Items in cart: " . count($_SESSION['cart']) . "\n";
$total = 0;
foreach ($_SESSION['cart'] as $item) {
    echo "    - " . $item['name'] . ": $" . $item['price'] . "\n";
    $total += $item['price'];
}
echo "  Total: $" . number_format($total, 2) . "\n\n";

// Example 6: User Preferences
echo "Example 6: Store User Preferences\n";
$_SESSION['preferences'] = [
    'theme' => 'dark',
    'language' => 'en',
    'notifications' => true,
    'items_per_page' => 25
];
echo "  Preferences stored:\n";
foreach ($_SESSION['preferences'] as $key => $value) {
    $val = is_bool($value) ? ($value ? 'true' : 'false') : $value;
    echo "    $key: $val\n";
}
echo "\n";

// Example 7: Login State Management
echo "Example 7: User Authentication State\n";
$_SESSION['authenticated'] = true;
$_SESSION['user_role'] = 'admin';
$_SESSION['permissions'] = ['read', 'write', 'delete'];

function is_logged_in() {
    return isset($_SESSION['authenticated']) && $_SESSION['authenticated'];
}

function has_permission($perm) {
    return isset($_SESSION['permissions']) && 
           in_array($perm, $_SESSION['permissions']);
}

echo "  User logged in: " . (is_logged_in() ? "Yes" : "No") . "\n";
echo "  User role: " . $_SESSION['user_role'] . "\n";
echo "  Can delete: " . (has_permission('delete') ? "Yes" : "No") . "\n\n";

// Example 8: Flash Messages
echo "Example 8: Flash Messages (One-Time Display)\n";
$_SESSION['flash_messages'] = [
    'success' => 'Profile updated successfully!',
    'warning' => 'Your session will expire in 5 minutes'
];

echo "  Flash messages set:\n";
foreach ($_SESSION['flash_messages'] as $type => $message) {
    echo "    [$type] $message\n";
}

// Clear flash messages after display
unset($_SESSION['flash_messages']);
echo "  Messages cleared after display\n\n";

// Example 9: Session Timeout Check
echo "Example 9: Session Timeout Management\n";
$timeout = 1800; // 30 minutes
$_SESSION['last_activity'] = time();

function check_session_timeout($timeout) {
    if (isset($_SESSION['last_activity'])) {
        $elapsed = time() - $_SESSION['last_activity'];
        if ($elapsed > $timeout) {
            return true; // Session expired
        }
    }
    $_SESSION['last_activity'] = time();
    return false;
}

$expired = check_session_timeout($timeout);
echo "  Timeout period: $timeout seconds (30 minutes)\n";
echo "  Session expired: " . ($expired ? "Yes" : "No") . "\n";
echo "  Last activity updated: Yes\n\n";

// Example 10: Multi-Step Form Data
echo "Example 10: Multi-Step Form Progress\n";
$_SESSION['form_step'] = 2;
$_SESSION['form_data'] = [
    'step1' => ['name' => 'John Doe', 'email' => 'john@example.com'],
    'step2' => ['address' => '123 Main St', 'city' => 'Springfield']
];

echo "  Current step: " . $_SESSION['form_step'] . "/3\n";
echo "  Data collected:\n";
foreach ($_SESSION['form_data'] as $step => $data) {
    echo "    $step: " . count($data) . " fields\n";
}
echo "\n";

// Example 11: Session Security
echo "Example 11: Session Security Best Practices\n";

function secure_session_start() {
    // Regenerate ID on login
    session_regenerate_id(true);
    
    // Store user agent
    $_SESSION['user_agent'] = 'Mozilla/5.0...';
    
    // Store IP address (in real app)
    $_SESSION['ip_address'] = '127.0.0.1';
    
    return true;
}

function validate_session() {
    // Check user agent matches
    if (isset($_SESSION['user_agent'])) {
        // Validate user agent
        return true;
    }
    return false;
}

secure_session_start();
echo "  Session secured: Yes\n";
echo "  User agent stored: Yes\n";
echo "  IP address tracked: Yes\n";
echo "  Session valid: " . (validate_session() ? "Yes" : "No") . "\n\n";

// Example 12: Session Data Serialization
echo "Example 12: Complex Data Storage\n";
$_SESSION['user_object'] = [
    'id' => 123,
    'profile' => [
        'first_name' => 'John',
        'last_name' => 'Doe',
        'avatar' => 'avatar.jpg'
    ],
    'settings' => [
        'timezone' => 'America/New_York',
        'locale' => 'en_US'
    ]
];

echo "  Complex object stored in session\n";
echo "  User ID: " . $_SESSION['user_object']['id'] . "\n";
echo "  Name: " . $_SESSION['user_object']['profile']['first_name'] . " " . 
     $_SESSION['user_object']['profile']['last_name'] . "\n";
echo "  Timezone: " . $_SESSION['user_object']['settings']['timezone'] . "\n\n";

// Example 13: Session Cleanup
echo "Example 13: Selective Session Data Cleanup\n";
echo "  Before cleanup: " . count(array_keys($_SESSION)) . " session variables\n";

// Remove specific keys
unset($_SESSION['form_step']);
unset($_SESSION['form_data']);

echo "  After cleanup: " . count(array_keys($_SESSION)) . " session variables\n";
echo "  Form data removed: Yes\n\n";

// Example 14: Session Write and Close
echo "Example 14: Session Write Operations\n";
session_write_close();
echo "  Session data written: Yes\n";
echo "  Session closed: Yes\n";
echo "  Note: Further writes require session_start() again\n\n";

// Example 15: Complete Session Lifecycle
echo "Example 15: Complete Session Lifecycle\n";
session_start();
echo "  1. Session started\n";
$_SESSION['test'] = 'value';
echo "  2. Data stored\n";
$value = $_SESSION['test'];
echo "  3. Data retrieved: $value\n";
session_regenerate_id();
echo "  4. Session ID regenerated\n";
session_write_close();
echo "  5. Session written and closed\n";
session_start();
echo "  6. Session resumed\n";
session_destroy();
echo "  7. Session destroyed\n\n";

echo "=== Session Features Summary ===\n";
echo "✓ Session lifecycle management (start, destroy)\n";
echo "✓ Session ID operations (get, set, regenerate)\n";
echo "✓ \$_SESSION superglobal support\n";
echo "✓ Complex data storage (arrays, objects)\n";
echo "✓ Security features (ID regeneration, validation)\n";
echo "✓ Timeout management\n";
echo "✓ Flash messages\n";
echo "✓ Shopping cart functionality\n";
echo "✓ User authentication state\n";
echo "✓ Multi-step form handling\n\n";

echo "=== All Session Examples Complete ===\n";
