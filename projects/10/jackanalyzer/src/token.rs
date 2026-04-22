use std::io::{Write, BufRead};
use anyhow::Result;
use quick_xml::Writer;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use crate::jacktokenizer::{JackTokenizer, TokenType};

pub enum Token<'a> {
    Keyword(&'a str),
    Symbol(&'a str),
    Identifier(&'a str),
    IntConst(&'a str),
    StringConst(&'a str),
}

impl<'a> Token<'a> {
    pub fn from_tokenizer<R: BufRead>(t: &'a mut JackTokenizer<R>) -> Option<Self> {
        match t.token_type() {
            TokenType::Keyword => Some(Token::Keyword(t.keyword())),
            TokenType::Symbol => Some(Token::Symbol(t.symbol())),
            TokenType::Identifier => Some(Token::Identifier(t.identifier())),
            TokenType::IntConst => Some(Token::IntConst(t.int_val())),
            TokenType::StringConst => Some(Token::StringConst(t.string_val())),
            TokenType::EOF => None,
            TokenType::Invalid(token) => {
                eprintln!("Неверный токен: {}", token);
                None
            }
        }
    }

    pub fn write_to<W: Write>( &self, writer: &mut Writer<W>) -> Result<()> {
        let (tag, value_str);
        match self {
            Self::Keyword(v) => {
                tag = "keyword";
                value_str = v;
            }
            Self::Symbol(v) => {
                tag = "symbol";
                value_str = v;
            }
            Self::Identifier(v) => {
                tag = "identifier";
                value_str = v;
            }
            Self::IntConst(v) => { 
                tag = "integerConstant"; 
                value_str = v; 
            }
            Self::StringConst(v) => {
                tag = "stringConstant";
                value_str = v;
            }
        };

        writer.write_event(Event::Start(BytesStart::new(tag)))?;
        writer.write_event(Event::Text(BytesText::new(value_str)))?;
        writer.write_event(Event::End(BytesEnd::new(tag)))?;
        
        Ok(())
    }
}
