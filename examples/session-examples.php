<?php
/**
 * Session-style state examples (phprs)
 *
 * The PHP session extension (session_start, session_id, etc.) is not implemented in phprs.
 * This script mirrors common patterns using a plain $_SESSION array so the demo runs here.
 */

echo '=== Session Handling Examples (simulated $_SESSION) ===' . "\n\n";
echo 'Note: session_* functions are not available in phprs; using $_SESSION as an array.' . "\n\n";

$_SESSION = [];

// Example 1: Basic usage
echo "Example 1: Store session-like data\n";
$_SESSION['username'] = 'john_doe';
$_SESSION['user_id'] = 12345;
$_SESSION['login_time'] = time();
echo "  Data stored: username, user_id, login_time\n\n";

// Example 2: Reading data
echo "Example 2: Read variables\n";
if (isset($_SESSION['username'])) {
    echo "  Username: " . $_SESSION['username'] . "\n";
    echo "  User ID: " . $_SESSION['user_id'] . "\n";
    echo "  Login time: " . $_SESSION['login_time'] . "\n";
}
echo "\n";

// Example 3: Shopping cart (no [] append on nested dim — use a temp array)
echo "Example 3: Shopping cart\n";
$cart = [];
$item0 = [];
$item0['id'] = 101;
$item0['name'] = 'Widget';
$item0['price'] = 19.99;
$cart[0] = $item0;
$item1 = [];
$item1['id'] = 102;
$item1['name'] = 'Gadget';
$item1['price'] = 29.99;
$cart[1] = $item1;
$_SESSION['cart'] = $cart;

echo "  Items in cart: " . count($_SESSION['cart']) . "\n";
$total = 0;
foreach ($_SESSION['cart'] as $item) {
    echo "    - " . $item['name'] . ": $" . $item['price'] . "\n";
    $total = $total + $item['price'];
}
echo "  Total: $" . $total . "\n\n";

// Example 4: Preferences (nested array built without chained [] assignment)
echo "Example 4: User preferences\n";
$prefs = [];
$prefs['theme'] = 'dark';
$prefs['language'] = 'en';
$prefs['notifications'] = true;
$prefs['items_per_page'] = 25;
$_SESSION['preferences'] = $prefs;
echo "  theme: " . $_SESSION['preferences']['theme'] . "\n";
echo "  language: " . $_SESSION['preferences']['language'] . "\n";
$n = $_SESSION['preferences']['notifications'];
echo "  notifications: " . ($n ? 'true' : 'false') . "\n";
echo "  items_per_page: " . $_SESSION['preferences']['items_per_page'] . "\n\n";

// Example 5: Auth state (inlined — user-defined functions are not executed in phprs yet)
echo "Example 5: Authentication state\n";
$_SESSION['authenticated'] = true;
$_SESSION['user_role'] = 'admin';
$_SESSION['permissions'] = ['read', 'write', 'delete'];

$logged_in = isset($_SESSION['authenticated']) && $_SESSION['authenticated'];
echo "  User logged in: " . ($logged_in ? "Yes" : "No") . "\n";
echo "  User role: " . $_SESSION['user_role'] . "\n";
$can_delete = isset($_SESSION['permissions']) && in_array('delete', $_SESSION['permissions']);
echo "  Can delete: " . ($can_delete ? "Yes" : "No") . "\n\n";

// Example 6: Timeout check (inlined)
echo "Example 6: Session timeout pattern\n";
$timeout = 1800;
$_SESSION['last_activity'] = time();
$expired = false;
if (isset($_SESSION['last_activity'])) {
    $elapsed = time() - $_SESSION['last_activity'];
    if ($elapsed > $timeout) {
        $expired = true;
    }
}
$_SESSION['last_activity'] = time();
echo "  Timeout period: $timeout seconds\n";
echo "  Session expired: " . ($expired ? "Yes" : "No") . "\n\n";

// Example 7: Cleanup
echo "Example 7: Cleanup\n";
$before = count($_SESSION);
echo "  Keys before clearing cart: $before\n";
$_SESSION['cart'] = [];
echo "  Cart cleared\n\n";

echo "=== Session Examples Complete ===\n";
