# phprs Quick Start Guide

Get up and running with phprs in 5 minutes!

## Installation

### Option 1: Build from Source (Recommended)
```bash
# Install Rust (if needed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/yingkitw/phprs.git
cd phprs
cargo build --release

# The binary is at: target/release/phprs
```

### Option 2: Install with Cargo
```bash
cargo install --git https://github.com/yingkitw/phprs.git phprs-cli
```

## Your First PHP Script

### 1. Create a PHP File
```bash
echo '<?php echo "Hello from phprs!\n";' > hello.php
```

### 2. Run It
```bash
# Using the built binary
./target/release/phprs run hello.php

# Or during development
cargo run -p phprs-cli -- run hello.php
```

**Output**:
```
Hello from phprs!
```

That's it! No configuration, no setup - just run your PHP code.

## Common Use Cases

### Run a Web Application
```bash
# Start the built-in development server
phprs serve

# Custom port
phprs serve --port 8080

# Open http://localhost:8080 in your browser
```

### Run WordPress
```bash
cd /path/to/wordpress
phprs run index.php

# Or start the dev server
phprs serve --port 8080
```

### Use Regular Expressions
```php
<?php
// regex-demo.php
$email = "user@example.com";

if (preg_match('/^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$/', $email)) {
    echo "Valid email!\n";
}

$text = "Hello World";
$result = preg_replace('/World/', 'phprs', $text);
echo $result . "\n"; // "Hello phprs"
```

```bash
phprs run regex-demo.php
```

### Fetch Data from APIs
```php
<?php
// api-demo.php
$json = file_get_contents('https://api.github.com/repos/rust-lang/rust');
$data = json_decode($json, true);

echo "Repository: " . $data['name'] . "\n";
echo "Stars: " . $data['stargazers_count'] . "\n";
```

```bash
phprs run api-demo.php
```

### Work with Databases
```php
<?php
// database-demo.php
$pdo = new PDO('mysql:host=localhost;dbname=myapp', 'user', 'password');

// Prepared statement (SQL injection safe)
$stmt = $pdo->prepare('SELECT * FROM users WHERE email = :email');
$stmt->bindParam(':email', $email);
$stmt->execute();

$user = $stmt->fetch();
print_r($user);
```

```bash
phprs run database-demo.php
```

### Manage Sessions
```php
<?php
// session-demo.php
session_start();

// Store data
$_SESSION['user_id'] = 123;
$_SESSION['username'] = 'john_doe';

// Retrieve data
echo "Welcome, " . $_SESSION['username'] . "!\n";
echo "Your ID: " . $_SESSION['user_id'] . "\n";
```

```bash
phprs run session-demo.php
```

## Package Management

### Initialize a Project
```bash
phprs pkg init
```

This creates a `composer.json` file.

### Install Dependencies
```bash
phprs pkg install
```

### Add a Package
```bash
phprs pkg require vendor/package-name
```

## Running Examples

phprs comes with 80+ examples demonstrating all features:

```bash
# Basic examples
phprs run examples/01_hello_world.php
phprs run examples/array_operations.php

# Feature examples
phprs run examples/regex-examples.php        # 15 regex patterns
phprs run examples/http-stream-examples.php  # HTTP/API integration
phprs run examples/session-examples.php      # Session management
phprs run examples/pdo-examples.php          # Database operations

# WordPress
phprs run examples/wordpress/index.php
phprs run examples/wordpress/test-theme-plugin.php

# Integration test (all features)
phprs run examples/integration-test.php

# Run all tests
cd examples
chmod +x run-all-tests.sh
./run-all-tests.sh
```

## Migrating Existing PHP Projects

### From PHP 8.x

**No changes needed!** Just run your existing code:

```bash
# Your existing PHP project
cd /path/to/your/project

# Run with phprs
phprs run index.php
```

### From Apache/Nginx + PHP-FPM

Replace your PHP interpreter:

**Before**:
```bash
php index.php
```

**After**:
```bash
phprs run index.php
```

### WordPress Sites

1. Navigate to your WordPress directory
2. Run with phprs:

```bash
cd /var/www/wordpress
phprs serve --port 8080
```

Visit `http://localhost:8080` - your WordPress site runs on phprs!

### Laravel Applications

```bash
cd your-laravel-app

# Run artisan commands
phprs run artisan serve

# Run tests
phprs run artisan test
```

## Development Workflow

### 1. Write PHP Code
Use your favorite editor - VS Code, PHPStorm, Vim, etc.

### 2. Run with phprs
```bash
phprs run your-script.php
```

### 3. Debug
Enable verbose output:
```bash
RUST_LOG=debug phprs run your-script.php
```

### 4. Test
```bash
# Run tests
cargo test

# Run specific example
phprs run examples/test-streams-regex-pdo.php
```

### 5. Deploy
```bash
# Build optimized binary
cargo build --release

# Deploy the binary
cp target/release/phprs /usr/local/bin/

# Run in production
phprs run production-app.php
```

## Performance Tips

### 1. Use Release Build
```bash
cargo build --release
# Binary is 2-3x faster than debug build
```

### 2. Enable JIT
JIT is enabled by default for hot code paths.

### 3. Use Opcode Caching
Opcode caching is automatic - frequently executed code is cached.

### 4. Optimize Regex Patterns
Patterns are compiled once and cached:
```php
// Good - pattern compiled once
for ($i = 0; $i < 1000; $i++) {
    preg_match('/pattern/', $text);
}
```

### 5. Use Prepared Statements
```php
// Good - statement prepared once
$stmt = $pdo->prepare('SELECT * FROM users WHERE id = :id');
for ($i = 0; $i < 1000; $i++) {
    $stmt->bindParam(':id', $i);
    $stmt->execute();
}
```

## Common Issues

### Issue: "Command not found: phprs"
**Solution**: Add to PATH or use full path:
```bash
export PATH="$PATH:/path/to/phprs/target/release"
# Or
./target/release/phprs run script.php
```

### Issue: "Compilation errors"
**Solution**: Ensure Rust 1.75+ is installed:
```bash
rustc --version
rustup update
```

### Issue: "HTTP requests fail"
**Solution**: Check network connectivity and firewall settings.

### Issue: "Session data not persisting"
**Solution**: Ensure `session_start()` is called before accessing `$_SESSION`.

## Next Steps

1. **Explore Examples**: Check out `examples/` directory for 80+ working examples
2. **Read Documentation**: See [README.md](README.md) for full feature list
3. **Try WordPress**: Run the WordPress example in `examples/wordpress/`
4. **Build Something**: Start building your PHP application with phprs!

## Getting Help

- **Documentation**: [README.md](README.md), [docs/](docs/)
- **Examples**: [examples/](examples/)
- **Issues**: [GitHub Issues](https://github.com/yingkitw/phprs/issues)
- **Discussions**: [GitHub Discussions](https://github.com/yingkitw/phprs/discussions)

## What's Next?

- **Learn More**: Read [ARCHITECTURE.md](ARCHITECTURE.md) to understand how phprs works
- **Performance**: Check [PERFORMANCE.md](PERFORMANCE.md) for benchmarks
- **Contribute**: See [README.md#contributing](README.md#contributing)
- **Stay Updated**: Watch the repository for new features

---

**Happy coding with phprs!** 🚀
