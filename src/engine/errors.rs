//! Error handling
//!
//! Migrated from php_errors.h and php_errors.c

/// Error types
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorType {
    Error = 1,
    Warning = 2,
    Parse = 4,
    Notice = 8,
    CoreError = 16,
    CoreWarning = 32,
    CompileError = 64,
    CompileWarning = 128,
    UserError = 256,
    UserWarning = 512,
    UserNotice = 1024,
    Strict = 2048,
    RecoverableError = 4096,
    Deprecated = 8192,
    UserDeprecated = 16384,
}

/// Error information
#[derive(Debug, Clone)]
pub struct PhpError {
    pub r#type: ErrorType,
    pub message: String,
    pub filename: Option<String>,
    pub lineno: u32,
}

/// Error handler function type
pub type ErrorHandler = fn(error: &PhpError);

static ERROR_HANDLER: std::sync::Mutex<Option<ErrorHandler>> = std::sync::Mutex::new(None);

/// Set error handler
pub fn set_error_handler(handler: ErrorHandler) {
    if let Ok(mut guard) = ERROR_HANDLER.lock() {
        *guard = Some(handler);
    }
}

/// Report an error
pub fn php_error(r#type: ErrorType, message: &str) {
    let error = PhpError {
        r#type,
        message: message.to_string(),
        filename: None,
        lineno: 0,
    };

    let handler = ERROR_HANDLER.lock().ok().and_then(|g| *g);
    if let Some(handler) = handler {
        handler(&error);
    } else {
        // Default error handling
        eprintln!("PHP {}: {}", error_type_name(r#type), message);
    }
}

/// Report an error with file and line
pub fn error_at_line(r#type: ErrorType, filename: &str, line: u32, message: &str) {
    let error = PhpError {
        r#type,
        message: message.to_string(),
        filename: Some(filename.to_string()),
        lineno: line,
    };

    let handler = ERROR_HANDLER.lock().ok().and_then(|g| *g);
    if let Some(handler) = handler {
        handler(&error);
    } else {
        // Default error handling
        if let Some(ref fname) = error.filename {
            eprintln!(
                "PHP {}: {} in {} on line {}",
                error_type_name(r#type),
                message,
                fname,
                line
            );
        } else {
            eprintln!("PHP {}: {}", error_type_name(r#type), message);
        }
    }
}

/// Get error type name
fn error_type_name(r#type: ErrorType) -> &'static str {
    match r#type {
        ErrorType::Error => "Error",
        ErrorType::Warning => "Warning",
        ErrorType::Parse => "Parse error",
        ErrorType::Notice => "Notice",
        ErrorType::CoreError => "Core Error",
        ErrorType::CoreWarning => "Core Warning",
        ErrorType::CompileError => "Compile error",
        ErrorType::CompileWarning => "Compile warning",
        ErrorType::UserError => "User Error",
        ErrorType::UserWarning => "User Warning",
        ErrorType::UserNotice => "User Notice",
        ErrorType::Strict => "Strict Standards",
        ErrorType::RecoverableError => "Recoverable error",
        ErrorType::Deprecated => "Deprecated",
        ErrorType::UserDeprecated => "User Deprecated",
    }
}

/// Trigger error (PHP function equivalent)
pub fn php_trigger_error(message: &str, error_type: ErrorType) {
    php_error(error_type, message);
}
