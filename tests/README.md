# Test Suite Documentation

This directory contains comprehensive test suites for phprs at different levels.

## Test Structure

### Unit Tests
Unit tests are located alongside the source code in `src/` directories:
- `src/engine/string/tests.rs` - String operations
- `src/engine/hash/tests.rs` - Hash table operations
- `src/engine/operators/tests.rs` - Operator functions (inline)
- `src/engine/alloc/tests.rs` - Memory allocation
- `src/engine/gc/tests.rs` - Garbage collection
- `src/php/filesystem/tests.rs` - Filesystem functions
- `src/php/ini/tests.rs` - INI configuration
- `src/php/variables/tests.rs` - Variable handling

### Integration Tests
Located in `tests/integration_tests.rs`:
- Tests interactions between multiple modules
- Verifies end-to-end functionality
- Tests module composition

### Edge Case Tests
Located in `tests/edge_cases.rs`:
- Boundary conditions
- Error cases
- Unusual inputs
- Stress tests

### Error Handling Tests
Located in `tests/error_handling.rs`:
- Error reporting
- Error handlers
- Different error types

## Running Tests

### Run all tests
```bash
cargo test
```

### Run only unit tests
```bash
cargo test --lib
```

### Run only integration tests
```bash
cargo test --test integration_tests
```

### Run only edge case tests
```bash
cargo test --test edge_cases
```

### Run only error handling tests
```bash
cargo test --test error_handling
```

### Run specific test
```bash
cargo test test_name
```

### Run tests with output
```bash
cargo test -- --nocapture
```

## Test Coverage Goals

- **Unit Tests**: Cover all public functions with:
  - Normal cases
  - Edge cases
  - Error cases
  
- **Integration Tests**: Cover:
  - Module interactions
  - Data flow between modules
  - Complex scenarios
  
- **Edge Case Tests**: Cover:
  - Empty inputs
  - Very large inputs
  - Boundary values
  - Special characters
  - Unicode handling

## Adding New Tests

When adding new functionality:

1. **Add unit tests** in the module's test file or `#[cfg(test)]` block
2. **Add integration tests** if the feature interacts with other modules
3. **Add edge case tests** for boundary conditions
4. **Add error handling tests** if the feature can fail

## Test Naming Convention

- Unit tests: `test_<function_name>_<scenario>`
- Integration tests: `test_<feature>_<scenario>`
- Edge case tests: `test_<feature>_edge_<case>`
- Error tests: `test_<feature>_error_<type>`

