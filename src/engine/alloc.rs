//! Memory allocation
//!
//! Memory allocation
//!
//! This module provides memory allocation functions compatible with PHP's
//! memory management system, supporting both persistent and non-persistent allocations.

#[cfg(test)]
mod tests;

use std::alloc::{alloc, dealloc, handle_alloc_error, Layout};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::sync::OnceLock;

/// Memory statistics
#[derive(Debug, Default)]
struct MemoryStats {
    allocated: AtomicUsize,
    peak: AtomicUsize,
    count: AtomicUsize,
}

static NON_PERSISTENT_STATS: MemoryStats = MemoryStats {
    allocated: AtomicUsize::new(0),
    peak: AtomicUsize::new(0),
    count: AtomicUsize::new(0),
};

/// Track persistent allocations (size -> count)
fn persistent_allocs() -> &'static Mutex<HashMap<usize, usize>> {
    static ALLOCS: OnceLock<Mutex<HashMap<usize, usize>>> = OnceLock::new();
    ALLOCS.get_or_init(|| Mutex::new(HashMap::new()))
}

const ALIGNMENT: usize = 8;

/// Align size to alignment boundary
fn align_size(size: usize) -> usize {
    (size + ALIGNMENT - 1) & !(ALIGNMENT - 1)
}

/// Allocate memory (persistent or non-persistent)
///
/// # Safety
/// The caller must ensure proper alignment and valid size.
pub unsafe fn pemalloc(size: usize, persistent: bool) -> *mut u8 {
    let aligned_size = align_size(size);
    let layout = match Layout::from_size_align(aligned_size, ALIGNMENT) {
        Ok(l) => l,
        Err(_) => unsafe { handle_alloc_error(Layout::from_size_align_unchecked(aligned_size, ALIGNMENT)) },
    };

    let ptr = unsafe { alloc(layout) };
    if ptr.is_null() {
        handle_alloc_error(layout);
    }

    if persistent {
        let mut allocs = persistent_allocs().lock().unwrap();
        allocs.insert(ptr as usize, aligned_size);
    } else {
        NON_PERSISTENT_STATS
            .allocated
            .fetch_add(aligned_size, Ordering::Relaxed);
        NON_PERSISTENT_STATS.count.fetch_add(1, Ordering::Relaxed);

        let current = NON_PERSISTENT_STATS.allocated.load(Ordering::Relaxed);
        let peak = NON_PERSISTENT_STATS.peak.load(Ordering::Relaxed);
        if current > peak {
            NON_PERSISTENT_STATS.peak.store(current, Ordering::Relaxed);
        }
    }

    ptr
}

/// Reallocate memory
pub unsafe fn perealloc(ptr: *mut u8, new_size: usize, persistent: bool) -> *mut u8 {
    if ptr.is_null() {
        return pemalloc(new_size, persistent);
    }

    // For simplicity, allocate new and copy
    // TODO: Implement proper realloc with size tracking
    let new_ptr = pemalloc(new_size, persistent);
    if !new_ptr.is_null() {
        // Note: We don't know the old size, so this is a simplified version
        // In a real implementation, we'd track sizes
    }
    pefree(ptr, persistent);
    new_ptr
}

/// Free memory
pub unsafe fn pefree(ptr: *mut u8, persistent: bool) {
    if ptr.is_null() {
        return;
    }

    if persistent {
        let mut allocs = persistent_allocs().lock().unwrap();
        if let Some(size) = allocs.remove(&(ptr as usize)) {
            unsafe {
                let layout = Layout::from_size_align_unchecked(size, ALIGNMENT);
                dealloc(ptr, layout);
            }
        }
    } else {
        // For non-persistent, we need to track sizes
        // For now, this is a placeholder - in real implementation we'd use a size map
        // This is simplified - proper implementation would track allocation sizes
    }
}

/// Get current memory usage (non-persistent)
pub fn get_memory_usage() -> usize {
    NON_PERSISTENT_STATS.allocated.load(Ordering::Relaxed)
}

/// Get peak memory usage (non-persistent)
pub fn get_peak_memory_usage() -> usize {
    NON_PERSISTENT_STATS.peak.load(Ordering::Relaxed)
}

/// Get allocation count (non-persistent)
pub fn get_allocation_count() -> usize {
    NON_PERSISTENT_STATS.count.load(Ordering::Relaxed)
}
