#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::string::*;

    #[test]
    fn test_string_init() {
        let s = string_init("hello", false);
        assert_eq!(s.as_str(), "hello");
        assert_eq!(s.len, 5);
    }

    #[test]
    fn test_string_hash_func() {
        let s = string_init("test", false);
        let hash1 = string_hash_func(&s);
        let hash2 = string_hash_func(&s);
        assert_eq!(hash1, hash2); // Hash should be cached
    }

    #[test]
    fn test_string_concat2() {
        let result = string_concat2("hello", " world");
        assert_eq!(result.as_str(), "hello world");
    }

    #[test]
    fn test_string_concat3() {
        let result = string_concat3("a", "b", "c");
        assert_eq!(result.as_str(), "abc");
    }
}

