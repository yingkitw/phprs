#!/bin/bash
# Test Runner for All New Features
# Runs all test cases and examples to validate functionality

echo "========================================"
echo "  phprs Feature Test Suite"
echo "========================================"
echo ""

PHPRS_CLI="cargo run -p phprs-cli --"
EXAMPLES_DIR="examples"
FAILED=0
PASSED=0

run_test() {
    local test_file=$1
    local test_name=$2
    
    echo "Running: $test_name"
    echo "----------------------------------------"
    
    if $PHPRS_CLI run "$EXAMPLES_DIR/$test_file" 2>&1; then
        echo "✓ PASSED: $test_name"
        ((PASSED++))
    else
        echo "✗ FAILED: $test_name"
        ((FAILED++))
    fi
    echo ""
}

echo "Starting test suite..."
echo ""

# Test 1: Regular Expressions
run_test "regex-examples.php" "Regular Expression Examples"

# Test 2: HTTP Streams
run_test "http-stream-examples.php" "HTTP Stream Wrapper Examples"

# Test 3: Sessions
run_test "session-examples.php" "Session Handling Examples"

# Test 4: PDO
run_test "pdo-examples.php" "PDO Database Examples"

# Test 5: Integration Test
run_test "integration-test.php" "Integration Test (All Features)"

# Test 6: Comprehensive Test Suite
run_test "test-streams-regex-pdo.php" "Comprehensive Test Suite"

# Test 7: WordPress Theme/Plugin
run_test "wordpress/test-theme-plugin.php" "WordPress Theme & Plugin Test"

echo "========================================"
echo "  Test Results Summary"
echo "========================================"
echo "Total Tests: $((PASSED + FAILED))"
echo "Passed: $PASSED"
echo "Failed: $FAILED"
echo ""

if [ $FAILED -eq 0 ]; then
    echo "✓ All tests passed!"
    exit 0
else
    echo "✗ Some tests failed"
    exit 1
fi
