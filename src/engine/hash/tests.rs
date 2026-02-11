#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::types::{Val, PhpValue, PhpType};
    use crate::engine::string::string_init;

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
        
        hash_add_or_update(&mut ht, Some(&key), 0, val, 0);
        
        let found = hash_find(&ht, &key);
        assert!(found.is_some());
        if let Some(z) = found {
            assert_eq!(zval_get_long(z), 42);
        }
    }
}

