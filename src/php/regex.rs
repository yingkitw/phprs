//! Regular Expression Support
//!
//! PHP-compatible regex functions using the Rust `regex` crate, with
//! `fancy-regex` for look-around assertions that the std engine lacks.

use fancy_regex::Regex as FancyRegex;
use regex::Regex;

/// Compiled pattern: fast std engine, or fancy engine for look-around.
enum CompiledPattern {
    Std(Regex),
    Fancy(FancyRegex),
}

enum Captures<'a> {
    Std(regex::Captures<'a>),
    Fancy(fancy_regex::Captures<'a>),
}

impl Captures<'_> {
    fn to_groups(&self) -> Vec<String> {
        let len = match self {
            Self::Std(caps) => caps.len(),
            Self::Fancy(caps) => caps.len(),
        };
        (0..len)
            .map(|i| match self {
                Self::Std(caps) => caps
                    .get(i)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default(),
                Self::Fancy(caps) => caps
                    .get(i)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default(),
            })
            .collect()
    }
}

impl CompiledPattern {
    fn captures<'a>(&self, subject: &'a str) -> Result<Option<Captures<'a>>, String> {
        match self {
            Self::Std(re) => Ok(re.captures(subject).map(Captures::Std)),
            Self::Fancy(re) => re
                .captures(subject)
                .map(|caps| caps.map(Captures::Fancy))
                .map_err(|e| format!("Regex match error: {}", e)),
        }
    }

    fn replace_all(&self, subject: &str, replacement: &str) -> Result<String, String> {
        match self {
            Self::Std(re) => Ok(re.replace_all(subject, replacement).into_owned()),
            Self::Fancy(re) => Ok(re.replace_all(subject, replacement).into_owned()),
        }
    }

    fn split(&self, subject: &str, limit: Option<usize>) -> Result<Vec<String>, String> {
        match self {
            Self::Std(re) => {
                let parts = if let Some(lim) = limit {
                    re.splitn(subject, lim).map(str::to_string).collect()
                } else {
                    re.split(subject).map(str::to_string).collect()
                };
                Ok(parts)
            }
            Self::Fancy(re) => {
                let parts = if let Some(lim) = limit {
                    re.splitn(subject, lim)
                        .map(|part| {
                            part.map(str::to_string)
                                .map_err(|e| format!("Regex match error: {}", e))
                        })
                        .collect::<Result<Vec<_>, _>>()?
                } else {
                    re.split(subject)
                        .map(|part| {
                            part.map(str::to_string)
                                .map_err(|e| format!("Regex match error: {}", e))
                        })
                        .collect::<Result<Vec<_>, _>>()?
                };
                Ok(parts)
            }
        }
    }

    fn is_match(&self, subject: &str) -> Result<bool, String> {
        match self {
            Self::Std(re) => Ok(re.is_match(subject)),
            Self::Fancy(re) => re
                .is_match(subject)
                .map_err(|e| format!("Regex match error: {}", e)),
        }
    }

    fn match_all_groups(&self, subject: &str) -> Result<Vec<Vec<String>>, String> {
        match self {
            Self::Std(re) => {
                let mut all_matches = Vec::new();
                for caps in re.captures_iter(subject) {
                    all_matches.push(Captures::Std(caps).to_groups());
                }
                Ok(all_matches)
            }
            Self::Fancy(re) => {
                let mut all_matches = Vec::new();
                for caps in re.captures_iter(subject) {
                    let caps = caps.map_err(|e| format!("Regex match error: {}", e))?;
                    all_matches.push(Captures::Fancy(caps).to_groups());
                }
                Ok(all_matches)
            }
        }
    }
}

fn needs_fancy_engine(regex_part: &str) -> bool {
    regex_part.contains("(?=")
        || regex_part.contains("(?!")
        || regex_part.contains("(?<=")
        || regex_part.contains("(?<!")
}

fn parse_pcre_pattern(pattern: &str) -> Result<(String, String), String> {
    if pattern.len() < 3 {
        return Err("Invalid regex pattern".to_string());
    }

    let delimiter = pattern.chars().next().unwrap();
    let end_pos = pattern.rfind(delimiter);

    if end_pos.is_none() || end_pos == Some(0) {
        return Err("Invalid regex pattern - missing delimiter".to_string());
    }

    let end_pos = end_pos.unwrap();
    let regex_part = pattern[1..end_pos].to_string();
    let flags = if end_pos + 1 < pattern.len() {
        pattern[end_pos + 1..].to_string()
    } else {
        String::new()
    };

    Ok((regex_part, flags))
}

fn build_regex_string(regex_part: &str, flags: &str) -> String {
    let mut regex_str = String::new();

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
    regex_str
}

/// Compile a PCRE pattern to a Rust regex engine (std or fancy).
fn compile_pattern(pattern: &str) -> Result<CompiledPattern, String> {
    let (regex_part, flags) = parse_pcre_pattern(pattern)?;
    let regex_str = build_regex_string(&regex_part, &flags);

    if needs_fancy_engine(&regex_part) {
        FancyRegex::new(&regex_str)
            .map(CompiledPattern::Fancy)
            .map_err(|e| format!("Regex compilation error: {}", e))
    } else {
        Regex::new(&regex_str)
            .map(CompiledPattern::Std)
            .map_err(|e| format!("Regex compilation error: {}", e))
    }
}

/// Perform preg_match operation
pub fn preg_match(
    pattern: &str,
    subject: &str,
    matches: Option<&mut Vec<String>>,
) -> Result<i64, String> {
    let re = compile_pattern(pattern)?;

    if let Some(caps) = re.captures(subject)? {
        if let Some(m) = matches {
            *m = caps.to_groups();
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
    re.match_all_groups(subject)
}

/// Perform preg_replace operation
pub fn preg_replace(pattern: &str, replacement: &str, subject: &str) -> Result<String, String> {
    let re = compile_pattern(pattern)?;
    re.replace_all(subject, replacement)
}

/// Perform preg_split operation
pub fn preg_split(
    pattern: &str,
    subject: &str,
    limit: Option<usize>,
) -> Result<Vec<String>, String> {
    let re = compile_pattern(pattern)?;
    re.split(subject, limit)
}

/// Perform preg_grep operation
pub fn preg_grep(pattern: &str, input: &[String]) -> Result<Vec<String>, String> {
    let re = compile_pattern(pattern)?;
    let mut result = Vec::new();

    for item in input {
        if re.is_match(item)? {
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
        assert!(re.is_match("test").unwrap());
        assert!(!re.is_match("TEST").unwrap());
    }

    #[test]
    fn test_compile_pattern_case_insensitive() {
        let re = compile_pattern("/test/i").unwrap();
        assert!(re.is_match("test").unwrap());
        assert!(re.is_match("TEST").unwrap());
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

    #[test]
    fn test_preg_match_positive_lookahead() {
        let result = preg_match("/foo(?=bar)/", "foobar", None).unwrap();
        assert_eq!(result, 1);

        let result = preg_match("/foo(?=bar)/", "foobaz", None).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_preg_match_negative_lookahead() {
        let result = preg_match("/foo(?!bar)/", "foobaz", None).unwrap();
        assert_eq!(result, 1);

        let result = preg_match("/foo(?!bar)/", "foobar", None).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_preg_match_positive_lookbehind() {
        let result = preg_match("/(?<=foo)bar/", "foobar", None).unwrap();
        assert_eq!(result, 1);

        let result = preg_match("/(?<=foo)bar/", "bazbar", None).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_preg_match_negative_lookbehind() {
        let result = preg_match("/(?<!foo)bar/", "bazbar", None).unwrap();
        assert_eq!(result, 1);

        let result = preg_match("/(?<!foo)bar/", "foobar", None).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_preg_match_all_lookahead() {
        let matches = preg_match_all("/\\w+(?=!)/", "so fancy! even with!").unwrap();
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0][0], "fancy");
        assert_eq!(matches[1][0], "with");
    }

    #[test]
    fn test_preg_replace_lookahead() {
        let result = preg_replace("/(\\w+)(?=\\s+is)/", "[$1]", "this is fine").unwrap();
        assert_eq!(result, "[this] is fine");
    }

    #[test]
    fn test_preg_grep_lookahead() {
        let input = vec![
            "foobar".to_string(),
            "foobaz".to_string(),
            "barfoo".to_string(),
        ];
        let result = preg_grep("/foo(?=bar)/", &input).unwrap();
        assert_eq!(result, vec!["foobar".to_string()]);
    }
}
