// src/codewriter.rs

use std::io;
use std::fs::File;
use std::path::Path;
use crate::parser::CommandType;
use crate::hack::HackWriter;

pub struct CodeWriter {
    writer: HackWriter,
    file_name: String,
    label_counter: u32,
    current_caller: Option<String>,
    current_calls: u32,
}

impl CodeWriter {
    pub fn new(path: &Path, file_name: &str) -> io::Result<Self> {
        let out = File::create(path)?;
        Ok(
            Self {
                writer: HackWriter::new(out),
                file_name: file_name.to_string(),
                label_counter: 0,
                current_caller: None,
                current_calls: 0,
            }
        )
    }

    pub fn bootstrap(&mut self, entrypoint: &str) -> io::Result<()> {
        self.writer.bootstrap()?;
        self.write_call(entrypoint, 0)?;
        Ok(())
    }

    fn replace_current_caller(&mut self, func: &str) {
        self.current_caller.replace(func.to_string());
        self.current_calls = 0;
    }

    fn caller_name(&self) -> &str {
        if let Some(ref caller) = self.current_caller {
            caller
        } else {
            &self.file_name
        }
    }

    pub fn set_file_name(&mut self, filename: &str) {
        self.file_name = filename.to_string();
    }

    pub fn write_label(&mut self, label: &str) -> io::Result<()> {
        let caller_name = self.caller_name().to_string();
        self.writer.write_label(&caller_name, label)?;
        Ok(())
    }

    pub fn write_goto(&mut self, label: &str) -> io::Result<()> {
        let caller_name = self.caller_name().to_string();
        self.writer.write_goto(&caller_name, label)?;
        Ok(())
    }

    pub fn write_if(&mut self, label: &str) -> io::Result<()> {
        let caller_name = self.caller_name().to_string();
        self.writer.write_if(&caller_name, label)?;
        Ok(())
    }

    pub fn write_function(&mut self, function_name: &str, nvars: u32) -> io::Result<()> {
        self.replace_current_caller(function_name);
        self.writer.write_function(function_name, nvars)?;
        Ok(())
    }

    pub fn write_call(&mut self, function_name: &str, nargs: u32) -> io::Result<()> {
        let caller_name = self.caller_name().to_string();
        self.writer.write_call(function_name, nargs, &caller_name, self.current_calls)?;
        self.current_calls += 1;
        Ok(())
    }

    pub fn write_return(&mut self) -> io::Result<()> {
        self.writer.write_return(&self.current_caller.clone().unwrap())?;
        Ok(())
    }

    pub fn write_arithmetic(&mut self, cmd: &str) -> io::Result<()> {
        self.writer.write_arithmetic(cmd, &self.file_name, self.label_counter)?;
        self.label_counter += 1;
        Ok(())
    }

    pub fn write_push_pop(
        &mut self,
        ctype: CommandType,
        segment: &str,
        index: u32) -> io::Result<()>
    {
        match ctype {
            CommandType::CPush => {
                self.writer.write_push_segment(&self.file_name, segment, index)?;
            }
            CommandType::CPop => {
                self.writer.write_pop_segment(&self.file_name, segment, index)?;
            }
            _ => unreachable!(),
        }
        Ok(())
    }

    pub fn close(&mut self) -> io::Result<()> {
        self.writer.close()
    }
}

