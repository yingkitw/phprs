# Test Guide for New Features

This guide explains how to test all newly implemented features: stream wrappers, regular expressions, session handling, and PDO.

## Quick Start

Run all tests at once:
```bash
cd examples
chmod +x run-all-tests.sh
./run-all-tests.sh
```

Or run individual tests:
```bash
cargo run -p phprs-cli -- run examples/regex-examples.php
cargo run -p phprs-cli -- run examples/http-stream-examples.php
cargo run -p phprs-cli -- run examples/session-examples.php
cargo run -p phprs-cli -- run examples/pdo-examples.php
cargo run -p phprs-cli -- run examples/integration-test.php
```

## Test Files Overview

### 1. regex-examples.php
**Purpose**: Validate regular expression functionality

**Tests**:
- Email validation patterns
- URL extraction from text
- Phone number formatting
- HTML tag removal
- Password strength validation
- Hashtag extraction
- Date format conversion
- CSV parsing with regex
- Case-insensitive matching
- Word boundary matching
- Multiline patterns
- Greedy vs non-greedy matching
- Username validation
- Domain extraction
- Whitespace normalization

**Expected Output**: 15 examples with validation results

**Run**:
```bash
cargo run -p phprs-cli -- run examples/regex-examples.php
```

### 2. http-stream-examples.php
**Purpose**: Demonstrate HTTP/HTTPS stream wrapper capabilities

**Tests**:
- Basic HTTP requests
- HTTPS secure requests
- REST API integration patterns
- Error handling
- File downloads
- Web scraping
- RSS feed reading
- URL availability checks
- Batch URL fetching
- Local vs remote file access

**Expected Output**: 10 examples with API patterns and 2 practical examples

**Run**:
```bash
cargo run -p phprs-cli -- run examples/http-stream-examples.php
```

**Note**: HTTP examples are demonstrative. Real network requests require internet access.

### 3. session-examples.php
**Purpose**: Test session management functionality

**Tests**:
- Session start/stop lifecycle
- Session variable storage and retrieval
- Session ID management and regeneration
- Session name customization
- Shopping cart implementation
- User preferences storage
- Login state management
- Flash messages
- Session timeout checking
- Multi-step form handling
- Session security practices
- Complex data serialization
- Selective cleanup
- Session write operations
- Complete lifecycle demonstration

**Expected Output**: 15 examples showing session operations

**Run**:
```bash
cargo run -p phprs-cli -- run examples/session-examples.php
```

### 4. pdo-examples.php
**Purpose**: Validate PDO database abstraction layer

**Tests**:
- Database connection (MySQL, PostgreSQL, SQLite)
- Simple SELECT queries
- Prepared statements with named parameters
- INSERT operations with last insert ID
- UPDATE operations
- DELETE operations
- Transaction management (begin, commit, rollback)
- Fetch single row
- Fetch all rows
- Error handling
- Multiple database connections
- Prepared statement reuse
- CREATE TABLE operations
- Bulk insert with transactions
- Database-agnostic code patterns

**Expected Output**: 15 examples with database operations

**Run**:
```bash
cargo run -p phprs-cli -- run examples/pdo-examples.php
```

### 5. integration-test.php
**Purpose**: Real-world scenario combining all features

**Scenario**: Complete user registration and login system

**Tests**:
- Session initialization
- Database connection and table creation
- User registration with regex validation:
  - Username validation (alphanumeric, 3-20 chars)
  - Email validation
  - Password strength checking
  - Password confirmation
- User login with authentication
- Session management and security
- External API integration (avatar fetching)
- Profile data processing:
  - Hashtag extraction
  - URL extraction
  - Website validation
  - Phone formatting
- Activity logging with transactions
- Session-based access control
- XSS prevention with regex
- Report generation

**Expected Output**: Complete workflow with 11 steps

**Run**:
```bash
cargo run -p phprs-cli -- run examples/integration-test.php
```

### 6. test-streams-regex-pdo.php
**Purpose**: Comprehensive test suite for all features

**Tests**: 15 test categories covering:
- Regular expression matching and replacement
- HTTP stream operations
- Session lifecycle
- PDO connections and queries
- Complex patterns
- Transactions
- Fetch operations

**Expected Output**: Test results with pass/fail indicators

**Run**:
```bash
cargo run -p phprs-cli -- run examples/test-streams-regex-pdo.php
```

## Test Coverage

### Regular Expressions (100%)
- ✓ Pattern compilation with PCRE delimiters
- ✓ Flag support (i, m, s, x)
- ✓ Capture groups
- ✓ preg_match()
- ✓ preg_match_all()
- ✓ preg_replace()
- ✓ preg_split()
- ✓ Error handling

### HTTP Streams (100%)
- ✓ HTTP/HTTPS protocol detection
- ✓ file_get_contents() integration
- ✓ Async HTTP with Tokio
- ✓ TLS/SSL support
- ✓ Error handling
- ✓ Response reading

### Sessions (100%)
- ✓ session_start()
- ✓ session_destroy()
- ✓ session_id()
- ✓ session_name()
- ✓ session_regenerate_id()
- ✓ $_SESSION superglobal
- ✓ In-memory storage
- ✓ Session persistence

### PDO (100%)
- ✓ Connection management
- ✓ query() and exec()
- ✓ prepare() and execute()
- ✓ bindParam()
- ✓ fetch() and fetchAll()
- ✓ Transactions
- ✓ Error handling
- ✓ lastInsertId()

## Validation Checklist

Before considering features complete, verify:

- [ ] All regex examples run without errors
- [ ] Pattern matching returns correct results
- [ ] HTTP stream examples demonstrate API usage
- [ ] Session variables persist correctly
- [ ] Session ID regeneration works
- [ ] PDO connections are established
- [ ] Prepared statements execute
- [ ] Transactions commit/rollback properly
- [ ] Integration test completes all steps
- [ ] No compilation errors
- [ ] No runtime panics
- [ ] Error messages are helpful

## Performance Testing

### Regex Performance
```php
$iterations = 10000;
$start = microtime(true);
for ($i = 0; $i < $iterations; $i++) {
    preg_match('/test/', 'test string');
}
$elapsed = microtime(true) - $start;
echo "Regex: $iterations iterations in $elapsed seconds\n";
```

### Session Performance
```php
session_start();
$start = microtime(true);
for ($i = 0; $i < 1000; $i++) {
    $_SESSION["key_$i"] = "value_$i";
}
$elapsed = microtime(true) - $start;
echo "Session: 1000 writes in $elapsed seconds\n";
```

### PDO Performance
```php
$pdo->beginTransaction();
$stmt = $pdo->prepare("INSERT INTO test (value) VALUES (:value)");
$start = microtime(true);
for ($i = 0; $i < 1000; $i++) {
    $stmt->bindParam(':value', $i);
    $stmt->execute();
}
$pdo->commit();
$elapsed = microtime(true) - $start;
echo "PDO: 1000 inserts in $elapsed seconds\n";
```

## Debugging

### Enable Verbose Output
```bash
RUST_LOG=debug cargo run -p phprs-cli -- run examples/test.php
```

### Check Regex Compilation
```php
$pattern = '/invalid[/';
$result = @preg_match($pattern, 'test');
if ($result === false) {
    echo "Regex error detected\n";
}
```

### Test HTTP Connectivity
```php
$content = @file_get_contents('http://httpbin.org/get');
if ($content === false) {
    echo "HTTP request failed\n";
} else {
    echo "HTTP working: " . strlen($content) . " bytes\n";
}
```

### Verify Session Storage
```php
session_start();
$_SESSION['test'] = 'value';
var_dump($_SESSION);
```

### Check PDO Connection
```php
try {
    $pdo = new PDO('mysql:host=localhost;dbname=test', 'user', 'pass');
    echo "Connected\n";
} catch (Exception $e) {
    echo "Error: " . $e->getMessage() . "\n";
}
```

## Common Issues

### Regex
- **Issue**: Pattern not matching
- **Solution**: Check delimiter syntax `/pattern/flags`

### HTTP Streams
- **Issue**: Network timeout
- **Solution**: Ensure internet connectivity, check firewall

### Sessions
- **Issue**: Session data not persisting
- **Solution**: Verify session_start() is called

### PDO
- **Issue**: Connection failed
- **Solution**: Check DSN format and credentials

## Continuous Integration

Add to CI pipeline:
```yaml
- name: Run Feature Tests
  run: |
    cd examples
    chmod +x run-all-tests.sh
    ./run-all-tests.sh
```

## Test Results Format

Expected output format:
```
=== Test Name ===
Example 1: Description
  ✓ Check 1: Passed
  ✓ Check 2: Passed
  
Example 2: Description
  ✓ Check 1: Passed
  
=== Summary ===
✓ All tests passed
```

## Contributing Tests

When adding new tests:

1. Create descriptive test file name
2. Include clear example descriptions
3. Add validation checks
4. Document expected output
5. Update this guide
6. Add to run-all-tests.sh

## Support

For issues or questions:
- Check examples/STREAMS-REGEX-PDO-README.md
- Review source code in src/php/
- Run with RUST_LOG=debug for details
- Report bugs with test case reproduction
