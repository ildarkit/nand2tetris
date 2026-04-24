use std::io::BufRead;
use crate::tokenize::{JackTokenizer, TokenType};

pub enum Token<'a> {
    Keyword(&'a str, &'a str),
    Symbol(&'a str, &'a str),
    Identifier(&'a str, &'a str),
    IntConst(&'a str, &'a str),
    StringConst(&'a str, &'a str),
}

impl<'a> Token<'a> {
    pub fn from_tokenizer<R: BufRead>(t: &'a mut JackTokenizer<R>) -> Option<Self> {
        match t.token_type() {
            TokenType::Keyword     => Some(Token::Keyword("keyword", t.keyword())),
            TokenType::Symbol      => Some(Token::Symbol("symbol", t.symbol())),
            TokenType::Identifier  => Some(Token::Identifier("identifier", t.identifier())),
            TokenType::IntConst    => Some(Token::IntConst("integerConstant", t.int_val())),
            TokenType::StringConst => {
                Some(Token::StringConst("stringConstant",
                        t.string_val().trim_matches('"'))
                    )
            },
            TokenType::EOF => None,
            TokenType::Invalid(token) => {
                eprintln!("Неверный токен: {}", token);
                None
            }
        }
    }

    pub fn unpack(&self) -> (&'a str, &'a str) {
        match self {
            Self::Keyword(tag, val) | 
            Self::Symbol(tag, val) | 
            Self::Identifier(tag, val) | 
            Self::IntConst(tag, val) | 
            Self::StringConst(tag, val) => (tag, val),
        }
    }
}
