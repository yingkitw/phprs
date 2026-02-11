//! Token definitions and structures

use crate::engine::types::PhpString;

/// PHP token types
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum TokenType {
    // Special tokens
    T_EOF = 0,
    T_OPEN_TAG = 316,
    T_OPEN_TAG_WITH_ECHO = 317,
    T_CLOSE_TAG = 318,
    T_ECHO = 319,
    T_WHITESPACE = 376,
    T_COMMENT = 377,
    T_DOC_COMMENT = 378,
    T_INLINE_HTML = 379,

    // Keywords
    T_ABSTRACT = 321,
    T_ARRAY = 322,
    T_AS = 323,
    T_BREAK = 324,
    T_CALLABLE = 325,
    T_CASE = 326,
    T_CATCH = 327,
    T_CLASS = 328,
    T_CLONE = 329,
    T_CONST = 330,
    T_CONTINUE = 331,
    T_DECLARE = 332,
    T_DEFAULT = 333,
    T_DO = 334,
    T_ELSE = 335,
    T_ELSEIF = 336,
    T_ENDDECLARE = 337,
    T_ENDFOR = 338,
    T_ENDFOREACH = 339,
    T_ENDIF = 340,
    T_ENDSWITCH = 341,
    T_ENDWHILE = 342,
    T_EXTENDS = 343,
    T_FINAL = 344,
    T_FINALLY = 345,
    T_FOR = 346,
    T_FOREACH = 347,
    T_FUNCTION = 348,
    T_FN = 349,
    T_GLOBAL = 350,
    T_GOTO = 351,
    T_IF = 352,
    T_IMPLEMENTS = 353,
    T_INCLUDE = 354,
    T_INCLUDE_ONCE = 355,
    T_INSTANCEOF = 356,
    T_INSTEADOF = 357,
    T_INTERFACE = 358,
    T_NAMESPACE = 359,
    T_NEW = 360,
    T_PRIVATE = 361,
    T_PROTECTED = 362,
    T_PUBLIC = 363,
    T_RETURN = 364,
    T_STATIC = 365,
    T_SWITCH = 366,
    T_THROW = 367,
    T_TRAIT = 368,
    T_TRY = 369,
    T_UNSET = 370,
    T_USE = 371,
    T_VAR = 372,
    T_WHILE = 373,
    T_YIELD = 374,
    T_YIELD_FROM = 375,
    T_REQUIRE = 380,
    T_REQUIRE_ONCE = 381,

    // Operators and punctuation
    T_PLUS = 43,                      // +
    T_MINUS = 45,                     // -
    T_MUL = 42,                       // *
    T_DIV = 47,                       // /
    T_MOD = 37,                       // %
    T_POW = 94,                       // ^
    T_CONCAT = 46,                    // .
    T_EQUAL = 61,                     // =
    T_PLUS_EQUAL = 277,               // +=
    T_MINUS_EQUAL = 278,              // -=
    T_MUL_EQUAL = 279,                // *=
    T_DIV_EQUAL = 280,                // /=
    T_MOD_EQUAL = 281,                // %=
    T_CONCAT_EQUAL = 282,             // .=
    T_AND_EQUAL = 283,                // &=
    T_OR_EQUAL = 284,                 // |=
    T_XOR_EQUAL = 285,                // ^=
    T_SL_EQUAL = 286,                 // <<=
    T_SR_EQUAL = 287,                 // >>=
    T_COALESCE_EQUAL = 288,           // ??=
    T_BOOLEAN_AND = 289,              // &&
    T_BOOLEAN_OR = 290,               // ||
    T_BOOLEAN_NOT = 291,              // !
    T_IS_EQUAL = 292,                 // ==
    T_IS_NOT_EQUAL = 293,             // !=
    T_IS_IDENTICAL = 294,             // ===
    T_IS_NOT_IDENTICAL = 295,         // !==
    T_SPACESHIP = 296,                // <=>
    T_IS_SMALLER_OR_EQUAL = 297,      // <=
    T_IS_GREATER_OR_EQUAL = 298,      // >=
    T_SL = 299,                       // <<
    T_SR = 300,                       // >>
    T_INC = 301,                      // ++
    T_DEC = 302,                      // --
    T_OBJECT_OPERATOR = 303,          // ->
    T_DOUBLE_ARROW = 304,             // =>
    T_NULLSAFE_OBJECT_OPERATOR = 305, // ?->
    T_NS_SEPARATOR = 306,             // \
    T_ELLIPSIS = 307,                 // ...
    T_COALESCE = 308,                 // ??
    T_AMPERSAND_FOLLOWED_BY_VAR_OR_VARARG = 309,
    T_AMPERSAND_NOT_FOLLOWED_BY_VAR_OR_VARARG = 310,
    T_PAAMAYIM_NEKUDOTAYIM = 400,     // :: (double colon / scope resolution)

    // Literals
    T_LNUMBER = 311,
    T_DNUMBER = 312,
    T_STRING = 313,
    T_VARIABLE = 314,
    T_CONSTANT_ENCAPSED_STRING = 315,
    T_ENCAPSED_AND_WHITESPACE = 320,

    // Other
    T_ERROR = 999,
}

/// Token structure
#[derive(Debug, Clone)]
pub struct Token {
    pub token_type: TokenType,
    pub value: Option<PhpString>,
    pub lineno: u32,
    pub offset: usize,
}

impl Token {
    pub fn new(
        token_type: TokenType,
        value: Option<PhpString>,
        lineno: u32,
        offset: usize,
    ) -> Self {
        Self {
            token_type,
            value,
            lineno,
            offset,
        }
    }

    pub fn eof(lineno: u32, offset: usize) -> Self {
        Self {
            token_type: TokenType::T_EOF,
            value: None,
            lineno,
            offset,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::string::string_init;

    #[test]
    fn test_token_creation() {
        let token = Token::new(
            TokenType::T_ECHO,
            Some(string_init("echo", false)),
            1,
            0,
        );
        assert_eq!(token.token_type, TokenType::T_ECHO);
        assert_eq!(token.lineno, 1);
        assert_eq!(token.offset, 0);
    }

    #[test]
    fn test_eof_token() {
        let token = Token::eof(42, 100);
        assert_eq!(token.token_type, TokenType::T_EOF);
        assert!(token.value.is_none());
    }
}
