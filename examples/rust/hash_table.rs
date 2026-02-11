//! Hash Table Example
//!
//! Demonstrates PHP array/hash table operations

use phprs::hash::{hash_add_or_update, hash_find, hash_index_find, hash_init};
use phprs::string::string_init;
use phprs::{PhpArray, PhpType, PhpValue, Val};

fn main() {
    println!("=== phprs Hash Table Example ===\n");

    // Initialize a hash table
    let mut ht = PhpArray::new();
    hash_init(&mut ht, 8);

    // Add string keys
    let key1 = string_init("name", false);
    let val1 = Val::new(
        PhpValue::String(Box::new(string_init("phprs", false))),
        PhpType::String,
    );
    hash_add_or_update(&mut ht, Some(&key1), 0, val1, 0);

    let key2 = string_init("version", false);
    let val2 = Val::new(PhpValue::Long(1), PhpType::Long);
    hash_add_or_update(&mut ht, Some(&key2), 0, val2, 0);

    // Add numeric keys
    let key3 = string_init("pi", false);
    let val3 = Val::new(PhpValue::Double(3.14), PhpType::Double);
    hash_add_or_update(&mut ht, Some(&key3), 0, val3, 0);

    println!("Hash table size: {}", ht.n_num_of_elements);

    // Find by string key
    if let Some(found) = hash_find(&ht, &key1) {
        println!("Found 'name': {:?}", found);
    }

    if let Some(found) = hash_find(&ht, &key2) {
        println!("Found 'version': {:?}", found);
    }

    // Find by numeric index
    if let Some(found) = hash_index_find(&ht, 0) {
        println!("Found at index 0: {:?}", found);
    }

    println!("\nHash table contents:");
    for (i, bucket) in ht.data.iter().enumerate() {
        if let Some(ref key) = bucket.key {
            println!("  [{}] '{}' => {:?}", i, key.as_str(), bucket.val);
        } else {
            println!("  [{}] {} => {:?}", i, bucket.h, bucket.val);
        }
    }
}
