use std::io::{Write, BufRead};
use anyhow::Result;
use quick_xml::Writer;
use quick_xml::events::{BytesText, BytesEnd, BytesStart, Event};
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
}

impl<W: Write> TokenSerializer for XmlSerializer<W> {

    fn serialize_all<R: BufRead>( &mut self, mut tokenizer: JackTokenizer<R>) -> Result<()> {
        self.start_tag("tokens")?;
        while tokenizer.advance()? {
            if let Some(token) = Token::from_tokenizer(&mut tokenizer) {
                let (tag, value) = token.unpack();
                self.writer.write_event(Event::Start(BytesStart::new(tag)))?;
                self.writer.write_event(Event::Text(BytesText::new(value)))?;
                self.writer.write_event(Event::End(BytesEnd::new(tag)))?;
            }
        }
        self.end_tag("tokens")?;
        Ok(())
    }
}
