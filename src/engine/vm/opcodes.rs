//! Opcode definitions and compiled op array structures

use crate::engine::types::Val;

/// Opcode types
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opcode {
    Nop = 0,
    Add = 1,
    Sub = 2,
    Mul = 3,
    Div = 4,
    Mod = 5,
    Sl = 6, // Shift left
    Sr = 7, // Shift right
    Concat = 8,
    BwOr = 9,
    BwAnd = 10,
    BwXor = 11,
    Pow = 12,
    BwNot = 13,
    BoolNot = 14,
    BoolAnd = 15,
    BoolOr = 16,
    BoolXor = 17,
    IsIdentical = 18,
    IsNotIdentical = 19,
    IsEqual = 20,
    IsNotEqual = 21,
    IsSmaller = 22,
    IsSmallerOrEqual = 23,
    Assign = 24,
    AssignDim = 25,
    AssignObj = 26,
    AssignStaticProp = 27,
    AssignOp = 28,
    Echo = 29,
    Return = 30,
    Jmp = 31,
    JmpZ = 32,
    JmpNZ = 33,
    InitFCall = 34,       // Initialize function call
    DoFCall = 35,         // Execute function call
    TryCatchBegin = 36,   // Begin try block
    TryCatchEnd = 37,     // End try block
    CatchBegin = 38,      // Begin catch block
    CatchEnd = 39,        // End catch block
    FinallyBegin = 40,    // Begin finally block
    FinallyEnd = 41,      // End finally block
    Throw = 42,           // Throw exception
    FetchVar = 43,        // Load variable from symbol table into temp
    SendVal = 44,         // Push argument for function call
    Include = 45,         // include/require
    InitArray = 46,       // Create empty array in temp slot
    AddArrayElement = 47, // Add element to array (op1=array temp, op2=value, extended_value for key)
    FetchDim = 48,        // Fetch array element by index/key
    NewObj = 49,          // Create new object instance (op1=class name, result=temp)
    FetchObjProp = 50,    // Fetch object property (op1=obj, op2=prop name, result=temp)
    AssignObjProp = 51,   // Assign to object property (op1=obj var, op2=prop name, result=value)
    InitMethodCall = 52,  // Init method call (op1=obj, op2=method name)
    DoMethodCall = 53,    // Execute method call (op1=method name, result=temp)
    TypeCheck = 54,       // Check type of operand
    IsSet = 55,           // Check if variable is set
    Empty = 56,           // Check if variable is empty
    Unset = 57,           // Unset a variable
    Count = 58,           // Count elements in array/string
    Keys = 59,            // Get array keys
    Values = 60,          // Get array values
    ArrayDiff = 61,       // Compare arrays
    Coalesce = 62,        // Null coalescing: if op1 is not null, result=op1, else result=op2
    JmpNullZ = 63,        // Jump if op1 is null (for ?? short-circuit)
    QmAssign = 64,        // Ternary assign: resolve op1, store in result temp slot
    FeReset = 65,         // Reset array iterator for foreach
    FeFetch = 66,         // Fetch next element from array iterator
}

/// Operation structure
#[derive(Debug, Clone)]
pub struct Op {
    pub opcode: Opcode,
    pub op1: Val,
    pub op2: Val,
    pub result: Val,
    pub extended_value: u32,
}

impl Op {
    pub fn new(opcode: Opcode, op1: Val, op2: Val, result: Val, extended_value: u32) -> Self {
        Self {
            opcode,
            op1,
            op2,
            result,
            extended_value,
        }
    }
}

/// Op array (compiled script)
#[derive(Debug)]
pub struct OpArray {
    pub ops: Vec<Op>,
    pub vars: Vec<Val>, // Variables
    pub filename: Option<String>,
    pub line_start: u32,
    pub line_end: u32,
    pub function_name: Option<String>,
    pub class_table: std::collections::HashMap<String, crate::engine::types::ClassEntry>,
    pub variadic_param: Option<String>,
}

impl OpArray {
    pub fn new(filename: String) -> Self {
        Self {
            ops: Vec::new(),
            vars: Vec::new(),
            filename: Some(filename),
            line_start: 0,
            line_end: 0,
            class_table: std::collections::HashMap::new(),
            function_name: None,
            variadic_param: None,
        }
    }

    /// Create OpArray with pre-allocated capacity for performance
    pub fn with_capacity(capacity: usize, filename: String) -> Self {
        Self {
            ops: Vec::with_capacity(capacity),
            vars: Vec::new(),
            filename: Some(filename),
            line_start: 0,
            line_end: 0,
            class_table: std::collections::HashMap::new(),
            function_name: None,
            variadic_param: None,
        }
    }

    #[inline]
    pub fn add_op(&mut self, op: Op) {
        self.ops.push(op);
    }
}

/// Get opcode name for debugging/display
pub fn get_opcode_name(opcode: Opcode) -> &'static str {
    match opcode {
        Opcode::Nop => "NOP",
        Opcode::Add => "ADD",
        Opcode::Sub => "SUB",
        Opcode::Mul => "MUL",
        Opcode::Div => "DIV",
        Opcode::Mod => "MOD",
        Opcode::Concat => "CONCAT",
        Opcode::Assign => "ASSIGN",
        Opcode::InitFCall => "INIT_FCALL",
        Opcode::DoFCall => "DO_FCALL",
        Opcode::IsSet => "ISSET",
        Opcode::Empty => "EMPTY",
        Opcode::Unset => "UNSET",
        Opcode::Count => "COUNT",
        Opcode::Keys => "KEYS",
        Opcode::Values => "VALUES",
        Opcode::ArrayDiff => "ARRAY_DIFF",
        Opcode::Coalesce => "COALESCE",
        Opcode::JmpNullZ => "JMP_NULLZ",
        Opcode::BoolAnd => "BOOL_AND",
        Opcode::BoolOr => "BOOL_OR",
        Opcode::FeReset => "FE_RESET",
        Opcode::FeFetch => "FE_FETCH",
        _ => "UNKNOWN",
    }
}
