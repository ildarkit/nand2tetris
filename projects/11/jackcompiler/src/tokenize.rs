use std::io::{self, BufRead};
use std::ops::Range;

const SYMBOLS: &[char] = &[
    '{', '}', '(', ')', '[', ']', '.', ',', ';', 
    '+', '-', '*', '/', '&', '|', '<', '>', '=', '~'
];

pub trait Tokenizer {
    fn token(&mut self) -> Option<Token>;
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Token {
    Keyword(String),
    Symbol(String),
    Identifier(String),
    IntegerConstant(u16),
    StringConstant(String),
    Invalid(String),
}

pub struct JackTokenizer<R: BufRead> {
    reader: R,
    data: String,
    current: Option<Range<usize>>,
    tokens: Vec<Range<usize>>,
    index: usize,
}

impl <R: BufRead> JackTokenizer<R> {

    pub fn new(reader: R) -> Self {
        Self {
            reader,
            data: String::new(),
            current: None,
            tokens: Vec::new(),
            index: 0,
        }
    }

    fn read_line(&mut self) -> io::Result<bool> {
        let mut multi_comment = false;
        self.data.clear();
        while self.reader.read_line(&mut self.data)? > 0 {
            if let Some(_) = self.data.find("/*") {
                multi_comment = true;
            }
            if multi_comment { 
                if let Some(_) = self.data.find("*/") {
                    multi_comment = false;
                }
                self.data.clear();
                continue;
            };

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

    fn get_tokens(&mut self) {
        let mut word_start: Option<usize> = None;
        let mut string_const = false;
        let mut chars = self.data.char_indices();

        while let Some((idx, ch)) = chars.next() {
            if string_const {
                if ch == '"' {
                    if let Some(start) = word_start.take() {
                        self.tokens.push(start..idx + 1);
                    }
                    string_const = false;
                }
                continue;
            }

            if ch == '"' {
                if let Some(start) = word_start.take() {
                    self.tokens.push(start..idx);
                }
                word_start = Some(idx);
                string_const = true;
                continue;
            }

            if ch.is_whitespace() || SYMBOLS.contains(&ch) {
                if let Some(start) = word_start.take() {
                    self.tokens.push(start..idx);
                }
                if SYMBOLS.contains(&ch) {
                    self.tokens.push(idx..idx + ch.len_utf8());
                }
            } else if word_start.is_none() {
                word_start = Some(idx);
            }
        }

        if let Some(start) = word_start {
            self.tokens.push(start..self.data.len());
        }
    }

    fn next_token(&mut self) -> bool {
        if let Some(range) = self.tokens.get(self.index) {
            self.current = Some(range.clone());
            self.index += 1;
            return true;
        } else {
            self.data.clear();
        }
        self.current = None;
        false
    }

    fn current_token(&self) -> &str {
        let range = self.current.as_ref()
            .expect("Сначала нужно вызвать метод token_type");
        &self.data[range.clone()]
    }

    fn token_type(&mut self) -> Token {
        while !self.next_token() { }

        let token = self.current_token();
        match token {
            "class" | "constructor" | "function" | "method" | "field" | "static" |
            "var" | "int" | "char" | "boolean" | "void" | "true" | "false" | "null" |
            "this" | "let" | "do" | "if" | "else" | "while" | "return" => {
                Token::Keyword(token.to_string())
            },
            _ if token.len() == 1 && SYMBOLS.contains(&token.chars().next().unwrap()) => {
                Token::Symbol(token.to_string())
            },
            _ if token.starts_with('"') && token.ends_with('"') => {
                Token::StringConstant(token.trim_matches('"').to_string())
            },
            _ if token.parse::<u16>().map_or(false, |n| (0..=32767).contains(&n)) => {
                Token::IntegerConstant(token.parse().unwrap())
            },
            _ if !token.is_empty() && !token.starts_with(|c: char| c.is_ascii_digit()) 
                 && token.chars().all(|c| c.is_alphanumeric() || c == '_') => {
                Token::Identifier(token.to_string())
            },
            _ => Token::Invalid(token.to_string()),
        }
    }

    fn advance(&mut self) -> io::Result<bool> {
        if self.data.is_empty() {
            if !self.read_line()? {
                return Ok(false);
            }
            self.index = 0;
            self.tokens.clear();
            self.get_tokens();
        }
        Ok(true)
    }
}

impl<R: BufRead> Tokenizer for JackTokenizer<R> {
    fn token(&mut self) -> Option<Token> {
        match self.advance() {
            Ok(true) => Some(self.token_type()),
            Ok(false) => None,
            Err(e) => {
                eprintln!("Ошибка чтения потока токенов: {}", e);
                None
            }
        }
    }
}
