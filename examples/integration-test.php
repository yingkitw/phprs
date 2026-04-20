<?php
/**
 * phprs integration script — exercises interpreter features together.
 * Keeps to constructs supported by the current compiler (see examples/*.php).
 */

echo "=== phprs integration check ===\n\n";

echo "Regex:\n";
$t = "Hello World 123";
echo "  preg_match /World/: " . (preg_match("/World/", $t) === 1 ? "ok" : "fail") . "\n";
echo "  preg_replace: " . preg_replace("/World/", "PHP", $t) . "\n";
echo "  preg_match_all cats: " . preg_match_all("/cat/", "cat dog cat") . "\n\n";

echo "Types:\n";
echo "  is_numeric(42): " . (is_numeric(42) ? "yes" : "no") . "\n";
echo "  is_float(3.14): " . (is_float(3.14) ? "yes" : "no") . "\n\n";

echo "Password:\n";
$h = password_hash("secret", "PASSWORD_DEFAULT");
echo "  password_verify: " . (password_verify("secret", $h) ? "ok" : "fail") . "\n\n";

echo "JSON:\n";
echo "  " . json_encode(42) . "\n\n";

echo "PDO (in-memory driver in phprs):\n";
try {
    $pdo = new PDO("mysql:host=localhost;dbname=webapp", "root", "");
    echo "  constructed: ok\n";
    $pdo->exec("CREATE TABLE IF NOT EXISTS users (id INT, name VARCHAR(50))");
    echo "  exec create: ok\n";
    $pdo->exec("INSERT INTO users (name) VALUES ('alice')");
    echo "  lastInsertId: " . $pdo->lastInsertId() . "\n";
} catch (Exception $e) {
    echo "  error: " . $e->getMessage() . "\n";
}
echo "\n=== integration check finished ===\n";
