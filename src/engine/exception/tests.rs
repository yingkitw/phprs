//! Unit tests for exception handling

use crate::engine::exception::*;

#[test]
fn test_exception_creation() {
    let ex = PhpException::new(ExceptionClass::Exception, "Test error");
    assert_eq!(ex.get_message(), "Test error");
    assert_eq!(ex.get_code(), 0);
    assert!(ex.get_previous().is_none());
    assert_eq!(ex.class, ExceptionClass::Exception);
}

#[test]
fn test_exception_with_code() {
    let ex = PhpException::with_code(ExceptionClass::RuntimeException, "Runtime error", 42);
    assert_eq!(ex.get_message(), "Runtime error");
    assert_eq!(ex.get_code(), 42);
}

#[test]
fn test_exception_with_previous() {
    let prev = PhpException::new(ExceptionClass::Exception, "Previous error");
    let ex = PhpException::with_previous(
        ExceptionClass::RuntimeException,
        "Current error",
        1,
        prev,
    );
    assert_eq!(ex.get_message(), "Current error");
    assert!(ex.get_previous().is_some());
    assert_eq!(ex.get_previous().unwrap().get_message(), "Previous error");
}

#[test]
fn test_exception_location() {
    let mut ex = PhpException::new(ExceptionClass::Exception, "Test");
    ex.set_location("test.php", 42);
    assert_eq!(ex.file, Some("test.php".to_string()));
    assert_eq!(ex.line, 42);
}

#[test]
fn test_exception_trace() {
    let mut ex = PhpException::new(ExceptionClass::Exception, "Test");
    ex.add_trace_frame(StackFrame {
        file: Some("test.php".to_string()),
        line: 10,
        function: Some("foo".to_string()),
        class: None,
        args: vec![],
    });
    ex.add_trace_frame(StackFrame {
        file: Some("test.php".to_string()),
        line: 20,
        function: Some("bar".to_string()),
        class: Some("MyClass".to_string()),
        args: vec![],
    });
    assert_eq!(ex.trace.len(), 2);
}

#[test]
fn test_exception_to_string() {
    let mut ex = PhpException::new(ExceptionClass::RuntimeException, "Something went wrong");
    ex.set_location("app.php", 15);
    let s = ex.to_string_repr();
    assert!(s.contains("RuntimeException"));
    assert!(s.contains("Something went wrong"));
    assert!(s.contains("app.php"));
    assert!(s.contains("15"));
}

#[test]
fn test_exception_class_hierarchy() {
    assert!(ExceptionClass::Exception.is_subclass_of(&ExceptionClass::Throwable));
    assert!(ExceptionClass::Error.is_subclass_of(&ExceptionClass::Throwable));
    assert!(ExceptionClass::RuntimeException.is_subclass_of(&ExceptionClass::Exception));
    assert!(ExceptionClass::TypeError.is_subclass_of(&ExceptionClass::Error));
    assert!(ExceptionClass::InvalidArgumentException.is_subclass_of(&ExceptionClass::LogicException));
    assert!(!ExceptionClass::Exception.is_subclass_of(&ExceptionClass::Error));
    assert!(!ExceptionClass::Error.is_subclass_of(&ExceptionClass::Exception));
}

#[test]
fn test_exception_class_name() {
    assert_eq!(ExceptionClass::Exception.name(), "Exception");
    assert_eq!(ExceptionClass::TypeError.name(), "TypeError");
    assert_eq!(ExceptionClass::Custom("MyException".to_string()).name(), "MyException");
}

#[test]
fn test_try_catch_block() {
    let mut block = TryCatchBlock::new(0);
    block.try_end = 10;
    block.catches.push(CatchBlock {
        exception_class: ExceptionClass::RuntimeException,
        variable_name: "$e".to_string(),
        body_start: 11,
        body_end: 20,
    });
    block.catches.push(CatchBlock {
        exception_class: ExceptionClass::Exception,
        variable_name: "$e".to_string(),
        body_start: 21,
        body_end: 30,
    });

    // RuntimeException should match the first catch
    let ex = PhpException::new(ExceptionClass::RuntimeException, "test");
    let catch = block.find_catch(&ex);
    assert!(catch.is_some());
    assert_eq!(catch.unwrap().body_start, 11);

    // TypeError should not match (it's an Error, not Exception)
    let ex2 = PhpException::new(ExceptionClass::TypeError, "test");
    let catch2 = block.find_catch(&ex2);
    assert!(catch2.is_none());
}

#[test]
fn test_exception_state() {
    let mut state = ExceptionState::new();
    assert!(!state.has_exception());
    assert_eq!(state.depth(), 0);

    // Push a try-catch block
    let mut block = TryCatchBlock::new(0);
    block.try_end = 10;
    block.catches.push(CatchBlock {
        exception_class: ExceptionClass::Exception,
        variable_name: "$e".to_string(),
        body_start: 11,
        body_end: 20,
    });
    state.push_try_catch(block);
    assert_eq!(state.depth(), 1);

    // Throw a matching exception
    let ex = PhpException::new(ExceptionClass::RuntimeException, "caught!");
    let action = state.throw(ex);
    match action {
        ExceptionAction::Catch { variable_name, jump_to, exception } => {
            assert_eq!(variable_name, "$e");
            assert_eq!(jump_to, 11);
            assert_eq!(exception.get_message(), "caught!");
        }
        ExceptionAction::Uncaught => panic!("Expected exception to be caught"),
    }

    // Throw an uncatchable exception (Error, not Exception)
    let ex2 = PhpException::new(ExceptionClass::TypeError, "uncaught!");
    let action2 = state.throw(ex2);
    assert!(matches!(action2, ExceptionAction::Uncaught));
    assert!(state.has_exception());

    // Clear exception
    state.clear_exception();
    assert!(!state.has_exception());
}

#[test]
fn test_exception_state_nested() {
    let mut state = ExceptionState::new();

    // Outer try-catch catches Exception
    let mut outer = TryCatchBlock::new(0);
    outer.try_end = 50;
    outer.catches.push(CatchBlock {
        exception_class: ExceptionClass::Exception,
        variable_name: "$outer_e".to_string(),
        body_start: 51,
        body_end: 60,
    });
    state.push_try_catch(outer);

    // Inner try-catch catches RuntimeException
    let mut inner = TryCatchBlock::new(10);
    inner.try_end = 30;
    inner.catches.push(CatchBlock {
        exception_class: ExceptionClass::RuntimeException,
        variable_name: "$inner_e".to_string(),
        body_start: 31,
        body_end: 40,
    });
    state.push_try_catch(inner);

    assert_eq!(state.depth(), 2);

    // RuntimeException should be caught by inner
    let ex = PhpException::new(ExceptionClass::RuntimeException, "inner catch");
    let action = state.throw(ex);
    match action {
        ExceptionAction::Catch { variable_name, .. } => {
            assert_eq!(variable_name, "$inner_e");
        }
        _ => panic!("Expected inner catch"),
    }

    // LogicException should be caught by outer (inner doesn't match)
    let ex2 = PhpException::new(ExceptionClass::LogicException, "outer catch");
    let action2 = state.throw(ex2);
    match action2 {
        ExceptionAction::Catch { variable_name, .. } => {
            assert_eq!(variable_name, "$outer_e");
        }
        _ => panic!("Expected outer catch"),
    }
}

#[test]
fn test_error_to_exception() {
    use crate::engine::errors::ErrorType;

    let ex = error_to_exception(ErrorType::Error, "Fatal error", Some("test.php"), 10);
    assert_eq!(ex.class, ExceptionClass::Error);
    assert_eq!(ex.get_message(), "Fatal error");
    assert_eq!(ex.file, Some("test.php".to_string()));
    assert_eq!(ex.line, 10);

    let ex2 = error_to_exception(ErrorType::Warning, "Warning msg", None, 0);
    assert_eq!(ex2.class, ExceptionClass::RuntimeException);
}

#[test]
fn test_try_catch_with_finally() {
    let mut block = TryCatchBlock::new(0);
    block.try_end = 10;
    block.catches.push(CatchBlock {
        exception_class: ExceptionClass::Exception,
        variable_name: "$e".to_string(),
        body_start: 11,
        body_end: 20,
    });
    block.finally_start = Some(21);
    block.finally_end = Some(30);

    assert!(block.finally_start.is_some());
    assert_eq!(block.finally_start.unwrap(), 21);
    assert_eq!(block.finally_end.unwrap(), 30);
}
