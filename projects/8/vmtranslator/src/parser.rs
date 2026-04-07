// src/parser.rs
use std::io::{self, BufRead};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CommandType {
    CArithmetic,
    CPush,
    CPop,
    CGoto,
    CIfGoto,
    CLabel,
    CFunction,
    CCall,
    CReturn,
}

pub struct Parser<R: BufRead> {
    reader: R,
    current: Option<String>,
}

impl<R: BufRead> Parser<R> {
    pub fn new(reader: R) -> Self {
        Self { reader, current: None }
    }

    // Read next non-empty, non-comment line; return true if a command is available
    pub fn advance(&mut self) -> io::Result<bool> {
        let mut line = String::new();
        while self.reader.read_line(&mut line)? > 0 {
            // strip comments
            if let Some(i) = line.find("//") {
                line.truncate(i);
            }
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                self.current = Some(trimmed.to_string());
                return Ok(true);
            }
            line.clear();
        }
        self.current = None;
        Ok(false)
    }

    pub fn command_type(&self) -> CommandType {
        let s = self.current.as_ref().unwrap();
        let first = s.split_whitespace().next().unwrap();
        match first {
            "push" => CommandType::CPush,
            "pop" => CommandType::CPop,
            "add" | "sub" | "neg" | "eq" | "gt" | "lt" | "and" | "or" | "not" => CommandType::CArithmetic,
            "goto"  => CommandType::CGoto,
            "if-goto" => CommandType::CIfGoto,
            "label" => CommandType::CLabel,
            "function" => CommandType::CFunction,
            "call" => CommandType::CCall,
            "return" => CommandType::CReturn,
            _ => panic!("unsupported command: {}", first),
        }
    }

    // returns Some(&str) for arg1 (command name for arithmetic)
    pub fn arg1(&self) -> Option<&str> {
        let s = self.current.as_ref()?;
        let parts: Vec<&str> = s.split_whitespace().collect();
        match self.command_type() {
            CommandType::CArithmetic |
            CommandType::CReturn => Some(parts[0]),
            CommandType::CPush |
            CommandType::CPop |
            CommandType::CGoto | 
            CommandType::CIfGoto |
            CommandType::CLabel |
            CommandType::CFunction |
            CommandType::CCall => Some(parts[1]),
        }
    }

    pub fn arg2(&self) -> Option<u32> {
        let s = self.current.as_ref()?;
        let parts: Vec<&str> = s.split_whitespace().collect();
        match self.command_type() {
            CommandType::CPush |
            CommandType::CPop |
            CommandType::CFunction |
            CommandType::CCall => parts.get(2).and_then(|p| p.parse().ok()),
            _ => None,
        }
    }
}

