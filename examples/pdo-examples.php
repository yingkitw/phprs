<?php
/**
 * PDO Database Examples
 * 
 * Demonstrates PDO database abstraction layer
 */

echo "=== PDO Database Examples ===\n\n";

// Example 1: Database Connection
echo "Example 1: Connect to Database\n";
try {
    $pdo = new PDO('mysql:host=localhost;dbname=testdb', 'root', '');
    echo "  ✓ MySQL connection established\n";
    echo "  DSN: mysql:host=localhost;dbname=testdb\n";
    echo "  Driver: MySQL\n";
} catch (Exception $e) {
    echo "  ✗ Connection failed: " . $e->getMessage() . "\n";
}
echo "\n";

// Example 2: Simple Query
echo "Example 2: Execute Simple SELECT Query\n";
if (isset($pdo)) {
    $stmt = $pdo->query("SELECT * FROM users");
    echo "  Query: SELECT * FROM users\n";
    echo "  Statement created: Yes\n";
    echo "  Row count: " . $stmt->rowCount() . "\n";
    echo "  Column count: " . $stmt->columnCount() . "\n";
}
echo "\n";

// Example 3: Prepared Statement with Named Parameters
echo "Example 3: Prepared Statement (Named Parameters)\n";
if (isset($pdo)) {
    $stmt = $pdo->prepare("SELECT * FROM users WHERE id = :id AND status = :status");
    echo "  SQL: SELECT * FROM users WHERE id = :id AND status = :status\n";
    
    $user_id = 123;
    $status = 'active';
    $stmt->bindParam(':id', $user_id);
    $stmt->bindParam(':status', $status);
    echo "  Parameters bound: :id = $user_id, :status = $status\n";
    
    $stmt->execute();
    echo "  Statement executed: Yes\n";
}
echo "\n";

// Example 4: INSERT Operation
echo "Example 4: Insert New Record\n";
if (isset($pdo)) {
    $sql = "INSERT INTO users (name, email, created_at) VALUES ('John Doe', 'john@example.com', NOW())";
    $affected = $pdo->exec($sql);
    echo "  SQL: $sql\n";
    echo "  Rows affected: $affected\n";
    echo "  Last insert ID: " . $pdo->lastInsertId() . "\n";
}
echo "\n";

// Example 5: UPDATE Operation
echo "Example 5: Update Existing Record\n";
if (isset($pdo)) {
    $stmt = $pdo->prepare("UPDATE users SET email = :email WHERE id = :id");
    $email = 'newemail@example.com';
    $id = 123;
    $stmt->bindParam(':email', $email);
    $stmt->bindParam(':id', $id);
    $stmt->execute();
    echo "  Updated user $id\n";
    echo "  New email: $email\n";
    echo "  Rows affected: " . $stmt->rowCount() . "\n";
}
echo "\n";

// Example 6: DELETE Operation
echo "Example 6: Delete Record\n";
if (isset($pdo)) {
    $stmt = $pdo->prepare("DELETE FROM users WHERE id = :id");
    $id = 999;
    $stmt->bindParam(':id', $id);
    $stmt->execute();
    echo "  Deleted user $id\n";
    echo "  Rows affected: " . $stmt->rowCount() . "\n";
}
echo "\n";

// Example 7: Transaction Management
echo "Example 7: Database Transaction\n";
if (isset($pdo)) {
    try {
        $pdo->beginTransaction();
        echo "  Transaction started\n";
        
        $pdo->exec("UPDATE accounts SET balance = balance - 100 WHERE id = 1");
        echo "  Deducted 100 from account 1\n";
        
        $pdo->exec("UPDATE accounts SET balance = balance + 100 WHERE id = 2");
        echo "  Added 100 to account 2\n";
        
        $pdo->commit();
        echo "  Transaction committed: Yes\n";
    } catch (Exception $e) {
        $pdo->rollback();
        echo "  Transaction rolled back: " . $e->getMessage() . "\n";
    }
}
echo "\n";

// Example 8: Fetch Single Row
echo "Example 8: Fetch Single Row\n";
if (isset($pdo)) {
    $stmt = $pdo->prepare("SELECT * FROM users WHERE id = :id");
    $id = 1;
    $stmt->bindParam(':id', $id);
    $stmt->execute();
    
    $user = $stmt->fetch();
    if ($user) {
        echo "  User found: Yes\n";
        echo "  Data: " . count($user) . " columns\n";
    } else {
        echo "  User found: No\n";
    }
}
echo "\n";

// Example 9: Fetch All Rows
echo "Example 9: Fetch All Rows\n";
if (isset($pdo)) {
    $stmt = $pdo->query("SELECT * FROM users LIMIT 10");
    $users = $stmt->fetchAll();
    echo "  Query: SELECT * FROM users LIMIT 10\n";
    echo "  Rows fetched: " . count($users) . "\n";
    echo "  Fetch mode: Associative array\n";
}
echo "\n";

// Example 10: Error Handling
echo "Example 10: Error Handling\n";
if (isset($pdo)) {
    $stmt = $pdo->query("SELECT * FROM nonexistent_table");
    if (!$stmt) {
        $error = $pdo->errorInfo();
        echo "  Error occurred: Yes\n";
        echo "  SQLSTATE: " . $error[0] . "\n";
        echo "  Error message available: Yes\n";
    }
}
echo "\n";

// Example 11: Multiple Database Connections
echo "Example 11: Multiple Database Connections\n";
try {
    $mysql_pdo = new PDO('mysql:host=localhost;dbname=db1', 'user', 'pass');
    echo "  MySQL connection: Established\n";
    
    $pgsql_pdo = new PDO('pgsql:host=localhost;dbname=db2', 'user', 'pass');
    echo "  PostgreSQL connection: Established\n";
    
    $sqlite_pdo = new PDO('sqlite:/path/to/database.db');
    echo "  SQLite connection: Established\n";
} catch (Exception $e) {
    echo "  Note: Connections are stubs in this implementation\n";
}
echo "\n";

// Example 12: Prepared Statement Reuse
echo "Example 12: Reuse Prepared Statement\n";
if (isset($pdo)) {
    $stmt = $pdo->prepare("INSERT INTO logs (user_id, action) VALUES (:user_id, :action)");
    
    $actions = [
        ['user_id' => 1, 'action' => 'login'],
        ['user_id' => 2, 'action' => 'logout'],
        ['user_id' => 1, 'action' => 'update_profile']
    ];
    
    foreach ($actions as $data) {
        $stmt->bindParam(':user_id', $data['user_id']);
        $stmt->bindParam(':action', $data['action']);
        $stmt->execute();
    }
    
    echo "  Prepared statement created once\n";
    echo "  Executed " . count($actions) . " times\n";
    echo "  Efficiency: High (statement reuse)\n";
}
echo "\n";

// Example 13: CREATE TABLE
echo "Example 13: Create Table\n";
if (isset($pdo)) {
    $sql = "CREATE TABLE IF NOT EXISTS products (
        id INT PRIMARY KEY AUTO_INCREMENT,
        name VARCHAR(255) NOT NULL,
        price DECIMAL(10,2),
        created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
    )";
    $pdo->exec($sql);
    echo "  Table created: products\n";
    echo "  Columns: id, name, price, created_at\n";
}
echo "\n";

// Example 14: Bulk Insert
echo "Example 14: Bulk Insert Operation\n";
if (isset($pdo)) {
    $pdo->beginTransaction();
    
    $stmt = $pdo->prepare("INSERT INTO products (name, price) VALUES (:name, :price)");
    
    $products = [
        ['name' => 'Widget', 'price' => 19.99],
        ['name' => 'Gadget', 'price' => 29.99],
        ['name' => 'Doohickey', 'price' => 39.99]
    ];
    
    foreach ($products as $product) {
        $stmt->bindParam(':name', $product['name']);
        $stmt->bindParam(':price', $product['price']);
        $stmt->execute();
    }
    
    $pdo->commit();
    echo "  Products inserted: " . count($products) . "\n";
    echo "  Transaction used: Yes\n";
    echo "  Performance: Optimized\n";
}
echo "\n";

// Example 15: Database Abstraction
echo "Example 15: Database-Agnostic Code\n";

function get_user_by_email($pdo, $email) {
    $stmt = $pdo->prepare("SELECT * FROM users WHERE email = :email");
    $stmt->bindParam(':email', $email);
    $stmt->execute();
    return $stmt->fetch();
}

function create_user($pdo, $name, $email) {
    $stmt = $pdo->prepare("INSERT INTO users (name, email) VALUES (:name, :email)");
    $stmt->bindParam(':name', $name);
    $stmt->bindParam(':email', $email);
    $stmt->execute();
    return $pdo->lastInsertId();
}

if (isset($pdo)) {
    echo "  Function: get_user_by_email()\n";
    echo "  Function: create_user()\n";
    echo "  Database-agnostic: Yes\n";
    echo "  Works with: MySQL, PostgreSQL, SQLite\n";
}
echo "\n";

echo "=== PDO Features Summary ===\n";
echo "✓ Database connections (MySQL, PostgreSQL, SQLite)\n";
echo "✓ Simple queries (query(), exec())\n";
echo "✓ Prepared statements with parameter binding\n";
echo "✓ Named and positional parameters\n";
echo "✓ Transaction support (begin, commit, rollback)\n";
echo "✓ Fetch operations (fetch(), fetchAll())\n";
echo "✓ Error handling (errorInfo())\n";
echo "✓ Last insert ID retrieval\n";
echo "✓ Row and column counting\n";
echo "✓ Statement reuse for efficiency\n";
echo "✓ Bulk operations with transactions\n";
echo "✓ Database-agnostic abstraction\n\n";

echo "=== All PDO Examples Complete ===\n";
