//! Keyword recognition and token mapping

use super::tokens::TokenType;

/// Convert keyword string to token
pub fn keyword_to_token(keyword: &str) -> TokenType {
    match keyword {
        "if" => TokenType::T_IF,
        "else" => TokenType::T_ELSE,
        "elseif" => TokenType::T_ELSEIF,
        "while" => TokenType::T_WHILE,
        "for" => TokenType::T_FOR,
        "foreach" => TokenType::T_FOREACH,
        "function" => TokenType::T_FUNCTION,
        "class" => TokenType::T_CLASS,
        "return" => TokenType::T_RETURN,
        "echo" => TokenType::T_ECHO,
        "true" => TokenType::T_STRING,
        "false" => TokenType::T_STRING,
        "null" => TokenType::T_STRING,
        "array" => TokenType::T_ARRAY,
        "new" => TokenType::T_NEW,
        "static" => TokenType::T_STATIC,
        "public" => TokenType::T_PUBLIC,
        "private" => TokenType::T_PRIVATE,
        "protected" => TokenType::T_PROTECTED,
        "const" => TokenType::T_CONST,
        "break" => TokenType::T_BREAK,
        "continue" => TokenType::T_CONTINUE,
        "switch" => TokenType::T_SWITCH,
        "case" => TokenType::T_CASE,
        "default" => TokenType::T_DEFAULT,
        "try" => TokenType::T_TRY,
        "catch" => TokenType::T_CATCH,
        "throw" => TokenType::T_THROW,
        "finally" => TokenType::T_FINALLY,
        "namespace" => TokenType::T_NAMESPACE,
        "use" => TokenType::T_USE,
        "as" => TokenType::T_AS,
        "abstract" => TokenType::T_ABSTRACT,
        "extends" => TokenType::T_EXTENDS,
        "final" => TokenType::T_FINAL,
        "implements" => TokenType::T_IMPLEMENTS,
        "interface" => TokenType::T_INTERFACE,
        "trait" => TokenType::T_TRAIT,
        "clone" => TokenType::T_CLONE,
        "instanceof" => TokenType::T_INSTANCEOF,
        "do" => TokenType::T_DO,
        "include" => TokenType::T_INCLUDE,
        "include_once" => TokenType::T_INCLUDE_ONCE,
        "require" => TokenType::T_REQUIRE,
        "require_once" => TokenType::T_REQUIRE_ONCE,
        _ => TokenType::T_STRING,
    }
}

/// Check if a string is a reserved keyword
pub fn is_keyword(keyword: &str) -> bool {
    matches!(
        keyword,
        "if" | "else" | "elseif" | "while" | "for" | "foreach" | "function" |
        "class" | "return" | "echo" | "array" | "new" | "static" | "public" |
        "private" | "protected" | "const" | "break" | "continue" | "switch" |
        "case" | "default" | "try" | "catch" | "throw" | "namespace" | "use" |
        "as" | "abstract" | "extends" | "final" | "implements" | "interface" |
        "trait" | "clone" | "instanceof" | "do" | "callable" | "insteadof" |
        "include" | "include_once" | "require" | "require_once"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyword_to_token() {
        assert_eq!(keyword_to_token("if"), TokenType::T_IF);
        assert_eq!(keyword_to_token("while"), TokenType::T_WHILE);
        assert_eq!(keyword_to_token("function"), TokenType::T_FUNCTION);
        assert_eq!(keyword_to_token("echo"), TokenType::T_ECHO);
        assert_eq!(keyword_to_token("unknown"), TokenType::T_STRING);
    }

    #[test]
    fn test_is_keyword() {
        assert!(is_keyword("if"));
        assert!(is_keyword("while"));
        assert!(is_keyword("function"));
        assert!(!is_keyword("notakeyword"));
        assert!(!is_keyword("myVariable"));
    }
}
