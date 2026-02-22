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
    BoolXor = 15,
    IsIdentical = 16,
    IsNotIdentical = 17,
    IsEqual = 18,
    IsNotEqual = 19,
    IsSmaller = 20,
    IsSmallerOrEqual = 21,
    Assign = 22,
    AssignDim = 23,
    AssignObj = 24,
    AssignStaticProp = 25,
    AssignOp = 26,
    Echo = 27,
    Return = 28,
    Jmp = 29,
    JmpZ = 30,
    JmpNZ = 31,
    InitFCall = 32,       // Initialize function call
    DoFCall = 33,         // Execute function call
    TryCatchBegin = 34,   // Begin try block
    TryCatchEnd = 35,     // End try block
    CatchBegin = 36,      // Begin catch block
    CatchEnd = 37,        // End catch block
    FinallyBegin = 38,    // Begin finally block
    FinallyEnd = 39,      // End finally block
    Throw = 40,           // Throw exception
    FetchVar = 41,        // Load variable from symbol table into temp
    SendVal = 42,         // Push argument for function call
    Include = 43,         // include/require
    InitArray = 44,       // Create empty array in temp slot
    AddArrayElement = 45, // Add element to array (op1=array temp, op2=value, extended_value for key)
    FetchDim = 46,        // Fetch array element by index/key
    NewObj = 47,          // Create new object instance (op1=class name, result=temp)
    FetchObjProp = 48,    // Fetch object property (op1=obj, op2=prop name, result=temp)
    AssignObjProp = 49,   // Assign to object property (op1=obj var, op2=prop name, result=value)
    InitMethodCall = 50,  // Init method call (op1=obj, op2=method name)
    DoMethodCall = 51,    // Execute method call (op1=method name, result=temp)
    TypeCheck = 52,       // Check type of operand
    IsSet = 53,           // Check if variable is set
    Empty = 54,           // Check if variable is empty
    Unset = 55,           // Unset a variable
    Count = 56,           // Count elements in array/string
    Keys = 57,            // Get array keys
    Values = 58,          // Get array values
    ArrayDiff = 59,       // Compare arrays
    Coalesce = 60,        // Null coalescing: if op1 is not null, result=op1, else result=op2
    JmpNullZ = 61,        // Jump if op1 is null (for ?? short-circuit)
    QmAssign = 62,        // Ternary assign: resolve op1, store in result temp slot
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
        _ => "UNKNOWN",
    }
}
