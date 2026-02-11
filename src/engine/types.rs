//! Core engine types
//!
//! This module contains the fundamental types used throughout the engine,
//! migrated from php_types.h

#[cfg(test)]
mod tests;

use std::sync::atomic::{AtomicU32, Ordering};

/// Result code for operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhpResult {
    Success = 0,
    Failure = -1,
}

impl From<i32> for PhpResult {
    fn from(value: i32) -> Self {
        match value {
            0 => PhpResult::Success,
            _ => PhpResult::Failure,
        }
    }
}

impl From<PhpResult> for i32 {
    fn from(value: PhpResult) -> Self {
        value as i32
    }
}

/// PHP value types
///
/// Note: In C, some values overlap (fake types and internal types both use 12-15).
/// We use separate enums to represent this distinction.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhpType {
    Undef = 0,
    Null = 1,
    False = 2,
    True = 3,
    Long = 4,
    Double = 5,
    String = 6,
    Array = 7,
    Object = 8,
    Resource = 9,
    Reference = 10,
    ConstantAst = 11,
    // Fake types for type hinting (values 12-17)
    Callable = 12,
    Iterable = 13,
    Void = 14,
    Static = 15,
    Mixed = 16,
    Never = 17,
    // Used for casts
    Bool = 18,
    Number = 19,
}

/// Internal types that overlap with fake types
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InternalType {
    Indirect = 12,
    Ptr = 13,
    AliasPtr = 14,
    Error = 15,
}

impl PhpType {
    /// Get the raw byte value
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// Create from raw byte value
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => PhpType::Undef,
            1 => PhpType::Null,
            2 => PhpType::False,
            3 => PhpType::True,
            4 => PhpType::Long,
            5 => PhpType::Double,
            6 => PhpType::String,
            7 => PhpType::Array,
            8 => PhpType::Object,
            9 => PhpType::Resource,
            10 => PhpType::Reference,
            11 => PhpType::ConstantAst,
            12 => PhpType::Callable, // Could also be Indirect
            13 => PhpType::Iterable, // Could also be Ptr
            14 => PhpType::Void,     // Could also be AliasPtr
            15 => PhpType::Static,   // Could also be Error
            16 => PhpType::Mixed,
            17 => PhpType::Never,
            18 => PhpType::Bool,
            19 => PhpType::Number,
            _ => PhpType::Undef,
        }
    }
}

/// Reference counted header
#[derive(Debug)]
pub struct RefcountedH {
    pub refcount: AtomicU32,
    pub type_info: AtomicU32,
}

impl Clone for RefcountedH {
    fn clone(&self) -> Self {
        Self {
            refcount: AtomicU32::new(self.refcount.load(Ordering::Relaxed)),
            type_info: AtomicU32::new(self.type_info.load(Ordering::Relaxed)),
        }
    }
}

impl RefcountedH {
    pub fn new(type_info: u32) -> Self {
        Self {
            refcount: AtomicU32::new(1),
            type_info: AtomicU32::new(type_info),
        }
    }

    pub fn refcount(&self) -> u32 {
        self.refcount.load(Ordering::Acquire)
    }

    pub fn addref(&self) -> u32 {
        self.refcount.fetch_add(1, Ordering::AcqRel) + 1
    }

    pub fn delref(&self) -> u32 {
        self.refcount.fetch_sub(1, Ordering::AcqRel) - 1
    }
}

/// Base reference counted structure
#[derive(Debug)]
pub struct Refcounted {
    pub gc: RefcountedH,
}

impl Refcounted {
    pub fn new(type_info: u32) -> Self {
        Self {
            gc: RefcountedH::new(type_info),
        }
    }
}

/// Value union - represents the actual value stored in a Val
#[derive(Debug)]
pub enum PhpValue {
    Long(i64),
    Double(f64),
    String(Box<PhpString>),
    Array(Box<PhpArray>),
    Object(Box<PhpObject>),
    Resource(Box<PhpResource>),
    Reference(Box<PhpReference>),
    Ast(Box<AstRef>),
    Val(Box<Val>),
    Ptr(*mut std::ffi::c_void),
    ClassEntry(*mut ClassEntry),
    Function(*mut PhpFunction),
}

// Safety: In PHP's execution model, these raw pointers are managed within
// a single-threaded context or with proper synchronization. We mark them
// as Send/Sync to allow use in Mutex, but actual thread safety must be
// ensured by the caller.
unsafe impl Send for PhpValue {}
unsafe impl Sync for PhpValue {}

/// Val - PHP value container
///
/// This is the fundamental type that represents any PHP value.
/// It contains the value itself and type information.
#[derive(Debug)]
pub struct Val {
    pub value: PhpValue,
    pub type_info: u32,
    pub u2: ValU2,
}

impl Val {
    pub fn new(value: PhpValue, php_type: PhpType) -> Self {
        Self {
            value,
            type_info: php_type as u32,
            u2: ValU2::default(),
        }
    }

    pub fn get_type(&self) -> PhpType {
        // Extract type from type_info (low 8 bits)
        match self.type_info & 0xff {
            0 => PhpType::Undef,
            1 => PhpType::Null,
            2 => PhpType::False,
            3 => PhpType::True,
            4 => PhpType::Long,
            5 => PhpType::Double,
            6 => PhpType::String,
            7 => PhpType::Array,
            8 => PhpType::Object,
            9 => PhpType::Resource,
            10 => PhpType::Reference,
            11 => PhpType::ConstantAst,
            _ => PhpType::Undef,
        }
    }
}

/// Val u2 union - stores various auxiliary data
#[derive(Debug, Default)]
pub struct ValU2 {
    pub next: u32,
    pub cache_slot: u32,
    pub opline_num: u32,
    pub lineno: u32,
    pub num_args: u32,
    pub fe_pos: u32,
    pub fe_iter_idx: u32,
    pub guard: u32,
    pub constant_flags: u32,
    pub extra: u32,
}

/// PHP String - reference counted string type
#[derive(Debug, Clone)]
pub struct PhpString {
    pub gc: RefcountedH,
    pub h: u64, // hash value
    pub len: usize,
    pub val: Vec<u8>, // null-terminated string data
}

impl PhpString {
    pub fn new(s: &str, persistent: bool) -> Self {
        let bytes = s.as_bytes();
        let len = bytes.len();
        let mut val = Vec::with_capacity(len + 1);
        val.extend_from_slice(bytes);
        val.push(0); // null terminator

        let type_info = if persistent {
            // GC_STRING | IS_STR_PERSISTENT
            0x00000006 | (1 << 10)
        } else {
            0x00000006 // GC_STRING
        };

        Self {
            gc: RefcountedH::new(type_info),
            h: 0, // hash will be computed on demand
            len,
            val,
        }
    }

    pub fn as_str(&self) -> &str {
        // Safety: we ensure val is valid UTF-8 and null-terminated
        unsafe {
            std::str::from_utf8_unchecked(std::slice::from_raw_parts(self.val.as_ptr(), self.len))
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.val[..self.len]
    }
}

/// Hash table bucket
#[derive(Debug)]
pub struct Bucket {
    pub val: Val,
    pub h: u64,                       // hash value or numeric index
    pub key: Option<Box<PhpString>>, // string key or None for numerics
}

/// Hash table / PHP Array
///
/// This is the core data structure for PHP arrays.
#[derive(Debug)]
pub struct PhpArray {
    pub gc: RefcountedH,
    pub flags: u32,
    pub n_table_mask: u32,
    pub ar_data: Vec<Bucket>, // array of hash buckets
    pub n_num_used: u32,
    pub n_num_of_elements: u32,
    pub n_table_size: u32,
    pub n_internal_pointer: u32,
    pub n_next_free_element: i64,
}

impl PhpArray {
    pub fn new() -> Self {
        Self {
            gc: RefcountedH::new(0x00000007), // GC_ARRAY
            flags: 0,
            n_table_mask: 0,
            ar_data: Vec::new(),
            n_num_used: 0,
            n_num_of_elements: 0,
            n_table_size: 0,
            n_internal_pointer: 0,
            n_next_free_element: 0,
        }
    }
}

impl Default for PhpArray {
    fn default() -> Self {
        Self::new()
    }
}

/// PHP Object
#[derive(Debug)]
pub struct PhpObject {
    pub gc: RefcountedH,
    pub handle: u32,
    pub class_name: String,
    pub properties: std::collections::HashMap<String, Val>,
}

impl PhpObject {
    pub fn new(class_name: &str) -> Self {
        Self {
            gc: RefcountedH { refcount: std::sync::atomic::AtomicU32::new(1), type_info: std::sync::atomic::AtomicU32::new(8) },
            handle: 0,
            class_name: class_name.to_string(),
            properties: std::collections::HashMap::new(),
        }
    }
}

// Safety: See PhpValue for explanation
unsafe impl Send for PhpObject {}
unsafe impl Sync for PhpObject {}

/// PHP Resource
#[derive(Debug)]
pub struct PhpResource {
    pub gc: RefcountedH,
    pub handle: i64,
    pub r#type: i32,
    pub ptr: *mut std::ffi::c_void,
}

/// PHP Reference
#[derive(Debug)]
pub struct PhpReference {
    pub gc: RefcountedH,
    pub val: Val,
}

/// AST Reference
#[derive(Debug)]
pub struct AstRef {
    pub gc: RefcountedH,
}

/// Class entry — defines a PHP class
#[derive(Debug)]
pub struct ClassEntry {
    pub name: String,
    pub parent_name: Option<String>,
    pub methods: std::collections::HashMap<String, ClassMethod>,
    pub default_properties: std::collections::HashMap<String, Val>,
    pub constants: std::collections::HashMap<String, Val>,
}

impl ClassEntry {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            parent_name: None,
            methods: std::collections::HashMap::new(),
            default_properties: std::collections::HashMap::new(),
            constants: std::collections::HashMap::new(),
        }
    }
}

/// A class method with its compiled opcodes
#[derive(Debug)]
pub struct ClassMethod {
    pub name: String,
    pub visibility: Visibility,
    pub is_static: bool,
    pub params: Vec<String>,
    pub op_array: crate::engine::vm::OpArray,
}

/// Visibility modifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Protected,
    Private,
}

/// Placeholder for object handlers (vtable)
#[derive(Debug)]
pub struct PhpObjectHandlers;

/// Placeholder for function type
#[derive(Debug)]
pub struct PhpFunction;
