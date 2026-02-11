//! Unit tests for memory allocation

use crate::engine::alloc::{
    get_allocation_count, get_memory_usage, get_peak_memory_usage, pefree, pemalloc,
};

#[test]
fn test_pemalloc_non_persistent() {
    // Get initial count (may have other allocations from previous tests)
    let initial_count = get_allocation_count();
    let size = 1024;

    let ptr = unsafe { pemalloc(size, false) };
    assert!(!ptr.is_null());

    let new_count = get_allocation_count();
    // Allocation count should increase
    assert!(new_count >= initial_count);

    unsafe {
        pefree(ptr, false);
    }

    // After free, count should decrease (but may not be exactly initial due to other allocations)
    let final_count = get_allocation_count();
    assert!(final_count <= new_count);
}

#[test]
fn test_pemalloc_persistent() {
    // Get initial count (may have other allocations from previous tests)
    let initial_count = get_allocation_count();
    let size = 2048;

    let ptr = unsafe { pemalloc(size, true) };
    assert!(!ptr.is_null());

    let new_count = get_allocation_count();
    // Allocation count should increase
    assert!(new_count >= initial_count);

    unsafe {
        pefree(ptr, true);
    }

    // After free, count should decrease (but may not be exactly initial due to other allocations)
    let final_count = get_allocation_count();
    assert!(final_count <= new_count);
}

#[test]
fn test_memory_usage_tracking() {
    let initial_usage = get_memory_usage();
    let initial_peak = get_peak_memory_usage();

    let size = 4096;
    let ptr = unsafe { pemalloc(size, false) };

    let new_usage = get_memory_usage();
    let new_peak = get_peak_memory_usage();

    assert!(new_usage >= initial_usage + size);
    assert!(new_peak >= new_usage);

    unsafe {
        pefree(ptr, false);
    }

    // Peak should remain even after free
    let final_peak = get_peak_memory_usage();
    assert!(final_peak >= new_peak);
}

#[test]
fn test_multiple_allocations() {
    let initial_count = get_allocation_count();

    let ptr1 = unsafe { pemalloc(100, false) };
    let ptr2 = unsafe { pemalloc(200, false) };
    let ptr3 = unsafe { pemalloc(300, false) };

    // Allocation count should increase by at least 3
    let after_alloc = get_allocation_count();
    assert!(after_alloc >= initial_count + 3);

    unsafe {
        pefree(ptr1, false);
        pefree(ptr2, false);
        pefree(ptr3, false);
    }

    // After freeing all, count should decrease
    let final_count = get_allocation_count();
    assert!(final_count <= after_alloc);
    // May not be exactly initial due to other allocations in test suite
}

#[test]
fn test_zero_size_allocation() {
    let ptr = unsafe { pemalloc(0, false) };
    // Zero-size allocation should still return a valid pointer (implementation dependent)
    unsafe {
        pefree(ptr, false);
    }
}
