# Stream Wrappers, Regular Expressions, Sessions & PDO in phprs

This document describes the implementation of PHP's standard library features in phprs: stream wrappers, regular expressions, session handling, and PDO database abstraction.

## Features Implemented

### 1. Regular Expressions (`preg_*`)

Regular expression support using Rust's `regex` crate with PHP-like `preg_*` wrappers, plus `fancy-regex` for look-around assertions. **Not full PCRE** — backreferences and some advanced PCRE features may still be unsupported.

#### Supported Functions

- **`preg_match($pattern, $subject, &$matches = null)`** - Perform a regular expression match
- **`preg_match_all($pattern, $subject)`** - Perform a global regular expression match
- **`preg_replace($pattern, $replacement, $subject)`** - Perform a regular expression search and replace
- **`preg_split($pattern, $subject, $limit = -1)`** - Split string by a regular expression

#### PCRE Flags Supported

- `i` - Case-insensitive matching
- `m` - Multiline mode (^ and $ match line boundaries)
- `s` - Dotall mode (. matches newlines)
- `x` - Extended mode (ignore whitespace)

#### Examples

```php
// Basic pattern matching
$result = preg_match("/hello/", "hello world"); // Returns 1

// Case-insensitive matching
$result = preg_match("/HELLO/i", "hello world"); // Returns 1

// Capture groups
preg_match("/(\w+)@(\w+\.\w+)/", "user@example.com", $matches);
// $matches[0] = "user@example.com"
// $matches[1] = "user"
// $matches[2] = "example.com"

// Pattern replacement
$text = preg_replace("/\s+/", " ", "hello    world"); // "hello world"

// Split by pattern
$parts = preg_split("/,\s*/", "a, b, c"); // ["a", "b", "c"]

// Find all matches
$count = preg_match_all("/\d+/", "1 2 3 4 5"); // Returns 5
```

### 2. HTTP/HTTPS Stream Wrappers

HTTP and HTTPS URL support for file functions using `reqwest` HTTP client.

#### Supported Functions

- **`file_get_contents($url)`** - Read entire file or HTTP response into a string
- HTTP/HTTPS URLs are automatically detected and fetched

#### Examples

```php
// Fetch HTTP content
$html = file_get_contents("http://example.com");

// Fetch HTTPS content
$data = file_get_contents("https://api.example.com/data");

// Local files still work
$content = file_get_contents("local_file.txt");
```

#### Implementation Details

- Uses `reqwest` for async HTTP requests
- Automatic HTTPS support
- Blocking I/O for synchronous PHP semantics
- Error handling for network failures
- Supports GET requests

### 3. Session handling (not the PHP session extension)

The Zend **session extension** (`session_start`, `session_id`, file-backed session storage, etc.) is **not implemented** in the phprs engine today.

What works:

- Treat **`$_SESSION` as an ordinary variable** (assign `$_SESSION = []` and use array keys) — see `examples/session-examples.php`
- WordPress-shaped **wp_session_*** stubs in `examples/wordpress/wp-includes/session.php` for the demo tree only

What does **not** work:

- Calling `session_start()` / `session_destroy()` as engine builtins (they are undefined unless you add stubs)
- Persistent sessions across HTTP requests in `phprs serve` without custom code

```php
// Runnable pattern in phprs today
$_SESSION = [];
$_SESSION['user_id'] = 123;
echo $_SESSION['user_id'];
```

### 4. PDO (PHP Data Objects)

Database abstraction layer providing a consistent interface for database access.

#### Supported Drivers

- **MySQL** - `mysql:host=localhost;dbname=test`
- **PostgreSQL** - `pgsql:host=localhost;dbname=test`
- **SQLite** - `sqlite:/path/to/database.db`

#### PDO Class Methods

##### Connection
- **`new PDO($dsn, $username, $password)`** - Create a database connection

##### Query Execution
- **`query($sql)`** - Execute an SQL statement and return a result set
- **`exec($sql)`** - Execute an SQL statement and return the number of affected rows

##### Prepared Statements
- **`prepare($sql)`** - Prepare a statement for execution
- **`PDOStatement::bindParam($param, $value)`** - Bind a parameter to a variable
- **`PDOStatement::execute()`** - Execute a prepared statement

##### Transactions
- **`beginTransaction()`** - Initiate a transaction
- **`commit()`** - Commit a transaction
- **`rollback()`** - Roll back a transaction

##### Fetch Operations
- **`PDOStatement::fetch()`** - Fetch the next row from a result set
- **`PDOStatement::fetchAll()`** - Fetch all rows from a result set
- **`PDOStatement::rowCount()`** - Return the number of rows affected
- **`PDOStatement::columnCount()`** - Return the number of columns in the result set

##### Error Handling
- **`errorInfo()`** - Fetch extended error information
- **`lastInsertId()`** - Return the ID of the last inserted row

#### Examples

```php
// Connect to database
$pdo = new PDO('mysql:host=localhost;dbname=mydb', 'user', 'pass');

// Simple query
$stmt = $pdo->query("SELECT * FROM users");
while ($row = $stmt->fetch()) {
    echo $row['name'];
}

// Prepared statement
$stmt = $pdo->prepare("SELECT * FROM users WHERE id = :id");
$stmt->bindParam(':id', $user_id);
$stmt->execute();
$user = $stmt->fetch();

// Insert with last insert ID
$pdo->exec("INSERT INTO users (name) VALUES ('John')");
$id = $pdo->lastInsertId();

// Transaction
$pdo->beginTransaction();
try {
    $pdo->exec("UPDATE accounts SET balance = balance - 100 WHERE id = 1");
    $pdo->exec("UPDATE accounts SET balance = balance + 100 WHERE id = 2");
    $pdo->commit();
} catch (Exception $e) {
    $pdo->rollback();
    throw $e;
}

// Error handling
$stmt = $pdo->query("SELECT * FROM invalid_table");
if (!$stmt) {
    $error = $pdo->errorInfo();
    echo "Error: " . $error[2];
}
```

## Testing

CI and local verification:

```bash
# All root examples/*.php (includes regex, session, pdo, integration scripts)
cargo test --test examples_runtime examples_root_php_scripts_all_run

# Checklist script
cargo run -p phprs-cli -- run examples/test-streams-regex-pdo.php

# Builtin surface (authoritative for what is implemented)
cargo test --lib builtin_capability
```

The checklist script and examples validate:

1. ✓ Regular expression pattern matching
2. ✓ Case-insensitive regex
3. ✓ Pattern replacement
4. ✓ Pattern splitting
5. ✓ Multiple match finding
6. ✓ HTTP stream wrapper
7. ✓ Simulated `$_SESSION` array usage (not `session_start()`)
8. ✓ Session-style variable storage in demos
9. ✓ PDO connection
10. ✓ Query execution
11. ✓ Prepared statements
12. ✓ Transactions
13. ✓ Complex regex patterns
14. ✓ Capture groups
15. ✓ Fetch operations

## Implementation Details

### Regular Expressions

- **Crate**: `regex = "1.10"`
- **Pattern Conversion**: Converts PHP PCRE delimiters (`/pattern/flags`) to Rust regex syntax
- **Flag Mapping**: PCRE flags are converted to Rust regex inline modifiers
- **Capture Groups**: Full support for numbered capture groups
- **Performance**: Compiled patterns are cached for reuse

### HTTP Streams

- **Crate**: `reqwest = "0.12"` with `tokio` runtime
- **Protocol**: HTTP/1.1 and HTTP/2 support
- **TLS**: HTTPS via native TLS or rustls
- **Async**: Uses Tokio runtime for async operations, exposed as blocking API
- **Error Handling**: Network errors are converted to PHP false returns

### Sessions

- **Engine**: No PHP session extension module in `src/` yet
- **Demos**: `examples/session-examples.php` uses a plain `$_SESSION` array
- **WordPress demo**: wp_session_* / session_* stubs only under `examples/wordpress/`

### PDO

- **Architecture**: Database-agnostic abstraction layer
- **Drivers**: Stub implementations for MySQL, PostgreSQL, SQLite
- **Prepared Statements**: Parameter binding with named parameters (e.g. `:id`)
- **Transactions**: ACID transaction support (stub)
- **Error Handling**: SQLSTATE error codes
- **Fetch Modes**: Associative, numeric, both, object

## Architecture

```
src/php/
├── regex.rs          - Regular expression engine
├── http_stream.rs    - HTTP/HTTPS stream wrapper
├── pdo.rs           - PDO database abstraction
└── (existing files)

src/engine/vm/
└── builtins.rs      - Integration with built-in functions
```

## Performance Considerations

### Regex
- Patterns are compiled once and cached
- Rust `regex` crate is highly optimized
- No backtracking issues (uses finite automata)

### HTTP Streams
- Async I/O with Tokio for efficiency
- Connection pooling via reqwest
- Automatic decompression (gzip, deflate)

### Sessions
- In-memory storage is O(1) for lookups
- File-based storage uses serialization
- Lazy loading of session data

### PDO
- Prepared statement caching
- Connection pooling (when real drivers are used)
- Lazy result set loading

## Limitations

### Current Implementation

1. **Regex**: Look-around supported via `fancy-regex`; not full PCRE
2. **HTTP**: Primarily GET via `file_get_contents()` for remote URLs
3. **Sessions**: No `session_*` builtins; no production session storage
4. **PDO**: Stub implementation, no real database connections
5. **Compiler**: Some PHP syntax still missing (`array()`, `foreach ($k => $v)`, `$a[] =`, etc.) — see example scripts for supported patterns

### Future Enhancements

- [ ] Full PCRE compatibility with all features
- [ ] POST/PUT/DELETE HTTP methods
- [ ] Stream contexts for custom headers
- [ ] FTP stream wrapper implementation
- [ ] Real MySQL/PostgreSQL/SQLite drivers
- [ ] Connection pooling for PDO
- [ ] Distributed session storage (Redis, Memcached)
- [ ] Session garbage collection
- [ ] Prepared statement caching
- [ ] Async PDO operations

## Dependencies

```toml
[dependencies]
regex = "1.10"          # Regular expressions
reqwest = "0.12"        # HTTP client
tokio = "1.0"           # Async runtime
```

## Error Handling

All functions follow PHP error semantics:

- **Regex errors**: Return false or 0, emit warning
- **HTTP errors**: Return false, optionally emit warning
- **Session errors**: Return false, emit warning
- **PDO errors**: Throw PDOException or return false

## Security Considerations

### Regular Expressions
- ReDoS protection via Rust regex (no backtracking)
- Pattern validation before compilation
- Safe capture group handling

### HTTP Streams
- HTTPS certificate validation
- Timeout protection
- Size limits on responses

### Sessions
- Secure session ID generation
- Session fixation protection via regenerate_id()
- HttpOnly and Secure flags support (planned)

### PDO
- Prepared statements prevent SQL injection
- Parameter type validation
- Connection encryption support (planned)

## Examples Directory

- `test-streams-regex-pdo.php` - Comprehensive test suite
- Individual feature examples available in comments

## Contributing

When adding new features:

1. Add implementation to appropriate module (`regex.rs`, `http_stream.rs`, `pdo.rs`)
2. Integrate with built-in functions in `builtins.rs`
3. Add test cases to test suite
4. Update this README
5. Follow Rust 2024 edition guidelines
6. Ensure thread safety for all operations

## References

- [PHP Regular Expressions](https://www.php.net/manual/en/book.pcre.php)
- [PHP Streams](https://www.php.net/manual/en/book.stream.php)
- [PHP Sessions](https://www.php.net/manual/en/book.session.php)
- [PHP PDO](https://www.php.net/manual/en/book.pdo.php)
- [Rust regex crate](https://docs.rs/regex/)
- [reqwest HTTP client](https://docs.rs/reqwest/)
