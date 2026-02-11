//! Memory Management Example
//!
//! Demonstrates PHP memory allocation and statistics

use phprs::alloc::{
    get_allocation_count, get_memory_usage, get_peak_memory_usage, pefree, pemalloc,
};

fn main() {
    println!("=== PHP-RS Memory Management Example ===\n");

    println!("Initial memory stats:");
    println!("  Memory usage: {} bytes", get_memory_usage());
    println!("  Peak usage: {} bytes", get_peak_memory_usage());
    println!("  Allocation count: {}", get_allocation_count());

    // Allocate some memory
    let size1 = 1024;
    let ptr1 = unsafe { pemalloc(size1, false) };
    println!("\nAllocated {} bytes (non-persistent)", size1);

    let size2 = 2048;
    let ptr2 = unsafe { pemalloc(size2, true) };
    println!("Allocated {} bytes (persistent)", size2);

    println!("\nAfter allocation:");
    println!("  Memory usage: {} bytes", get_memory_usage());
    println!("  Peak usage: {} bytes", get_peak_memory_usage());
    println!("  Allocation count: {}", get_allocation_count());

    // Free memory
    unsafe {
        pefree(ptr1, false);
        println!("\nFreed non-persistent allocation");

        pefree(ptr2, true);
        println!("Freed persistent allocation");
    }

    println!("\nAfter freeing:");
    println!("  Memory usage: {} bytes", get_memory_usage());
    println!("  Peak usage: {} bytes", get_peak_memory_usage());
    println!("  Allocation count: {}", get_allocation_count());
}
