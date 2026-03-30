use std::io::{BufRead, Lines};
use std::io::BufReader;
use std::fs::File;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstrType {
    AInstruction,
    CInstruction,
    LInstruction,
    NoInstruction,
}

#[derive(Debug)]
pub struct Parser {
    lines: Lines<BufReader<File>>,
    curr: Option<String>,
}

impl Parser {
    pub fn from_path<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let f = File::open(path)?;
        Ok(Parser { lines: std::io::BufReader::new(f).lines(), curr: None })
    }

    // advance to next meaningful command; returns false if EOF
    pub fn advance(&mut self) -> bool {
        while let Some(ln) = self.lines.next() {
            match ln {
                Ok(mut s) => {
                    // strip comments
                    if let Some(idx) = s.find("//") {
                        s.truncate(idx);
                    }
                    // trim whitespace
                    let t = s.trim();
                    if t.is_empty() {
                        continue;
                    }
                    self.curr = Some(t.to_string());
                    return true;
                }
                Err(_) => continue,
            }
        }
        self.curr = None;
        false
    }

    pub fn instruction_type(&self) -> InstrType {
        if let Some(ref s) = self.curr {
            let first = s.chars().next().unwrap_or('\0');
            if first == '@' {
                InstrType::AInstruction
            } else if first == '(' && s.ends_with(')') {
                InstrType::LInstruction
            } else {
                InstrType::CInstruction
            }
        } else {
            InstrType::NoInstruction
        }
    }

    pub fn symbol(&self) -> Option<String> {
        match self.instruction_type() {
            InstrType::AInstruction => self.curr.as_ref().map(|s| s[1..].to_string()),
            InstrType::LInstruction => self.curr.as_ref().map(|s| s[1..s.len()-1].to_string()),
            _ => None,
        }
    }

    pub fn dest(&self) -> Option<String> {
        if self.instruction_type() != InstrType::CInstruction { return None; }
        self.curr.as_ref().and_then(|s| {
            if let Some(idx) = s.find('=') {
                Some(s[..idx].trim().to_string())
            } else {
                Some(String::new()) // null dest
            }
        })
    }

    pub fn comp(&self) -> Option<String> {
        if self.instruction_type() != InstrType::CInstruction { return None; }
        self.curr.as_ref().map(|s| {
            let without_dest = if let Some(idx) = s.find('=') { &s[idx+1..] } else { &s[..] };
            if let Some(idx) = without_dest.find(';') {
                without_dest[..idx].trim().to_string()
            } else {
                without_dest.trim().to_string()
            }
        })
    }

    pub fn jump(&self) -> Option<String> {
        if self.instruction_type() != InstrType::CInstruction { return None; }
        self.curr.as_ref().and_then(|s| {
            if let Some(idx) = s.find(';') {
                Some(s[idx+1..].trim().to_string())
            } else {
                Some(String::new()) // null jump
            }
        })
    }
}

