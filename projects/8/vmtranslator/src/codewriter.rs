// src/codewriter.rs
use std::iter::zip;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use crate::parser::CommandType;

pub struct CodeWriter {
    out: File,
    file_name: String,
    label_counter: u32,
    call_stack: Vec<String>,
    current_calls: Vec<u32>,
}

impl CodeWriter {
    pub fn new(path: &Path, file_name: &str) -> io::Result<Self> {
        let out = File::create(path)?;
        Ok(
            Self {
                out,
                file_name: file_name.to_string(),
                label_counter: 0,
                call_stack: vec![],
                current_calls: vec![],
            }
        )
    }

    fn push_call_stack(&mut self, func: &str) {
        self.call_stack.push(func.to_string());
        self.current_calls.push(0);
    }

    fn pop_call_stack(&mut self) -> Option<String> {
        self.current_calls.pop();
        self.call_stack.pop()
    }

    fn return_label(&self) -> String {
        let caller = if let Some(caller) = self.call_stack.last() {
            caller
        } else {
            self.file_name.as_str()
        };
        let index = if self.current_calls.len() > 0 {
            self.current_calls[self.current_calls.len() - 1]
        } else { 0 };
        format!("{}$ret{}", caller, index)
    }

    fn pop_stack(&mut self) -> io::Result<()> {
        writeln!(self.out, "@SP")?;
        writeln!(self.out, "AM=M-1")?; // SP--, A=SP
        writeln!(self.out, "D=M")?;    // D = y
        writeln!(self.out, "A=A-1")?; // A = SP-1 (x)
        Ok(())
    }

    fn push_stack(&mut self) -> io::Result<()> {
        writeln!(self.out, "@SP")?;
        writeln!(self.out, "A=M")?;
        writeln!(self.out, "M=D")?;
        writeln!(self.out, "@SP")?;
        writeln!(self.out, "M=M+1")?;
        Ok(())
    }

    fn end_asm(&mut self) -> io::Result<()> {
        writeln!(self.out, "(END)")?;
        writeln!(self.out, "@END")?;
        writeln!(self.out, "0;JMP")?;
        Ok(())
    }

    pub fn set_file_name(&mut self, filename: &str) {
        self.file_name = filename.to_string();
    }

    pub fn write_label(&mut self, label: &str) -> io::Result<()> {
        writeln!(self.out, "({}${})", self.file_name, label)?;
        Ok(())
    }

    pub fn write_goto(&mut self, label: &str) -> io::Result<()> {
        writeln!(self.out, "// goto {}", label)?;
        writeln!(self.out, "@{}${}", self.file_name, label)?;
        writeln!(self.out, "0;JMP")?;
        Ok(())
    }

    pub fn write_if(&mut self, label: &str) -> io::Result<()> {
        writeln!(self.out, "// if-goto {}", label)?;
        writeln!(self.out, "@SP")?;
        writeln!(self.out, "AM=M-1")?; // SP--, A=SP
        writeln!(self.out, "D=M")?;    // D = y
        writeln!(self.out, "@{}${}", self.file_name, label)?;
        writeln!(self.out, "D;JNE")?;
        Ok(())
    }

    pub fn write_function(&mut self, function_name: &str, nvars: u32) -> io::Result<()> {
        writeln!(self.out, "// function {} {}", function_name, nvars)?;
        self.push_call_stack(function_name);
        writeln!(self.out, "({})", function_name)?;
        writeln!(self.out, "@0")?;
        writeln!(self.out, "D=A")?;
        writeln!(self.out, "@LCL")?;
        writeln!(self.out, "A=M")?;
        for i in 0..nvars {
            if i > 0 {
                writeln!(self.out, "M=M+{}", i)?;
            }
            writeln!(self.out, "M=D")?;
        }
        Ok(())
    }

    pub fn write_call(&mut self, function_name: &str, nargs: u32) -> io::Result<()> {
        writeln!(self.out, "// call {} {}", function_name, nargs)?;
        writeln!(self.out, "@{}", self.return_label())?; // store segments address in stack
        writeln!(self.out, "D=A")?;
        self.push_stack()?;
        writeln!(self.out, "@LCL")?;
        writeln!(self.out, "D=A")?;
        self.push_stack()?;
        writeln!(self.out, "@ARG")?;
        writeln!(self.out, "D=A")?;
        self.push_stack()?;
        writeln!(self.out, "@THIS")?;
        writeln!(self.out, "D=A")?;
        self.push_stack()?;
        writeln!(self.out, "@THAT")?;
        writeln!(self.out, "D=A")?;
        self.push_stack()?;

        writeln!(self.out, "@SP")?; // ARG
        writeln!(self.out, "D=A")?;
        writeln!(self.out, "@ARG")?;
        writeln!(self.out, "AM=D-5-{}", nargs)?;

        writeln!(self.out, "@SP")?; // LCL
        writeln!(self.out, "D=A")?;
        writeln!(self.out, "@LCL")?;
        writeln!(self.out, "A=M")?;
        writeln!(self.out, "M=D")?;

        self.write_goto(function_name)?; // goto function

        writeln!(self.out, "({})", self.return_label())?;
        let len = self.current_calls.len();
        self.current_calls[len - 1] += 1;
        Ok(())
    }

    pub fn write_return(&mut self) -> io::Result<()> {
        if let Some(func) = self.pop_call_stack() {
            writeln!(self.out, "// return from {}", func)?;
        }

        writeln!(self.out, "@LCL")?; // local segment address
        writeln!(self.out, "D=A")?;
        writeln!(self.out, "@R13")?;
        writeln!(self.out, "M=D")?;

        writeln!(self.out, "@SP")?; // return to ARG 0
        writeln!(self.out, "AM=M-1")?;
        writeln!(self.out, "D=M")?;
        writeln!(self.out, "@ARG")?;
        writeln!(self.out, "M=D")?;

        writeln!(self.out, "@ARG")?; // SP
        writeln!(self.out, "D=A+1")?;
        writeln!(self.out, "@SP")?; 
        writeln!(self.out, "A=D")?;

        let index = vec![1, 2, 3, 4]; // restore segment address
        let segment = vec!["THAT", "THIS", "ARG", "LCL"];
        for (i, seg) in zip(index, segment) {
            writeln!(self.out, "@R13")?;
            writeln!(self.out, "D=M-{}", i)?;
            writeln!(self.out, "@{}", seg)?;
            writeln!(self.out, "M=D")?;
        }

        writeln!(self.out, "@R13")?; // goto return address
        writeln!(self.out, "A=M-5")?;
        writeln!(self.out, "@R13")?;
        writeln!(self.out, "0;JMP")?;
        Ok(())
    }

    pub fn write_arithmetic(&mut self, cmd: &str) -> io::Result<()> {
        match cmd {
            "add" => {
                writeln!(self.out, "// add")?;
                self.pop_stack()?;
                writeln!(self.out, "M=M+D")?; // x = x + y
            }
            "sub" => {
                writeln!(self.out, "// sub")?;
                self.pop_stack()?;
                writeln!(self.out, "M=M-D")?;
            }
            "neg" => {
                writeln!(self.out, "// neg")?;
                writeln!(self.out, "@SP")?;
                writeln!(self.out, "A=M-1")?;
                writeln!(self.out, "M=-M")?;
            }
            "and" => {
                writeln!(self.out, "// and")?;
                self.pop_stack()?;
                writeln!(self.out, "M=M&D")?;
            }
            "or" => {
                writeln!(self.out, "// or")?;
                self.pop_stack()?;
                writeln!(self.out, "M=M|D")?;
            }
            "not" => {
                writeln!(self.out, "// not")?;
                writeln!(self.out, "@SP")?;
                writeln!(self.out, "A=M-1")?;
                writeln!(self.out, "M=!M")?;
            }
            "eq" | "gt" | "lt" => {
                // comparison requires unique labels
                let label_true = format!("{}_TRUE_{}", self.file_name, self.label_counter);
                let label_end = format!("{}_END_{}", self.file_name, self.label_counter);
                self.label_counter += 1;
                writeln!(self.out, "// {}", cmd)?;
                self.pop_stack()?;
                writeln!(self.out, "D=M-D")?;
                writeln!(self.out, "@{}", label_true)?;
                match cmd {
                    "eq" => writeln!(self.out, "D;JEQ")?,
                    "gt" => writeln!(self.out, "D;JGT")?,
                    "lt" => writeln!(self.out, "D;JLT")?,
                    _ => unreachable!(),
                }
                writeln!(self.out, "@SP")?;
                writeln!(self.out, "A=M-1")?;
                writeln!(self.out, "M=0")?; // false
                writeln!(self.out, "@{}", label_end)?;
                writeln!(self.out, "0;JMP")?;
                writeln!(self.out, "({})", label_true)?;
                writeln!(self.out, "@SP")?;
                writeln!(self.out, "A=M-1")?;
                writeln!(self.out, "M=-1")?; // true
                writeln!(self.out, "({})", label_end)?;
            }
            _ => panic!("unsupported arithmetic: {}", cmd),
        }
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
                match segment {
                    "constant" => {
                        writeln!(self.out, "// push constant {}", index)?;
                        writeln!(self.out, "@{}", index)?;
                        writeln!(self.out, "D=A")?;
                        self.push_stack()?;
                    }
                    "local" | "argument" | "this" | "that" => {
                        let base = match segment {
                            "local" => "LCL",
                            "argument" => "ARG",
                            "this" => "THIS",
                            "that" => "THAT",
                            _ => unreachable!(),
                        };
                        writeln!(self.out, "// push {} {}", segment, index)?;
                        writeln!(self.out, "@{}", base)?;
                        writeln!(self.out, "D=M")?;
                        writeln!(self.out, "@{}", index)?;
                        writeln!(self.out, "A=D+A")?;
                        writeln!(self.out, "D=M")?;
                        self.push_stack()?;
                    }
                    "temp" => {
                        let addr = 5 + index;
                        writeln!(self.out, "// push temp {}", index)?;
                        writeln!(self.out, "@{}", addr)?;
                        writeln!(self.out, "D=M")?;
                        self.push_stack()?;
                    }
                    "pointer" => {
                        let addr = if index == 0 { "THIS" } else { "THAT" };
                        writeln!(self.out, "// push pointer {}", index)?;
                        writeln!(self.out, "@{}", addr)?;
                        writeln!(self.out, "D=M")?;
                        self.push_stack()?;
                    }
                    "static" => {
                        writeln!(self.out, "// push static {}.{}", self.file_name, index)?;
                        writeln!(self.out, "@{}.{}", self.file_name, index)?;
                        writeln!(self.out, "D=M")?;
                        self.push_stack()?;
                    }
                    _ => panic!("unsupported push segment: {}", segment),
                }
            }
            CommandType::CPop => {
                match segment {
                    "local" | "argument" | "this" | "that" => {
                        let base = match segment {
                            "local" => "LCL",
                            "argument" => "ARG",
                            "this" => "THIS",
                            "that" => "THAT",
                            _ => unreachable!(),
                        };
                        // store target address in R13, pop stack into D, then *R13 = D
                        writeln!(self.out, "// pop {} {}", segment, index)?;
                        writeln!(self.out, "@{}", base)?;
                        writeln!(self.out, "D=M")?;
                        writeln!(self.out, "@{}", index)?;
                        writeln!(self.out, "D=D+A")?;
                        writeln!(self.out, "@R13")?;
                        writeln!(self.out, "M=D")?;
                        writeln!(self.out, "@SP")?;
                        writeln!(self.out, "AM=M-1")?;
                        writeln!(self.out, "D=M")?;
                        writeln!(self.out, "@R13")?;
                        writeln!(self.out, "A=M")?;
                        writeln!(self.out, "M=D")?;
                    }
                    "temp" => {
                        let addr = 5 + index;
                        writeln!(self.out, "// pop temp {}", index)?;
                        writeln!(self.out, "@SP")?;
                        writeln!(self.out, "AM=M-1")?;
                        writeln!(self.out, "D=M")?;
                        writeln!(self.out, "@{}", addr)?;
                        writeln!(self.out, "M=D")?;
                    }
                    "pointer" => {
                        let addr = if index == 0 { "THIS" } else { "THAT" };
                        writeln!(self.out, "// pop pointer {}", index)?;
                        writeln!(self.out, "@SP")?;
                        writeln!(self.out, "AM=M-1")?;
                        writeln!(self.out, "D=M")?;
                        writeln!(self.out, "@{}", addr)?;
                        writeln!(self.out, "M=D")?;
                    }
                    "static" => {
                        writeln!(self.out, "// pop static {}.{}", self.file_name, index)?;
                        writeln!(self.out, "@SP")?;
                        writeln!(self.out, "AM=M-1")?;
                        writeln!(self.out, "D=M")?;
                        writeln!(self.out, "@{}.{}", self.file_name, index)?;
                        writeln!(self.out, "M=D")?;
                    }
                    _ => panic!("unsupported pop segment: {}", segment),
                }
            }
            _ => unreachable!(),
        }
        Ok(())
    }

    pub fn close(&mut self) -> io::Result<()> {
        self.end_asm()?;
        self.out.flush()
    }
}

