use anyhow::Result;
use crate::tokenize::{Tokenizer, Token};

#[derive(thiserror::Error, Debug)]
pub enum ParserError {
    #[error("Syntax error: {0}")]
    SyntaxError(String),
    #[error("Undefined variable: {0}")]
    UndefinedVariable(String),
    #[error("Invalid operator: {0}")]
    InvalidOperator(String),
    #[error("Unexpected keyword: {0}")]
    UnexpectedKeyword(String),
}

pub trait Parser {
    fn peek_keyword(&self) -> Result<String>;
    fn peek_keyword_matches(&self, options: &[&str]) -> bool;
    fn peek_symbol_matches(&self, expected: &str) -> bool;
    fn peek_symbol_matches_choices(&self, choices: &[&str]) -> bool;
    fn peek_is_identifier(&self) -> bool;
    fn peek_is_int_const(&self) -> bool;
    fn peek_is_string_const(&self) -> bool;
    fn peek_next_char(&mut self) -> &str;
    fn expect_keyword(&mut self, expected: &str) -> Result<()>;
    fn expect_keyword_choices(&mut self, choices: &[&str]) -> Result<String>;
    fn expect_symbol(&mut self, expected: &str) -> Result<()>;
    fn get_symbol(&mut self) -> Result<String>;
    fn expect_identifier(&mut self) -> Result<String>;
    fn get_int_const(&mut self) -> Result<usize>;
    fn get_string_const(&mut self) -> Result<String>;
    fn get_keyword(&mut self) -> Result<String>;
    fn parse_type(&mut self) -> Result<String>;
}

pub struct JackParser<T: Tokenizer> {
    source: T,
    token: Option<Token>,
}

impl<T: Tokenizer> JackParser<T> {
    pub fn new(mut source: T) -> Self {
        let token = source.token();
        Self {
            source,
            token,
        }
    }

    fn next_token(&mut self) {
        self.token = self.source.token();
    }
}

impl<T: Tokenizer> Parser for JackParser<T> {
    fn peek_keyword(&self) -> Result<String> {
        match &self.token {
            Some(Token::Keyword(kw)) => Ok(kw.clone()),
            _ => Err(ParserError::SyntaxError("Expected keyword".to_string()).into()),
        }
    }

    fn peek_keyword_matches(&self, options: &[&str]) -> bool {
        if let Some(Token::Keyword(kw)) = &self.token {
            options.contains(&kw.as_str())
        } else {
            false
        }
    }

    fn peek_symbol_matches(&self, expected: &str) -> bool {
        matches!(&self.token, Some(Token::Symbol(c)) if *c == *expected)
    }

    fn peek_symbol_matches_choices(&self, choices: &[&str]) -> bool {
        if let Some(Token::Symbol(ref c)) = self.token {
            choices.contains(&c.as_str())
        } else {
            false
        }
    }

    fn peek_is_identifier(&self) -> bool {
        matches!(self.token, Some(Token::Identifier(_)))
    }

    fn peek_is_int_const(&self) -> bool {
        matches!(self.token, Some(Token::IntConst(_)))
    }

    fn peek_is_string_const(&self) -> bool {
        matches!(self.token, Some(Token::StringConst(_)))
    }

    fn peek_next_char(&mut self) -> &str {
        match &self.token {
            Some(Token::Symbol(c)) => &c,
            _ => " ", // Если там не символ или конец файла
        }
    }

    fn expect_keyword(&mut self, expected: &str) -> Result<()> {
        match &self.token {
            Some(Token::Keyword(kw)) if kw == expected => {
                self.next_token();
                Ok(())
            }
            _ => Err(
                ParserError::SyntaxError(format!("Expected keyword '{}'", expected)).into()
                ),
        }
    }

    fn expect_keyword_choices(&mut self, choices: &[&str]) -> Result<String> {
        match &self.token {
            Some(Token::Keyword(kw)) if choices.contains(&kw.as_str()) => {
                let keyword = kw.clone();
                self.next_token();
                Ok(keyword)
            }
            _ => Err(
                ParserError::SyntaxError(
                    format!("Expected one of keywords {:?}", choices)).into()
                ),
        }
    }

    fn expect_symbol(&mut self, expected: &str) -> Result<()> {
        match &self.token {
            Some(Token::Symbol(c)) if c == expected => {
                self.next_token();
                Ok(())
            }
            _ => Err(
                ParserError::SyntaxError(
                    format!("Expected symbol '{}'", expected)).into()
                ),
        }
    }

    fn get_symbol(&mut self) -> Result<String> {
        match &self.token {
            Some(Token::Symbol(c)) => {
                let symbol = c.clone();
                self.next_token();
                Ok(symbol)
            }
            _ => Err(
                ParserError::SyntaxError("Expected symbol".to_string()).into()
                ),
        }
    }

    fn expect_identifier(&mut self) -> Result<String> {
        match &self.token {
            Some(Token::Identifier(id)) => {
                let identifier = id.clone();
                self.next_token();
                Ok(identifier)
            }
            _ => Err(
                ParserError::SyntaxError("Expected identifier".to_string()).into()
                ),
        }
    }

    fn get_int_const(&mut self) -> Result<usize> {
        match &self.token {
            Some(Token::IntConst(val)) => {
                let int_const = *val;
                self.next_token();
                Ok(int_const)
            }
            _ => Err(
                ParserError::SyntaxError("Expected integer constant".to_string()).into()
                ),
        }
    }

    fn get_string_const(&mut self) -> Result<String> {
        match &self.token {
            Some(Token::StringConst(s)) => {
                let string_const = s.clone();
                self.next_token();
                Ok(string_const)
            }
            _ => Err(
                ParserError::SyntaxError("Expected string constant".to_string()).into()
                ),
        }
    }

    fn get_keyword(&mut self) -> Result<String> {
        match &self.token {
            Some(Token::Keyword(kw)) => {
                let keyword = kw.clone();
                self.next_token();
                Ok(keyword)
            }
            _ => Err(ParserError::SyntaxError("Expected keyword".to_string()).into()),
        }
    }

    fn parse_type(&mut self) -> Result<String> {
        if self.peek_keyword_matches(&["int", "char", "boolean", "void"]) {
            self.get_keyword()
        } else if self.peek_is_identifier() {
            self.expect_identifier()
        } else {
            Err(ParserError::SyntaxError("Expected data type".to_string()).into())
        }
    }
}
