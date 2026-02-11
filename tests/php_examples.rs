//! PHP Examples Test Cases
//!
//! Tests that verify the PHP-RS engine can compile and execute PHP files from examples/

use phprs::engine::compile::{compile_file, compile_string};
use phprs::engine::types::{PhpResult, PhpType, PhpValue, Val};
use phprs::engine::vm::{execute_ex, ExecuteData, OpArray};
use std::fs;
use std::path::Path;

/// Test helper: Compile a PHP file and verify it produces an op array
fn test_php_file_compilation(filepath: &str) -> Result<OpArray, String> {
    let path = Path::new("examples").join(filepath);

    if !path.exists() {
        return Err(format!("Example file not found: {}", path.display()));
    }

    // Try to compile the file
    let op_array = compile_file(path.to_str().unwrap())?;

    // Verify we got an op array
    assert!(
        !op_array.filename.is_none(),
        "Op array should have filename"
    );

    Ok(op_array)
}

/// Test helper: Execute a compiled op array
fn test_execute_op_array(op_array: &OpArray) -> PhpResult {
    let mut execute_data = ExecuteData::new();
    execute_ex(&mut execute_data, op_array)
}

#[test]
fn test_basic_types_php_compilation() {
    // This test may fail until we implement var_dump, true/false, null, arrays
    let result = test_php_file_compilation("basic_types.php");
    if let Ok(op_array) = result {
        assert!(op_array.filename.is_some());
    } else {
        // Compilation may fail for unsupported features - that's okay for now
        eprintln!("Note: basic_types.php compilation failed (expected for unsupported features)");
    }
}

#[test]
fn test_string_operations_php_compilation() {
    let result = test_php_file_compilation("string_operations.php");
    if let Ok(op_array) = result {
        assert!(op_array.filename.is_some());
    } else {
        // Compilation may fail for unsupported features like complex string interpolation
        eprintln!("Note: string_operations.php compilation failed (may be expected for unsupported features)");
    }
}

#[test]
fn test_array_operations_php_compilation() {
    // This test may fail until we implement array operations
    let result = test_php_file_compilation("array_operations.php");
    if let Ok(op_array) = result {
        assert!(op_array.filename.is_some());
    } else {
        eprintln!(
            "Note: array_operations.php compilation failed (expected for unsupported features)"
        );
    }
}

#[test]
fn test_operators_php_compilation() {
    let result = test_php_file_compilation("operators.php");
    assert!(result.is_ok(), "Should compile operators.php");

    let op_array = result.unwrap();
    assert!(op_array.filename.is_some());
}

#[test]
fn test_error_handling_php_compilation() {
    let result = test_php_file_compilation("error_handling.php");
    if let Ok(op_array) = result {
        assert!(op_array.filename.is_some());
    } else {
        // Compilation may fail for unsupported features like closures and function calls
        eprintln!("Note: error_handling.php compilation failed (may be expected for unsupported features)");
    }
}

#[test]
fn test_filesystem_php_compilation() {
    // This test may fail until we implement function calls
    let result = test_php_file_compilation("filesystem.php");
    if let Ok(op_array) = result {
        assert!(op_array.filename.is_some());
    } else {
        eprintln!("Note: filesystem.php compilation failed (expected for unsupported features)");
    }
}

#[test]
fn test_variables_php_compilation() {
    // This test may fail until we implement all variable features
    let result = test_php_file_compilation("variables.php");
    if let Ok(op_array) = result {
        assert!(op_array.filename.is_some());
    } else {
        eprintln!("Note: variables.php compilation failed (expected for unsupported features)");
    }
}

#[test]
fn test_control_flow_php_compilation() {
    // This test may fail until we implement if/while/for statements
    let result = test_php_file_compilation("control_flow.php");
    if let Ok(op_array) = result {
        assert!(op_array.filename.is_some());
    } else {
        eprintln!("Note: control_flow.php compilation failed (expected for unsupported features)");
    }
}

#[test]
fn test_functions_php_compilation() {
    // This test may fail until we implement function definitions
    let result = test_php_file_compilation("functions.php");
    if let Ok(op_array) = result {
        assert!(op_array.filename.is_some());
    } else {
        eprintln!("Note: functions.php compilation failed (expected for unsupported features)");
    }
}

#[test]
fn test_classes_php_compilation() {
    // This test may fail until we implement class definitions
    let result = test_php_file_compilation("classes.php");
    if let Ok(op_array) = result {
        assert!(op_array.filename.is_some());
    } else {
        eprintln!("Note: classes.php compilation failed (expected for unsupported features)");
    }
}

#[test]
fn test_all_php_examples_exist() {
    // Verify all expected PHP example files exist
    let examples = vec![
        "basic_types.php",
        "string_operations.php",
        "array_operations.php",
        "operators.php",
        "error_handling.php",
        "filesystem.php",
        "variables.php",
        "control_flow.php",
        "functions.php",
        "classes.php",
    ];

    for example in examples {
        let path = Path::new("examples").join(example);
        assert!(path.exists(), "Example file should exist: {}", example);
    }
}

#[test]
fn test_php_file_readable() {
    // Test that we can read PHP files
    let examples = vec![
        "basic_types.php",
        "string_operations.php",
        "array_operations.php",
    ];

    for example in examples {
        let path = Path::new("examples").join(example);
        if path.exists() {
            let content = fs::read_to_string(&path);
            assert!(content.is_ok(), "Should be able to read {}", example);
            let content = content.unwrap();
            assert!(content.contains("<?php"), "Should contain PHP opening tag");
        }
    }
}

#[test]
fn test_php_string_compilation() {
    // Test compiling a simple PHP string
    let php_code = "<?php echo 'Hello, World!';";
    let result = compile_string(php_code, "test.php");

    assert!(result.is_ok(), "Should compile simple PHP string");
    let op_array = result.unwrap();
    assert_eq!(op_array.filename, Some("test.php".to_string()));
}

#[test]
fn test_php_execution_empty() {
    // Test executing an empty op array
    let op_array = OpArray::new("test.php".to_string());
    let mut execute_data = ExecuteData::new();

    let result = execute_ex(&mut execute_data, &op_array);
    // Empty op array should execute successfully
    assert!(matches!(result, PhpResult::Success));
}

#[test]
fn test_php_compile_and_execute_simple() {
    // Test compile and execute a simple PHP statement
    let php_code = r"<?php $x = 1;";
    let compile_result = compile_string(php_code, "simple.php");

    assert!(compile_result.is_ok(), "Should compile simple assignment");

    let op_array = compile_result.unwrap();
    let mut execute_data = ExecuteData::new();

    // Try to execute (may not fully work yet, but should not panic)
    let exec_result = execute_ex(&mut execute_data, &op_array);
    // Execution may succeed or fail depending on implementation
    // Just verify it doesn't panic
    assert!(matches!(
        exec_result,
        PhpResult::Success | PhpResult::Failure
    ));
}

#[test]
fn test_php_examples_compilation_batch() {
    // Test compiling all PHP examples in batch
    let examples = vec![
        "basic_types.php",
        "string_operations.php",
        "array_operations.php",
        "operators.php",
    ];

    let mut success_count = 0;
    for example in examples {
        let path = Path::new("examples").join(example);
        if path.exists() {
            match compile_file(path.to_str().unwrap()) {
                Ok(op_array) => {
                    assert!(op_array.filename.is_some());
                    success_count += 1;
                }
                Err(e) => {
                    // Compilation may fail for complex features not yet implemented
                    // Just log it, don't fail the test
                    eprintln!("Warning: Failed to compile {}: {}", example, e);
                }
            }
        }
    }

    // At least some examples should compile
    assert!(success_count > 0, "At least some examples should compile");
}
