//! String interning.
//!
//! Deduplicates frequently-repeated strings (class/method names, identifiers,
//! constant keys, builtin function names) into a single shared allocation so
//! that later equality checks reduce to cheap pointer comparison and memory
//! for duplicate copies is reclaimed.
//!
//! This is the building block referenced by the "String interning" TODO item.
//! It is exercised by tests; adopting it across more hot paths (lexer
//! identifiers, class-table keys) is incremental future work.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use crate::engine::string::hash_func;

/// A handle to a canonicalized string. Two handles compare equal iff the
/// underlying content is equal; equal content shares the same `Arc<str>`.
#[derive(Debug, Clone)]
pub struct InternedString(Arc<str>);

impl InternedString {
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// True when two handles point at the exact same allocation (fast path).
    #[inline]
    pub fn is_same_allocation(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl PartialEq for InternedString {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || self.0 == other.0
    }
}
impl Eq for InternedString {}

impl std::hash::Hash for InternedString {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}

impl AsRef<str> for InternedString {
    #[inline]
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Concurrent string interner. Strings are bucketed by their DJBX33A hash so
/// that lookups only scan collisions for the same hash.
pub struct StringInterner {
    // hash -> vec of unique strings sharing that hash
    entries: Mutex<HashMap<u64, Vec<Arc<str>>>>,
    total_intern_calls: AtomicU64,
    dedup_hits: AtomicU64,
}

impl StringInterner {
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            total_intern_calls: AtomicU64::new(0),
            dedup_hits: AtomicU64::new(0),
        }
    }

    /// Intern `s`, returning a handle that shares storage with any equal prior
    /// intern. Equal content always yields handles comparing `==`.
    pub fn intern(&self, s: &str) -> InternedString {
        self.total_intern_calls.fetch_add(1, Ordering::Relaxed);
        let h = hash_func(s.as_bytes());
        let mut entries = self.entries.lock().unwrap();
        let bucket = entries.entry(h).or_default();
        for existing in bucket.iter() {
            if existing.as_ref() == s {
                self.dedup_hits.fetch_add(1, Ordering::Relaxed);
                return InternedString(existing.clone());
            }
        }
        let arc: Arc<str> = Arc::from(s);
        bucket.push(arc.clone());
        InternedString(arc)
    }

    /// Number of unique interned strings currently held.
    pub fn unique_count(&self) -> usize {
        self.entries
            .lock()
            .unwrap()
            .values()
            .map(|v| v.len())
            .sum()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.unique_count() == 0
    }

    /// (total intern calls, dedup hits) — useful for tests and telemetry.
    pub fn stats(&self) -> (u64, u64) {
        (
            self.total_intern_calls.load(Ordering::Relaxed),
            self.dedup_hits.load(Ordering::Relaxed),
        )
    }

    /// Remove every interned entry. Mainly for tests.
    pub fn clear(&self) {
        self.entries.lock().unwrap().clear();
        self.total_intern_calls.store(0, Ordering::Relaxed);
        self.dedup_hits.store(0, Ordering::Relaxed);
    }
}

impl Default for StringInterner {
    fn default() -> Self {
        Self::new()
    }
}

static GLOBAL_INTERNER: OnceLock<StringInterner> = OnceLock::new();

/// Process-wide interner shared by helpers below.
pub fn global() -> &'static StringInterner {
    GLOBAL_INTERNER.get_or_init(StringInterner::new)
}

/// Intern through the global interner.
pub fn intern(s: &str) -> InternedString {
    global().intern(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interns_equal_strings_share_allocation() {
        let it = StringInterner::new();
        let a = it.intern("hello");
        let b = it.intern("hello");
        assert!(a.is_same_allocation(&b));
        assert_eq!(a, b);
        assert_eq!(a.as_str(), "hello");
        assert_eq!(it.unique_count(), 1);
    }

    #[test]
    fn distinct_strings_stay_distinct() {
        let it = StringInterner::new();
        let a = it.intern("foo");
        let b = it.intern("bar");
        assert_ne!(a, b);
        assert!(!a.is_same_allocation(&b));
        assert_eq!(it.unique_count(), 2);
    }

    #[test]
    fn stats_track_dedup_hits() {
        let it = StringInterner::new();
        it.intern("x");
        it.intern("x");
        it.intern("y");
        let (calls, hits) = it.stats();
        assert_eq!(calls, 3);
        assert_eq!(hits, 1, "second 'x' should be a dedup hit");
        assert_eq!(it.unique_count(), 2);
    }

    #[test]
    fn handles_hash_collisions_correctly() {
        // Interning many distinct strings must still keep them distinct even
        // if some happen to share a DJBX33A bucket.
        let it = StringInterner::new();
        let words = ["alpha", "beta", "gamma", "delta", "epsilon", "alpha"];
        let handles: Vec<_> = words.iter().map(|w| it.intern(w)).collect();
        assert_eq!(handles[0], handles[5], "duplicate 'alpha' must match");
        assert_ne!(handles[0], handles[1]);
        assert_eq!(it.unique_count(), 5);
    }

    #[test]
    fn global_interner_is_consistent() {
        // Run in isolation (clear first) so order of test execution does not matter.
        global().clear();
        let a = intern("phprs_global_token");
        let b = intern("phprs_global_token");
        assert!(a.is_same_allocation(&b));
        global().clear();
    }

    #[test]
    fn works_as_hashmap_key() {
        let mut m: HashMap<InternedString, i32> = HashMap::new();
        let k1 = intern("phprs_key_1");
        let k2 = intern("phprs_key_1");
        m.insert(k1, 42);
        assert_eq!(m.get(&k2), Some(&42));
        global().clear();
    }

    #[test]
    fn empty_string_interns_fine() {
        let it = StringInterner::new();
        let a = it.intern("");
        let b = it.intern("");
        assert!(a.is_same_allocation(&b));
        assert_eq!(a.as_str(), "");
    }
}
