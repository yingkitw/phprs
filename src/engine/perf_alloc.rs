//! High-performance memory management
//!
//! This module implements optimized memory allocation and garbage collection
//! to outperform PHP 8's memory management.

use std::alloc::{alloc, dealloc, Layout};
use std::ptr::NonNull;
use std::sync::atomic::{AtomicUsize, Ordering};

/// High-performance memory pool for small allocations
#[derive(Debug)]
pub struct MemoryPool {
    pools: [Vec<NonNull<u8>>; 8],
    sizes: [usize; 8],
    total_allocated: AtomicUsize,
}

impl MemoryPool {
    pub fn new() -> Self {
        Self {
            pools: [
                Vec::new(), // 8 bytes
                Vec::new(), // 16 bytes
                Vec::new(), // 32 bytes
                Vec::new(), // 64 bytes
                Vec::new(), // 128 bytes
                Vec::new(), // 256 bytes
                Vec::new(), // 512 bytes
                Vec::new(), // 1024 bytes
            ],
            sizes: [8, 16, 32, 64, 128, 256, 512, 1024],
            total_allocated: AtomicUsize::new(0),
        }
    }

    #[inline]
    fn get_pool_index(size: usize) -> Option<usize> {
        if size <= 8 {
            Some(0)
        } else if size <= 16 {
            Some(1)
        } else if size <= 32 {
            Some(2)
        } else if size <= 64 {
            Some(3)
        } else if size <= 128 {
            Some(4)
        } else if size <= 256 {
            Some(5)
        } else if size <= 512 {
            Some(6)
        } else if size <= 1024 {
            Some(7)
        } else {
            None
        }
    }

    pub fn allocate(&mut self, size: usize) -> Option<NonNull<u8>> {
        if let Some(pool_idx) = Self::get_pool_index(size) {
            // Try to reuse from pool
            if let Some(ptr) = self.pools[pool_idx].pop() {
                return Some(ptr);
            }
        }

        // Allocate new memory
        let layout = Layout::from_size_align(size, 8).ok()?;
        unsafe {
            let ptr = alloc(layout);
            if !ptr.is_null() {
                self.total_allocated.fetch_add(size, Ordering::Relaxed);
                NonNull::new(ptr)
            } else {
                None
            }
        }
    }

    pub fn deallocate(&mut self, ptr: NonNull<u8>, size: usize) {
        if let Some(pool_idx) = Self::get_pool_index(size) {
            // Return to pool if under limit
            if self.pools[pool_idx].len() < 100 {
                self.pools[pool_idx].push(ptr);
                self.total_allocated.fetch_sub(size, Ordering::Relaxed);
                return;
            }
        }

        // Free directly
        let layout = Layout::from_size_align(size, 8).unwrap();
        unsafe {
            dealloc(ptr.as_ptr(), layout);
            self.total_allocated.fetch_sub(size, Ordering::Relaxed);
        }
    }

    pub fn total_allocated(&self) -> usize {
        self.total_allocated.load(Ordering::Relaxed)
    }

    pub fn clear_pools(&mut self) {
        for pool in &mut self.pools {
            for ptr in pool.drain(..) {
                let size = unsafe { std::mem::transmute::<_, usize>(ptr) };
                let layout = Layout::from_size_align(size, 8).unwrap();
                unsafe { dealloc(ptr.as_ptr(), layout) };
            }
        }
    }
}

impl Default for MemoryPool {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-local memory pool for fast allocations
thread_local! {
    static MEMORY_POOL: std::cell::RefCell<MemoryPool> = std::cell::RefCell::new(MemoryPool::new());
}

/// High-performance string builder with pre-allocation
#[derive(Debug)]
pub struct StringBuilder {
    buffer: Vec<u8>,
    capacity: usize,
}

impl StringBuilder {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
            capacity,
        }
    }

    #[inline]
    pub fn push_str(&mut self, s: &str) {
        self.buffer.extend_from_slice(s.as_bytes());
    }

    #[inline]
    pub fn push(&mut self, ch: char) {
        self.buffer
            .extend_from_slice(ch.encode_utf8(&mut [0; 4]).as_bytes());
    }

    #[inline]
    pub fn extend_from_slice(&mut self, bytes: &[u8]) {
        self.buffer.extend_from_slice(bytes);
    }

    pub fn into_string(self) -> String {
        unsafe { String::from_utf8_unchecked(self.buffer) }
    }

    pub fn as_str(&self) -> &str {
        unsafe { std::str::from_utf8_unchecked(&self.buffer) }
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

impl Default for StringBuilder {
    fn default() -> Self {
        Self::with_capacity(64)
    }
}

/// Optimized PHP string allocator
pub fn allocate_php_string(content: &str, persistent: bool) -> crate::engine::types::PhpString {
    let len = content.len();
    let mut val = Vec::with_capacity(len + 1);
    val.extend_from_slice(content.as_bytes());
    val.push(0); // null terminator

    let type_info = if persistent {
        // GC_STRING | IS_STR_PERSISTENT
        0x00000006 | (1 << 10)
    } else {
        0x00000006 // GC_STRING
    };

    crate::engine::types::PhpString {
        gc: crate::engine::types::RefcountedH::new(type_info),
        h: 0, // hash will be computed on demand
        len,
        val,
    }
}

/// Optimized string concatenation that minimizes allocations
pub fn fast_concat(str1: &str, str2: &str) -> crate::engine::types::PhpString {
    let total_len = str1.len() + str2.len();
    let mut val = Vec::with_capacity(total_len + 1);
    val.extend_from_slice(str1.as_bytes());
    val.extend_from_slice(str2.as_bytes());
    val.push(0); // null terminator

    crate::engine::types::PhpString {
        gc: crate::engine::types::RefcountedH::new(0x00000006), // GC_STRING
        h: 0,
        len: total_len,
        val,
    }
}

/// Memory usage statistics
#[derive(Debug, Default)]
pub struct MemoryStats {
    pub total_allocations: AtomicUsize,
    pub total_deallocations: AtomicUsize,
    pub peak_memory_usage: AtomicUsize,
    pub current_memory_usage: AtomicUsize,
}

impl MemoryStats {
    pub fn record_allocation(&self, size: usize) {
        self.total_allocations.fetch_add(1, Ordering::Relaxed);
        let current = self.current_memory_usage.fetch_add(size, Ordering::Relaxed) + size;

        // Update peak if necessary
        let mut peak = self.peak_memory_usage.load(Ordering::Relaxed);
        while current > peak {
            match self.peak_memory_usage.compare_exchange_weak(
                peak,
                current,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => peak = x,
            }
        }
    }

    pub fn record_deallocation(&self, size: usize) {
        self.total_deallocations.fetch_add(1, Ordering::Relaxed);
        self.current_memory_usage.fetch_sub(size, Ordering::Relaxed);
    }

    pub fn get_stats(&self) -> (usize, usize, usize, usize) {
        (
            self.total_allocations.load(Ordering::Relaxed),
            self.total_deallocations.load(Ordering::Relaxed),
            self.peak_memory_usage.load(Ordering::Relaxed),
            self.current_memory_usage.load(Ordering::Relaxed),
        )
    }
}

/// Global memory statistics
static MEMORY_STATS: std::sync::LazyLock<MemoryStats> =
    std::sync::LazyLock::new(MemoryStats::default);

pub fn get_memory_stats() -> &'static MemoryStats {
    &MEMORY_STATS
}

/// Zero-copy string reference for performance
#[derive(Debug, Clone, Copy)]
pub struct StrRef<'a> {
    pub data: &'a [u8],
    pub len: usize,
}

impl<'a> StrRef<'a> {
    #[inline]
    pub fn from_str(s: &'a str) -> Self {
        Self {
            data: s.as_bytes(),
            len: s.len(),
        }
    }

    #[inline]
    pub fn as_str(&self) -> &'a str {
        unsafe { std::str::from_utf8_unchecked(&self.data[..self.len]) }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }
}

/// Fast hash function optimized for short strings
#[inline]
pub fn fast_hash(data: &[u8]) -> u64 {
    let mut hash = 5381u64;
    for &byte in data {
        hash = hash.wrapping_mul(33).wrapping_add(byte as u64);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_pool() {
        let mut pool = MemoryPool::new();

        // Test small allocation
        let ptr1 = pool.allocate(32).unwrap();
        let ptr2 = pool.allocate(32).unwrap();

        pool.deallocate(ptr1, 32);
        pool.deallocate(ptr2, 32);

        assert_eq!(pool.total_allocated(), 0);
    }

    #[test]
    fn test_string_builder() {
        let mut builder = StringBuilder::with_capacity(10);
        builder.push_str("Hello");
        builder.push(' ');
        builder.push_str("World");

        assert_eq!(builder.as_str(), "Hello World");
        assert_eq!(builder.len(), 11);
    }

    #[test]
    fn test_fast_concat() {
        let result = fast_concat("Hello", " World");
        assert_eq!(result.as_str(), "Hello World");
        assert_eq!(result.len, 11);
    }

    #[test]
    fn test_str_ref() {
        let s = "Hello World";
        let r = StrRef::from_str(s);

        assert_eq!(r.as_str(), s);
        assert_eq!(r.len(), s.len());
        assert!(!r.is_empty());
    }

    #[test]
    fn test_fast_hash() {
        let s1 = b"Hello";
        let s2 = b"Hello";
        let s3 = b"World";

        assert_eq!(fast_hash(s1), fast_hash(s2));
        assert_ne!(fast_hash(s1), fast_hash(s3));
    }
}
