//! High-performance array operations
//!
//! Optimized array implementation to outperform PHP 8

use super::perf_alloc::MemoryPool;
use crate::engine::types::{Bucket, PhpArray, PhpString, Val};
use crate::vm::execute_data::clone_val;
use std::collections::HashMap;

/// Optimized array with pre-allocation and efficient operations
#[derive(Debug)]
pub struct OptimizedArray {
    pub inner: PhpArray,
    pub string_cache: HashMap<u64, String>, // Cache for string keys
    pub memory_pool: MemoryPool,
}

impl OptimizedArray {
    pub fn new() -> Self {
        Self {
            inner: PhpArray::new(),
            string_cache: HashMap::new(),
            memory_pool: MemoryPool::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let mut array = Self::new();
        array.inner.ar_data.reserve(capacity);
        array.inner.n_table_size = capacity.next_power_of_two() as u32;
        array.inner.n_table_mask = !(array.inner.n_table_size - 1);
        array
    }

    /// Fast array push operation
    #[inline]
    pub fn push(&mut self, value: Val) {
        let idx = self.inner.n_next_free_element;
        let bucket = Bucket {
            val: value,
            h: idx as u64,
            key: None,
        };
        self.inner.ar_data.push(bucket);
        self.inner.n_num_used += 1;
        self.inner.n_num_of_elements += 1;
        self.inner.n_next_free_element += 1;
    }

    /// Fast string key insert with caching
    pub fn insert_string(&mut self, key: &str, value: Val) {
        let key_hash = self.fast_hash(key.as_bytes());

        // Cache the string for future use
        if !self.string_cache.contains_key(&key_hash) {
            self.string_cache.insert(key_hash, key.to_string());
        }

        let key_php = PhpString::new(key, false);
        let bucket = Bucket {
            val: value,
            h: key_hash,
            key: Some(Box::new(key_php)),
        };

        self.inner.ar_data.push(bucket);
        self.inner.n_num_used += 1;
        self.inner.n_num_of_elements += 1;
    }

    /// Fast lookup with caching
    pub fn get(&self, key: &str) -> Option<&Val> {
        let key_hash = self.fast_hash(key.as_bytes());

        for bucket in &self.inner.ar_data {
            if let Some(ref bucket_key) = bucket.key {
                if bucket.h == key_hash && bucket_key.as_str() == key {
                    return Some(&bucket.val);
                }
            }
        }

        None
    }

    /// Fast numeric index lookup
    #[inline]
    pub fn get_index(&self, index: u64) -> Option<&Val> {
        self.inner
            .ar_data
            .get(index as usize)
            .map(|bucket| &bucket.val)
    }

    /// Fast array merge operation
    pub fn merge(&mut self, other: &OptimizedArray) {
        self.inner.ar_data.reserve(other.inner.ar_data.len());

        for bucket in &other.inner.ar_data {
            let mut new_bucket = Bucket {
                val: clone_val(&bucket.val),
                h: bucket.h,
                key: None,
            };

            // Clone string key if present
            if let Some(ref key) = bucket.key {
                let key_str = key.as_str();
                new_bucket.key = Some(Box::new(PhpString::new(key_str, false)));
            }

            self.inner.ar_data.push(new_bucket);
        }

        self.inner.n_num_used += other.inner.n_num_used;
        self.inner.n_num_of_elements += other.inner.n_num_of_elements;
        self.inner.n_next_free_element = self
            .inner
            .n_next_free_element
            .max(other.inner.n_next_free_element);
    }

    /// Fast array iteration
    pub fn iter(&self) -> impl Iterator<Item = &Val> {
        self.inner.ar_data.iter().map(|bucket| &bucket.val)
    }

    /// Fast keyed iteration
    pub fn iter_keys(&self) -> impl Iterator<Item = (&str, &Val)> {
        self.inner
            .ar_data
            .iter()
            .filter_map(|bucket| bucket.key.as_ref().map(|key| (key.as_str(), &bucket.val)))
    }

    /// Fast array count
    #[inline]
    pub fn len(&self) -> usize {
        self.inner.n_num_of_elements as usize
    }

    /// Check if array is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.n_num_of_elements == 0
    }

    /// Clear array efficiently
    pub fn clear(&mut self) {
        self.inner.ar_data.clear();
        self.inner.n_num_used = 0;
        self.inner.n_num_of_elements = 0;
        self.inner.n_next_free_element = 0;
        self.string_cache.clear();
    }

    /// Fast hash function for strings
    #[inline]
    fn fast_hash(&self, data: &[u8]) -> u64 {
        let mut hash = 5381u64;
        for &byte in data {
            hash = hash.wrapping_mul(33).wrapping_add(byte as u64);
        }
        hash
    }

    /// Optimize memory layout
    pub fn compact(&mut self) {
        // Remove deleted entries and compact memory
        self.inner
            .ar_data
            .retain(|bucket| match bucket.val.get_type() {
                crate::engine::types::PhpType::Undef => false,
                _ => true,
            });

        self.inner.n_num_used = self.inner.ar_data.len() as u32;
        self.inner.n_num_of_elements = self.inner.n_num_used;

        // Shrink capacity if needed
        if self.inner.ar_data.capacity() > self.inner.ar_data.len() * 2 {
            self.inner.ar_data.shrink_to_fit();
        }
    }
}

impl Default for OptimizedArray {
    fn default() -> Self {
        Self::new()
    }
}

/// High-performance array operations
pub struct ArrayOps;

impl ArrayOps {
    /// Fast array creation from iterator
    pub fn from_iter<T: IntoIterator<Item = Val>>(iter: T) -> OptimizedArray {
        let iterator = iter.into_iter();
        let (lower, upper) = iterator.size_hint();
        let mut array = OptimizedArray::with_capacity(upper.unwrap_or(lower));

        for item in iterator {
            array.push(item);
        }

        array
    }

    /// Fast range array creation
    pub fn range(start: i64, end: i64, step: i64) -> OptimizedArray {
        let size = ((end - start).abs() / step.abs() + 1) as usize;
        let mut array = OptimizedArray::with_capacity(size);

        let mut current = start;
        if step > 0 {
            while current <= end {
                array.push(Val::new(
                    crate::engine::types::PhpValue::Long(current),
                    crate::engine::types::PhpType::Long,
                ));
                current += step;
            }
        } else {
            while current >= end {
                array.push(Val::new(
                    crate::engine::types::PhpValue::Long(current),
                    crate::engine::types::PhpType::Long,
                ));
                current += step;
            }
        }

        array
    }

    /// Fast array slice operation
    pub fn slice(array: &OptimizedArray, offset: isize, length: Option<usize>) -> OptimizedArray {
        let len = array.len();
        let start = if offset >= 0 {
            offset as usize
        } else {
            len.saturating_sub((-offset) as usize)
        };

        let end = match length {
            Some(len) => start.saturating_add(len).min(array.inner.ar_data.len()),
            None => array.inner.ar_data.len(),
        };

        let mut result = OptimizedArray::with_capacity(end - start);

        for bucket in array.inner.ar_data[start..end].iter() {
            let mut new_bucket = Bucket {
                val: clone_val(&bucket.val),
                h: bucket.h,
                key: None,
            };

            if let Some(ref key) = bucket.key {
                new_bucket.key = Some(Box::new(PhpString::new(key.as_str(), false)));
            }

            result.inner.ar_data.push(new_bucket);
        }

        result.inner.n_num_used = result.inner.ar_data.len() as u32;
        result.inner.n_num_of_elements = result.inner.n_num_used;

        result
    }

    /// Fast array chunk operation
    pub fn chunk(array: &OptimizedArray, size: usize) -> Vec<OptimizedArray> {
        let mut result = Vec::with_capacity((array.len() + size - 1) / size);

        for chunk in array.inner.ar_data.chunks(size) {
            let mut new_array = OptimizedArray::with_capacity(chunk.len());

            for bucket in chunk {
                let mut new_bucket = Bucket {
                    val: clone_val(&bucket.val),
                    h: bucket.h,
                    key: None,
                };

                if let Some(ref key) = bucket.key {
                    new_bucket.key = Some(Box::new(PhpString::new(key.as_str(), false)));
                }

                new_array.inner.ar_data.push(new_bucket);
            }

            new_array.inner.n_num_used = new_array.inner.ar_data.len() as u32;
            new_array.inner.n_num_of_elements = new_array.inner.n_num_used;

            result.push(new_array);
        }

        result
    }

    /// Fast array filter operation
    pub fn filter<F>(array: &OptimizedArray, mut predicate: F) -> OptimizedArray
    where
        F: FnMut(&Val) -> bool,
    {
        let mut result = OptimizedArray::with_capacity(array.len());

        for bucket in &array.inner.ar_data {
            if predicate(&bucket.val) {
                let mut new_bucket = Bucket {
                    val: clone_val(&bucket.val),
                    h: bucket.h,
                    key: None,
                };

                if let Some(ref key) = bucket.key {
                    new_bucket.key = Some(Box::new(PhpString::new(key.as_str(), false)));
                }

                result.inner.ar_data.push(new_bucket);
            }
        }

        result.inner.n_num_used = result.inner.ar_data.len() as u32;
        result.inner.n_num_of_elements = result.inner.n_num_used;

        result
    }

    /// Fast array map operation
    pub fn map<F>(array: &OptimizedArray, mut mapper: F) -> OptimizedArray
    where
        F: FnMut(&Val) -> Val,
    {
        let mut result = OptimizedArray::with_capacity(array.len());

        for bucket in &array.inner.ar_data {
            let mapped = mapper(&bucket.val);
            let mut new_bucket = Bucket {
                val: mapped,
                h: bucket.h,
                key: None,
            };

            if let Some(ref key) = bucket.key {
                new_bucket.key = Some(Box::new(PhpString::new(key.as_str(), false)));
            }

            result.inner.ar_data.push(new_bucket);
        }

        result.inner.n_num_used = result.inner.ar_data.len() as u32;
        result.inner.n_num_of_elements = result.inner.n_num_used;

        result
    }

    /// Fast array reduce operation
    pub fn reduce<F, T>(array: &OptimizedArray, mut reducer: F, initial: T) -> T
    where
        F: FnMut(T, &Val) -> T,
    {
        let mut result = initial;

        for bucket in &array.inner.ar_data {
            result = reducer(result, &bucket.val);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::types::{PhpType, PhpValue};

    #[test]
    fn test_optimized_array_push() {
        let mut array = OptimizedArray::new();
        let val = Val::new(PhpValue::Long(42), PhpType::Long);

        array.push(val);
        assert_eq!(array.len(), 1);
        assert!(array.get_index(0).is_some());
    }

    #[test]
    fn test_optimized_array_string_insert() {
        let mut array = OptimizedArray::new();
        let val = Val::new(PhpValue::Long(42), PhpType::Long);

        array.insert_string("test_key", val);
        assert_eq!(array.len(), 1);
        assert!(array.get("test_key").is_some());
    }

    #[test]
    fn test_array_ops_range() {
        let array = ArrayOps::range(1, 5, 1);
        assert_eq!(array.len(), 5);
        assert_eq!(array.get_index(0).unwrap().get_type(), PhpType::Long);
    }

    #[test]
    fn test_array_ops_slice() {
        let mut array = OptimizedArray::with_capacity(10);
        for i in 0..10 {
            array.push(Val::new(PhpValue::Long(i as i64), PhpType::Long));
        }

        let slice = ArrayOps::slice(&array, 2, Some(3));
        assert_eq!(slice.len(), 3);
    }

    #[test]
    fn test_array_ops_filter() {
        let mut array = OptimizedArray::with_capacity(10);
        for i in 0..10 {
            array.push(Val::new(PhpValue::Long(i as i64), PhpType::Long));
        }

        let filtered = ArrayOps::filter(&array, |val| {
            if let PhpValue::Long(n) = val.value {
                n % 2 == 0
            } else {
                false
            }
        });

        assert_eq!(filtered.len(), 5);
    }

    #[test]
    fn test_array_ops_map() {
        let mut array = OptimizedArray::with_capacity(5);
        for i in 0..5 {
            array.push(Val::new(PhpValue::Long(i as i64), PhpType::Long));
        }

        let mapped = ArrayOps::map(&array, |val| {
            if let PhpValue::Long(n) = val.value {
                Val::new(PhpValue::Long(n * 2), PhpType::Long)
            } else {
                val.clone()
            }
        });

        assert_eq!(mapped.len(), 5);
    }
}
