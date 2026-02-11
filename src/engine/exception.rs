//! Exception handling
//!
//! Implements PHP exception handling (try-catch-finally)
//! Exception handling

#[cfg(test)]
mod tests;

use crate::engine::errors::ErrorType;

/// Exception class hierarchy
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExceptionClass {
    /// Base \Throwable interface
    Throwable,
    /// \Exception class
    Exception,
    /// \Error class
    Error,
    /// \RuntimeException
    RuntimeException,
    /// \LogicException
    LogicException,
    /// \InvalidArgumentException
    InvalidArgumentException,
    /// \TypeError
    TypeError,
    /// \ValueError
    ValueError,
    /// \DivisionByZeroError
    DivisionByZeroError,
    /// \OverflowException
    OverflowException,
    /// \UnderflowException
    UnderflowException,
    /// \OutOfRangeException
    OutOfRangeException,
    /// \OutOfBoundsException
    OutOfBoundsException,
    /// \BadMethodCallException
    BadMethodCallException,
    /// \BadFunctionCallException
    BadFunctionCallException,
    /// Custom exception class
    Custom(String),
}

impl ExceptionClass {
    /// Check if this class is a subclass of another
    pub fn is_subclass_of(&self, parent: &ExceptionClass) -> bool {
        match parent {
            ExceptionClass::Throwable => true, // Everything is Throwable
            ExceptionClass::Exception => matches!(
                self,
                ExceptionClass::Exception
                    | ExceptionClass::RuntimeException
                    | ExceptionClass::LogicException
                    | ExceptionClass::InvalidArgumentException
                    | ExceptionClass::OverflowException
                    | ExceptionClass::UnderflowException
                    | ExceptionClass::OutOfRangeException
                    | ExceptionClass::OutOfBoundsException
                    | ExceptionClass::BadMethodCallException
                    | ExceptionClass::BadFunctionCallException
                    | ExceptionClass::Custom(_)
            ),
            ExceptionClass::Error => matches!(
                self,
                ExceptionClass::Error
                    | ExceptionClass::TypeError
                    | ExceptionClass::ValueError
                    | ExceptionClass::DivisionByZeroError
            ),
            ExceptionClass::RuntimeException => matches!(
                self,
                ExceptionClass::RuntimeException
                    | ExceptionClass::OverflowException
                    | ExceptionClass::UnderflowException
                    | ExceptionClass::OutOfRangeException
            ),
            ExceptionClass::LogicException => matches!(
                self,
                ExceptionClass::LogicException
                    | ExceptionClass::InvalidArgumentException
                    | ExceptionClass::OutOfBoundsException
                    | ExceptionClass::BadMethodCallException
                    | ExceptionClass::BadFunctionCallException
            ),
            _ => self == parent,
        }
    }

    /// Get the class name as a string
    pub fn name(&self) -> &str {
        match self {
            ExceptionClass::Throwable => "Throwable",
            ExceptionClass::Exception => "Exception",
            ExceptionClass::Error => "Error",
            ExceptionClass::RuntimeException => "RuntimeException",
            ExceptionClass::LogicException => "LogicException",
            ExceptionClass::InvalidArgumentException => "InvalidArgumentException",
            ExceptionClass::TypeError => "TypeError",
            ExceptionClass::ValueError => "ValueError",
            ExceptionClass::DivisionByZeroError => "DivisionByZeroError",
            ExceptionClass::OverflowException => "OverflowException",
            ExceptionClass::UnderflowException => "UnderflowException",
            ExceptionClass::OutOfRangeException => "OutOfRangeException",
            ExceptionClass::OutOfBoundsException => "OutOfBoundsException",
            ExceptionClass::BadMethodCallException => "BadMethodCallException",
            ExceptionClass::BadFunctionCallException => "BadFunctionCallException",
            ExceptionClass::Custom(name) => name.as_str(),
        }
    }
}

/// PHP Exception object
#[derive(Debug, Clone)]
pub struct PhpException {
    /// Exception class
    pub class: ExceptionClass,
    /// Exception message
    pub message: String,
    /// Exception code
    pub code: i64,
    /// File where exception was thrown
    pub file: Option<String>,
    /// Line where exception was thrown
    pub line: u32,
    /// Previous exception (chained exceptions)
    pub previous: Option<Box<PhpException>>,
    /// Stack trace
    pub trace: Vec<StackFrame>,
}

/// Stack frame for exception traces
#[derive(Debug, Clone)]
pub struct StackFrame {
    pub file: Option<String>,
    pub line: u32,
    pub function: Option<String>,
    pub class: Option<String>,
    pub args: Vec<String>,
}

impl PhpException {
    /// Create a new exception
    pub fn new(class: ExceptionClass, message: &str) -> Self {
        Self {
            class,
            message: message.to_string(),
            code: 0,
            file: None,
            line: 0,
            previous: None,
            trace: Vec::new(),
        }
    }

    /// Create a new exception with code
    pub fn with_code(class: ExceptionClass, message: &str, code: i64) -> Self {
        Self {
            class,
            message: message.to_string(),
            code,
            file: None,
            line: 0,
            previous: None,
            trace: Vec::new(),
        }
    }

    /// Create a new exception with previous exception
    pub fn with_previous(
        class: ExceptionClass,
        message: &str,
        code: i64,
        previous: PhpException,
    ) -> Self {
        Self {
            class,
            message: message.to_string(),
            code,
            file: None,
            line: 0,
            previous: Some(Box::new(previous)),
            trace: Vec::new(),
        }
    }

    /// Set the file and line where the exception was thrown
    pub fn set_location(&mut self, file: &str, line: u32) {
        self.file = Some(file.to_string());
        self.line = line;
    }

    /// Add a stack frame to the trace
    pub fn add_trace_frame(&mut self, frame: StackFrame) {
        self.trace.push(frame);
    }

    /// Get the exception message
    pub fn get_message(&self) -> &str {
        &self.message
    }

    /// Get the exception code
    pub fn get_code(&self) -> i64 {
        self.code
    }

    /// Get the previous exception
    pub fn get_previous(&self) -> Option<&PhpException> {
        self.previous.as_deref()
    }

    /// Format the exception as a string (like PHP's __toString)
    pub fn to_string_repr(&self) -> String {
        let mut result = format!(
            "{}: {} in {}:{}",
            self.class.name(),
            self.message,
            self.file.as_deref().unwrap_or("unknown"),
            self.line
        );

        if !self.trace.is_empty() {
            result.push_str("\nStack trace:");
            for (i, frame) in self.trace.iter().enumerate() {
                result.push_str(&format!(
                    "\n#{} {}({}): {}{}",
                    i,
                    frame.file.as_deref().unwrap_or("unknown"),
                    frame.line,
                    if let Some(ref class) = frame.class {
                        format!("{}::", class)
                    } else {
                        String::new()
                    },
                    frame.function.as_deref().unwrap_or("{main}"),
                ));
            }
        }

        if let Some(ref prev) = self.previous {
            result.push_str(&format!("\n\nCaused by:\n{}", prev.to_string_repr()));
        }

        result
    }
}

/// Catch block definition
#[derive(Debug, Clone)]
pub struct CatchBlock {
    /// Exception class to catch
    pub exception_class: ExceptionClass,
    /// Variable name to bind the exception to
    pub variable_name: String,
    /// Opcode index where the catch body starts
    pub body_start: usize,
    /// Opcode index where the catch body ends
    pub body_end: usize,
}

/// Try-catch-finally block definition
#[derive(Debug, Clone)]
pub struct TryCatchBlock {
    /// Opcode index where the try body starts
    pub try_start: usize,
    /// Opcode index where the try body ends
    pub try_end: usize,
    /// Catch blocks
    pub catches: Vec<CatchBlock>,
    /// Finally block start (None if no finally)
    pub finally_start: Option<usize>,
    /// Finally block end (None if no finally)
    pub finally_end: Option<usize>,
}

impl TryCatchBlock {
    pub fn new(try_start: usize) -> Self {
        Self {
            try_start,
            try_end: 0,
            catches: Vec::new(),
            finally_start: None,
            finally_end: None,
        }
    }

    /// Find the matching catch block for an exception
    pub fn find_catch(&self, exception: &PhpException) -> Option<&CatchBlock> {
        self.catches
            .iter()
            .find(|c| exception.class.is_subclass_of(&c.exception_class))
    }
}

/// Exception handler state for the VM
pub struct ExceptionState {
    /// Stack of try-catch blocks (for nested try-catch)
    try_catch_stack: Vec<TryCatchBlock>,
    /// Current unhandled exception
    current_exception: Option<PhpException>,
}

impl ExceptionState {
    pub fn new() -> Self {
        Self {
            try_catch_stack: Vec::new(),
            current_exception: None,
        }
    }

    /// Push a new try-catch block onto the stack
    pub fn push_try_catch(&mut self, block: TryCatchBlock) {
        self.try_catch_stack.push(block);
    }

    /// Pop the current try-catch block
    pub fn pop_try_catch(&mut self) -> Option<TryCatchBlock> {
        self.try_catch_stack.pop()
    }

    /// Get the current try-catch block
    pub fn current_try_catch(&self) -> Option<&TryCatchBlock> {
        self.try_catch_stack.last()
    }

    /// Throw an exception
    pub fn throw(&mut self, exception: PhpException) -> ExceptionAction {
        // Search for a matching catch block from innermost to outermost
        for block in self.try_catch_stack.iter().rev() {
            if let Some(catch) = block.find_catch(&exception) {
                let action = ExceptionAction::Catch {
                    variable_name: catch.variable_name.clone(),
                    jump_to: catch.body_start,
                    exception: exception.clone(),
                };
                return action;
            }
        }

        // No catch block found - store as unhandled
        self.current_exception = Some(exception);
        ExceptionAction::Uncaught
    }

    /// Get the current unhandled exception
    pub fn get_current_exception(&self) -> Option<&PhpException> {
        self.current_exception.as_ref()
    }

    /// Clear the current exception
    pub fn clear_exception(&mut self) {
        self.current_exception = None;
    }

    /// Check if there's an active exception
    pub fn has_exception(&self) -> bool {
        self.current_exception.is_some()
    }

    /// Get the nesting depth of try-catch blocks
    pub fn depth(&self) -> usize {
        self.try_catch_stack.len()
    }
}

impl Default for ExceptionState {
    fn default() -> Self {
        Self::new()
    }
}

/// Action to take when an exception is thrown
#[derive(Debug, Clone)]
pub enum ExceptionAction {
    /// Exception was caught - jump to catch block
    Catch {
        variable_name: String,
        jump_to: usize,
        exception: PhpException,
    },
    /// No matching catch block - exception is uncaught
    Uncaught,
}

/// Convert a PhpError to a PhpException
pub fn error_to_exception(
    error_type: ErrorType,
    message: &str,
    file: Option<&str>,
    line: u32,
) -> PhpException {
    let class = match error_type {
        ErrorType::Error | ErrorType::CoreError | ErrorType::CompileError => {
            ExceptionClass::Error
        }
        ErrorType::Warning
        | ErrorType::CoreWarning
        | ErrorType::CompileWarning
        | ErrorType::UserWarning => ExceptionClass::RuntimeException,
        ErrorType::RecoverableError => ExceptionClass::Error,
        _ => ExceptionClass::Exception,
    };

    let mut exception = PhpException::new(class, message);
    if let Some(f) = file {
        exception.set_location(f, line);
    }
    exception
}
