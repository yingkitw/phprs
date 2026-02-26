//! Hash table implementation
//!
//! Hash table

use crate::engine::string::string_hash_func;
use crate::engine::types::{Bucket, PhpArray, PhpResult, PhpString, Val};

/// Calculate table mask from table size
fn ht_size_to_mask(n_table_size: u32) -> u32 {
    if n_table_size == 0 {
        return !0u32;
    }
    !(n_table_size - 1)
}

/// Initialize a hash table
pub fn hash_init(ht: &mut PhpArray, n_size: u32) {
    ht.n_table_size = if n_size < 8 {
        8
    } else {
        n_size.next_power_of_two()
    };
    ht.n_table_mask = !(ht.n_table_size - 1);
    ht.ar_data = Vec::with_capacity(ht.n_table_size as usize);
    ht.n_num_used = 0;
    ht.n_num_of_elements = 0;
    ht.n_internal_pointer = 0;
    ht.n_next_free_element = 0;
}

/// Add or update an element in the hash table
pub fn hash_add_or_update(
    ht: &mut PhpArray,
    key: Option<&PhpString>,
    h: u64,
    p_data: Val,
    flag: u32,
) -> PhpResult {
    // Check if we need to resize
    if ht.n_num_used >= ht.n_table_size {
        hash_do_resize(ht);
    }

    // Calculate hash index
    let hash = if let Some(k) = key {
        string_hash_func(k)
    } else {
        h
    };

    // Check if key already exists
    let existing_idx = ht.ar_data.iter().position(|bucket| {
        if let Some(k) = key {
            bucket
                .key
                .as_ref()
                .map(|bk| bk.as_str() == k.as_str())
                .unwrap_or(false)
        } else {
            bucket.h == hash
        }
    });

    if let Some(idx) = existing_idx {
        if flag & 0x01 == 0 {
            // HASH_UPDATE
            // Update existing
            ht.ar_data[idx].val = p_data;
            return PhpResult::Success;
        } else {
            // HASH_ADD - fail if exists
            return PhpResult::Failure;
        }
    }

    // Add new bucket
    let bucket = Bucket {
        val: p_data,
        h: hash,
        key: key.map(|k| Box::new(PhpString::new(k.as_str(), false))),
    };

    ht.ar_data.push(bucket);
    ht.n_num_used += 1;
    ht.n_num_of_elements += 1;

    PhpResult::Success
}

/// Resize hash table
fn hash_do_resize(ht: &mut PhpArray) {
    let new_size = ht.n_table_size * 2;
    ht.n_table_size = new_size;
    ht.n_table_mask = ht_size_to_mask(new_size);
    ht.ar_data.reserve(new_size as usize);
}

/// Find an element in the hash table by string key
pub fn hash_find<'a>(ht: &'a PhpArray, key: &PhpString) -> Option<&'a Val> {
    let _hash = string_hash_func(key);
    ht.ar_data
        .iter()
        .find(|bucket| {
            bucket
                .key
                .as_ref()
                .map(|k| k.as_str() == key.as_str())
                .unwrap_or(false)
        })
        .map(|bucket| &bucket.val)
}

/// Find an element in the hash table by numeric index
pub fn hash_index_find<'a>(ht: &'a PhpArray, h: u64) -> Option<&'a Val> {
    ht.ar_data
        .iter()
        .find(|bucket| bucket.h == h && bucket.key.is_none())
        .map(|bucket| &bucket.val)
}

/// Delete an element from the hash table
pub fn hash_del(ht: &mut PhpArray, key: &PhpString) -> PhpResult {
    let pos = ht.ar_data.iter().position(|bucket| {
        bucket
            .key
            .as_ref()
            .map(|k| k.as_str() == key.as_str())
            .unwrap_or(false)
    });

    if let Some(pos) = pos {
        ht.ar_data.remove(pos);
        ht.n_num_of_elements -= 1;
        PhpResult::Success
    } else {
        PhpResult::Failure
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::operators::zval_get_long;
    use crate::engine::string::string_init;
    use crate::engine::types::{PhpType, PhpValue, Val};

    #[test]
    fn test_hash_init() {
        let mut ht = PhpArray::new();
        hash_init(&mut ht, 16);
        assert_eq!(ht.n_table_size, 16);
        assert_eq!(ht.n_num_of_elements, 0);
    }

    #[test]
    fn test_hash_add_and_find() {
        let mut ht = PhpArray::new();
        hash_init(&mut ht, 8);

        let key = string_init("test_key", false);
        let val = Val::new(PhpValue::Long(42), PhpType::Long);

        let result = hash_add_or_update(&mut ht, Some(&key), 0, val, 0);
        assert_eq!(result, PhpResult::Success);

        let found = hash_find(&ht, &key);
        assert!(found.is_some());
        if let Some(z) = found {
            assert_eq!(zval_get_long(z), 42);
        }
    }

    #[test]
    fn test_hash_del() {
        let mut ht = PhpArray::new();
        hash_init(&mut ht, 8);

        let key = string_init("test_key", false);
        let val = Val::new(PhpValue::Long(42), PhpType::Long);

        hash_add_or_update(&mut ht, Some(&key), 0, val, 0);
        assert_eq!(ht.n_num_of_elements, 1);

        let result = hash_del(&mut ht, &key);
        assert_eq!(result, PhpResult::Success);
        assert_eq!(ht.n_num_of_elements, 0);

        let found = hash_find(&ht, &key);
        assert!(found.is_none());
    }
}
