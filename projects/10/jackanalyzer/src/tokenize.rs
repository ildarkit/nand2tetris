use std::io::BufRead;
use std::ops::Range;
use anyhow::Result;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TokenType {
    Keyword,
    Symbol,
    Identifier,
    IntConst,
    StringConst,
    EOF,
    Invalid(String),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Keyword {
    Class,
    Function,
    Constructor,
    Int,
    Boolean,
    Field,
    Let,
    Do,
    If,
    Else,
    While,
    Return,
    True,
    False,
    Null,
    This,
}

pub struct JackTokenizer<R: BufRead> {
    reader: R,
    data: String,
    tokens: Vec<Range<usize>>,
    current: usize,
}

impl <R: BufRead> JackTokenizer<R> {

    pub fn new(reader: R) -> Self {
        Self {
            reader,
            data: String::new(),
            tokens: Vec::new(),
            current: 0,
        }
    }

    fn read_line(&mut self) -> Result<bool> {
        self.data.clear();
        while self.reader.read_line(&mut self.data)? > 0 {
            if let Some(i) = self.data.find("//") {
                self.data.truncate(i);
            }
            let trimmed = self.data.trim();
            if !trimmed.is_empty() {
                return Ok(true);
            }
            self.data.clear();
        }
        Ok(false)
    }

    pub fn advance(&mut self) -> Result<bool> {
        if self.data.is_empty() {
            self.read_line()?;
            let data_ptr = self.data.as_ptr() as usize;
            self.current = 0;
            self.tokens.clear();
            let new_tokens = self.data
                .split_whitespace()
                .map(|word| {
                    let start = word.as_ptr() as usize - data_ptr;
                    start..(start + word.len())
                });
            self.tokens.extend(new_tokens);
        }
        Ok(true)
    }

    fn next_token(&mut self) -> Option<&str> {
        if let Some(range) = self.tokens.get(self.current) {
            self.current += 1;
            return Some(&self.data[range.clone()]);
        }
        None
    }

    pub fn token_type(&mut self) -> TokenType {
        let Some(token) = self.next_token() else {
            return TokenType::EOF;
        };

        match token {
            "class" | "constructor" | "function" | "method" | "field" | "static" |
                "var" | "int" | "char" | "boolean" | "void" | "true" | "false" | "null" |
                "this" | "let" | "do" | "if" | "else" | "while" | "return" => {
                TokenType::Keyword
            },
            "{" | "}" | "(" | ")" | "[" | "]" | ". " | ", " | "; " | "+" | "-" | "*" | "/" |
                "&" | "|" | "<" | ">" | "=" | "~" => {
                TokenType::Symbol
            },
            _ if (token.starts_with('"') && token.ends_with('"')) => {
                TokenType::StringConst
            },
            _ if token.parse::<i32>().map_or(false, |n| (0..=32767).contains(&n)) => {
                TokenType::IntConst
            },
            _ if !token.is_empty() && !token.starts_with(|c: char| c.is_ascii_digit()) 
                 && token.chars().all(|c| c.is_alphanumeric() || c == '_') => {
                TokenType::Identifier
            },
            token => TokenType::Invalid(token.to_string()),
        }
    }

    pub fn keyword(&self) -> &str {
        todo!();
    }

    pub fn symbol(&self) -> &str {
        todo!();
    }

    pub fn identifier(&self) -> &str {
        todo!();
    }

    pub fn int_val(&self) -> &str {
        todo!();
    }

    pub fn string_val(&self) -> &str {
        todo!();
    }
}
