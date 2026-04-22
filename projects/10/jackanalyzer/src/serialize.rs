use std::io::{Write, BufRead};
use anyhow::Result;
use quick_xml::Writer;
use quick_xml::events::{BytesEnd, BytesStart, Event};
use crate::tokenize::JackTokenizer;
use crate::token::Token;

pub trait TokenSerializer {
    fn serialize_all<R: BufRead>(&mut self, tokenizer: JackTokenizer<R>) -> Result<()>;
}

pub struct XmlSerializer<W: Write> {
    writer: Writer<W>,
}

impl<W: Write> XmlSerializer<W> {
    pub fn new(inner: W) -> Self {
        Self {
            writer: Writer::new_with_indent(inner, b' ', 4),
        }
    }

    fn start_tag(&mut self, name: &str) -> Result<()> {
        self.writer.write_event(Event::Start(BytesStart::new(name)))?;
        Ok(())
    }

    fn end_tag(&mut self, name: &str) -> Result<()> {
        self.writer.write_event(Event::End(BytesEnd::new(name)))?;
        Ok(())
    }

    fn write_token(&mut self, token: &Token) -> Result<()> {
        token.write_to(&mut self.writer)?;
        Ok(())
    }
}

impl<W: Write> TokenSerializer for XmlSerializer<W> {

    fn serialize_all<R: BufRead>( &mut self, mut tokenizer: JackTokenizer<R>) -> Result<()> {
        self.start_tag("tokens")?;

        while tokenizer.advance()? {
            if let Some(token) = Token::from_tokenizer(&mut tokenizer) {
                self.write_token(&token)?;
            }
        }

        self.end_tag("tokens")?;
        Ok(())
    }
}
