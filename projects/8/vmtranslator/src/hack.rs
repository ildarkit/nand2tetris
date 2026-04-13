use std::io::{self, Write};
use std::fs::File;

pub struct HackWriter {
    out: File, 
}

impl HackWriter {
    pub fn new(out: File) -> Self {
        Self { out }
    }

    pub fn bootstrap(&mut self) -> io::Result<()> {
        writeln!(self.out, "@256")?;
        writeln!(self.out, "D=A")?;
        writeln!(self.out, "@SP")?;
        writeln!(self.out, "M=D")?;
        Ok(())
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

    pub fn write_label(&mut self, label: &str) -> io::Result<()> {
        writeln!(self.out, "// label {}", label)?;
        writeln!(self.out, "({})", label)?;
        Ok(())
    }

    pub fn write_goto(&mut self, label: &str) -> io::Result<()> {
        writeln!(self.out, "// goto {}", label)?;
        writeln!(self.out, "@{}", label)?;
        writeln!(self.out, "0;JMP")?;
        Ok(())
    }

    pub fn write_if(&mut self, label: &str) -> io::Result<()> {
        writeln!(self.out, "// if-goto {}", label)?;
        writeln!(self.out, "@SP")?;
        writeln!(self.out, "AM=M-1")?; // SP--, A=SP
        writeln!(self.out, "D=M")?;    // D = y
        writeln!(self.out, "@{}", label)?;
        writeln!(self.out, "D;JNE")?;
        Ok(())
    }

    pub fn write_function(&mut self, function_name: &str, nvars: u32) -> io::Result<()> {
        writeln!(self.out, "// function {} {}", function_name, nvars)?;

        writeln!(self.out, "({})", function_name)?; // function label

        if nvars > 0 {
            writeln!(self.out, "@0")?; // locals = 0
            writeln!(self.out, "D=A")?;
            writeln!(self.out, "@LCL")?;
            writeln!(self.out, "A=M")?;
            for i in 0..nvars {
                writeln!(self.out, "M=D")?;
                if i < nvars - 1 {
                    writeln!(self.out, "A=A+1")?;
                }
            }
        }
        Ok(())
    }

    pub fn write_call(
        &mut self,
        function_name: &str,
        nargs: u32,
        return_address: &str) -> io::Result<()>
    {
        writeln!(self.out, "// call {} {}", function_name, nargs)?;
        writeln!(self.out, "@{}", return_address)?; // store segments address in stack
        writeln!(self.out, "D=A")?;
        self.push_stack()?;
        writeln!(self.out, "@LCL")?;
        writeln!(self.out, "D=M")?;
        self.push_stack()?;
        writeln!(self.out, "@ARG")?;
        writeln!(self.out, "D=M")?;
        self.push_stack()?;
        writeln!(self.out, "@THIS")?;
        writeln!(self.out, "D=M")?;
        self.push_stack()?;
        writeln!(self.out, "@THAT")?;
        writeln!(self.out, "D=M")?;
        self.push_stack()?;

        writeln!(self.out, "@SP")?; // ARG
        writeln!(self.out, "D=M")?;
        if nargs > 0 {
            writeln!(self.out, "@{}", nargs)?;
            writeln!(self.out, "D=D-A")?;
        }
        writeln!(self.out, "@5")?;
        writeln!(self.out, "D=D-A")?;
        writeln!(self.out, "@ARG")?;
        writeln!(self.out, "M=D")?;

        writeln!(self.out, "@SP")?; // LCL
        writeln!(self.out, "D=M")?;
        writeln!(self.out, "@LCL")?;
        writeln!(self.out, "M=D")?;

        writeln!(self.out, "@{}", function_name)?; // goto function
        writeln!(self.out, "0;JMP")?;

        writeln!(self.out, "({})", return_address)?;
        Ok(())
    }

    pub fn write_return(&mut self, function: &str) -> io::Result<()> {
        writeln!(self.out, "// return {}", function)?;

        writeln!(self.out, "@LCL")?; // store LCL segment address
        writeln!(self.out, "D=M")?;
        writeln!(self.out, "@R13")?;
        writeln!(self.out, "M=D")?;

        writeln!(self.out, "@5")?; // store return address
        writeln!(self.out, "A=D-A")?;
        writeln!(self.out, "D=M")?;
        writeln!(self.out, "@R14")?;
        writeln!(self.out, "M=D")?;

        writeln!(self.out, "@SP")?; // return to ARG 0
        writeln!(self.out, "AM=M-1")?;
        writeln!(self.out, "D=M")?;
        writeln!(self.out, "@ARG")?;
        writeln!(self.out, "A=M")?;
        writeln!(self.out, "M=D")?;

        writeln!(self.out, "@ARG")?; // SP = ARG + 1
        writeln!(self.out, "D=M")?;
        writeln!(self.out, "@SP")?; 
        writeln!(self.out, "M=D+1")?;

        let segment = vec!["THAT", "THIS", "ARG", "LCL"]; // restore segment address
        let mut i = 1;
        for seg in segment {
            writeln!(self.out, "@R13")?;
            writeln!(self.out, "D=M")?;
            writeln!(self.out, "@{}", i)?;
            writeln!(self.out, "A=D-A")?;
            writeln!(self.out, "D=M")?;
            writeln!(self.out, "@{}", seg)?;
            writeln!(self.out, "M=D")?;
            i += 1;
        }

        writeln!(self.out, "@R14")?; // goto return address
        writeln!(self.out, "D=M")?;
        writeln!(self.out, "A=D")?;
        writeln!(self.out, "0;JMP")?;
        Ok(())
    }

    pub fn write_arithmetic(
        &mut self,
        cmd: &str,
        file_name: &str,
        label_counter: u32) -> io::Result<()>
    {
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
                let label_true = format!("{}_TRUE_{}", file_name, label_counter);
                let label_end = format!("{}_END_{}", file_name, label_counter);
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

    pub fn write_push_segment(
        &mut self,
        file_name: &str,
        segment: &str,
        index: u32) -> io::Result<()>
    {
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
                writeln!(self.out, "// push static {}.{}", file_name, index)?;
                writeln!(self.out, "@{}.{}", file_name, index)?;
                writeln!(self.out, "D=M")?;
                self.push_stack()?;
            }
            _ => panic!("unsupported push segment: {}", segment),
        }
        Ok(())
    }

    pub fn write_pop_segment(
        &mut self,
        file_name: &str,
        segment: &str,
        index: u32) -> io::Result<()>
    {
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
                writeln!(self.out, "// pop static {}.{}", file_name, index)?;
                writeln!(self.out, "@SP")?;
                writeln!(self.out, "AM=M-1")?;
                writeln!(self.out, "D=M")?;
                writeln!(self.out, "@{}.{}", file_name, index)?;
                writeln!(self.out, "M=D")?;
            }
            _ => panic!("unsupported pop segment: {}", segment),
        }
        Ok(())
    }

    pub fn close(&mut self) -> io::Result<()> {
        self.out.flush()
    }
}
