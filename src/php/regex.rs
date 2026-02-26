//! Regular Expression Support
//!
//! PHP-compatible regex functions using Rust regex crate

use regex::Regex;

/// Compile a PCRE pattern to Rust regex
/// Converts PHP PCRE delimiters and flags to Rust regex syntax
pub fn compile_pattern(pattern: &str) -> Result<Regex, String> {
    // PHP patterns are typically /pattern/flags
    if pattern.len() < 3 {
        return Err("Invalid regex pattern".to_string());
    }

    let delimiter = pattern.chars().next().unwrap();
    let end_pos = pattern.rfind(delimiter);

    if end_pos.is_none() || end_pos == Some(0) {
        return Err("Invalid regex pattern - missing delimiter".to_string());
    }

    let end_pos = end_pos.unwrap();
    let regex_part = &pattern[1..end_pos];
    let flags = if end_pos + 1 < pattern.len() {
        &pattern[end_pos + 1..]
    } else {
        ""
    };

    // Build regex with flags
    let mut regex_str = String::new();

    // Handle flags
    if flags.contains('i') {
        regex_str.push_str("(?i)");
    }
    if flags.contains('m') {
        regex_str.push_str("(?m)");
    }
    if flags.contains('s') {
        regex_str.push_str("(?s)");
    }
    if flags.contains('x') {
        regex_str.push_str("(?x)");
    }

    regex_str.push_str(regex_part);

    Regex::new(&regex_str).map_err(|e| format!("Regex compilation error: {}", e))
}

/// Perform preg_match operation
pub fn preg_match(
    pattern: &str,
    subject: &str,
    matches: Option<&mut Vec<String>>,
) -> Result<i64, String> {
    let re = compile_pattern(pattern)?;

    if let Some(caps) = re.captures(subject) {
        if let Some(m) = matches {
            m.clear();
            for cap in caps.iter() {
                if let Some(c) = cap {
                    m.push(c.as_str().to_string());
                } else {
                    m.push(String::new());
                }
            }
        }
        Ok(1)
    } else {
        if let Some(m) = matches {
            m.clear();
        }
        Ok(0)
    }
}

/// Perform preg_match_all operation
pub fn preg_match_all(pattern: &str, subject: &str) -> Result<Vec<Vec<String>>, String> {
    let re = compile_pattern(pattern)?;
    let mut all_matches = Vec::new();

    for caps in re.captures_iter(subject) {
        let mut match_group = Vec::new();
        for cap in caps.iter() {
            if let Some(c) = cap {
                match_group.push(c.as_str().to_string());
            } else {
                match_group.push(String::new());
            }
        }
        all_matches.push(match_group);
    }

    Ok(all_matches)
}

/// Perform preg_replace operation
pub fn preg_replace(pattern: &str, replacement: &str, subject: &str) -> Result<String, String> {
    let re = compile_pattern(pattern)?;
    Ok(re.replace_all(subject, replacement).to_string())
}

/// Perform preg_split operation
pub fn preg_split(
    pattern: &str,
    subject: &str,
    limit: Option<usize>,
) -> Result<Vec<String>, String> {
    let re = compile_pattern(pattern)?;
    let parts: Vec<String> = if let Some(lim) = limit {
        re.splitn(subject, lim).map(|s| s.to_string()).collect()
    } else {
        re.split(subject).map(|s| s.to_string()).collect()
    };
    Ok(parts)
}

/// Perform preg_grep operation
pub fn preg_grep(pattern: &str, input: &[String]) -> Result<Vec<String>, String> {
    let re = compile_pattern(pattern)?;
    let mut result = Vec::new();

    for item in input {
        if re.is_match(item) {
            result.push(item.clone());
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_pattern() {
        let re = compile_pattern("/test/").unwrap();
        assert!(re.is_match("test"));
        assert!(!re.is_match("TEST"));
    }

    #[test]
    fn test_compile_pattern_case_insensitive() {
        let re = compile_pattern("/test/i").unwrap();
        assert!(re.is_match("test"));
        assert!(re.is_match("TEST"));
    }

    #[test]
    fn test_preg_match() {
        let mut matches = Vec::new();
        let result = preg_match("/h(\\w+)o/", "hello", Some(&mut matches)).unwrap();
        assert_eq!(result, 1);
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0], "hello");
        assert_eq!(matches[1], "ell");
    }

    #[test]
    fn test_preg_match_no_match() {
        let result = preg_match("/xyz/", "hello", None).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_preg_replace() {
        let result = preg_replace("/world/", "Rust", "Hello world").unwrap();
        assert_eq!(result, "Hello Rust");
    }

    #[test]
    fn test_preg_split() {
        let result = preg_split("/,\\s*/", "a, b, c", None).unwrap();
        assert_eq!(result, vec!["a", "b", "c"]);
    }
}
