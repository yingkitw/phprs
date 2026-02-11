//! Unit tests for garbage collection

use crate::engine::gc::Gc;

#[test]
fn test_gc_new() {
    let gc = Gc::new();
    // Verify GC is initialized
    assert_eq!(gc.should_collect(), false);
    let stats = gc.get_stats();
    assert_eq!(stats.collected, 0);
}

#[test]
fn test_gc_set_threshold() {
    let gc = Gc::new();
    gc.set_threshold(5000);
    // Verify threshold was set by checking stats
    let stats = gc.get_stats();
    assert_eq!(stats.threshold, 5000);
}

#[test]
fn test_gc_should_collect() {
    let mut gc = Gc::new();
    gc.set_threshold(5);

    // Initially should not collect
    assert_eq!(gc.should_collect(), false);
}

#[test]
fn test_gc_collect_cycles() {
    let mut gc = Gc::new();
    gc.set_threshold(1);

    // Without actual roots, collection should return 0
    let cycles = gc.collect_cycles();
    assert_eq!(cycles, 0);
}

#[test]
fn test_gc_stats() {
    let gc = Gc::new();
    let stats = gc.get_stats();
    assert!(stats.root_count >= 0);
    assert!(stats.collected >= 0);
    assert!(stats.threshold > 0);
}
