use std::io::Write;
use anyhow::Result;
use quick_xml::Writer;
use quick_xml::events::{BytesText, BytesEnd, BytesStart, Event};

pub trait Serializer {
    fn write_name(&mut self, name: &str) -> Result<()>;
    fn end_name(&mut self, name: &str) -> Result<()>;
    fn write_data(&mut self, data: &str) -> Result<()>;
    fn write_node(&mut self, name: &str, value: &str) -> Result<()>;
}

pub struct XmlSerializer<W: Write> {
    writer: Writer<W>,
}

impl<W: Write> XmlSerializer<W> {
    pub fn new(inner: W) -> Self {
        Self {
            writer: Writer::new_with_indent(inner, b' ', 2),
        }
    }
}

impl<W: Write> Serializer for XmlSerializer<W> {
    fn write_name(&mut self, name: &str) -> Result<()> {
        self.writer.write_event(Event::Start(BytesStart::new(name)))?;
        Ok(())
    }

    fn end_name(&mut self, name: &str) -> Result<()> {
        self.writer.write_event(Event::End(BytesEnd::new(name)))?;
        Ok(())
    }

    fn write_data(&mut self, data: &str) -> Result<()> {
        self.writer.write_event(Event::Text(BytesText::new(data)))?;
        Ok(())
    }

    fn write_node(&mut self, name: &str, value: &str) -> Result<()> {
        self.write_name(name)?;
        self.write_data(value)?;
        self.end_name(name)?;
        Ok(())
    }
}
