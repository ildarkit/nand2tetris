use std::io::{self, Write, BufWriter};

pub trait VMCommandWriter {
    fn write_push(&mut self, segment: Segment, index: usize) -> io::Result<()>;
    fn write_pop(&mut self, segment: Segment, index: usize) -> io::Result<()>;
    fn write_arithmetic(&mut self, command: Command) -> io::Result<()>;
    fn write_label(&mut self, label: &str) -> io::Result<()>;
    fn write_goto(&mut self, label: &str) -> io::Result<()>;
    fn write_if(&mut self, label: &str) -> io::Result<()>;
    fn write_call(&mut self, label: &str, n_args: usize) -> io::Result<()>;
    fn write_function(&mut self, label: &str, n_locals: usize) -> io::Result<()>;
    fn write_return(&mut self) -> io::Result<()>;
    fn close(&mut self) -> io::Result<()>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Segment {
    Constant,
    Argument,
    Local,
    Static,
    This,
    That,
    Pointer,
    Temp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Add,
    Sub,
    Neg,
    Eq,
    Gt,
    Lt,
    And,
    Or,
    Not,
}

pub struct VMWriter<W: Write> {
    writer: BufWriter<W>,
}

impl<W: Write> VMWriter<W> {
    pub fn new(output: W) -> Self {
        Self {
            writer: BufWriter::new(output),
        }
    }

    fn segment_to_str(segment: Segment) -> &'static str {
        match segment {
            Segment::Constant => "constant",
            Segment::Argument => "argument",
            Segment::Local => "local",
            Segment::Static => "static",
            Segment::This => "this",
            Segment::That => "that",
            Segment::Pointer => "pointer",
            Segment::Temp => "temp",
        }
    }
}

impl<W: Write> VMCommandWriter for VMWriter<W> {
    fn write_push(&mut self, segment: Segment, index: usize) -> io::Result<()> {
        writeln!(self.writer, "push {} {}", Self::segment_to_str(segment), index)
    }

    fn write_pop(&mut self, segment: Segment, index: usize) -> io::Result<()> {
        writeln!(self.writer, "pop {} {}", Self::segment_to_str(segment), index)
    }

    fn write_arithmetic(&mut self, command: Command) -> io::Result<()> {
        let cmd_str = match command {
            Command::Add => "add",
            Command::Sub => "sub",
            Command::Neg => "neg",
            Command::Eq => "eq",
            Command::Gt => "gt",
            Command::Lt => "lt",
            Command::And => "and",
            Command::Or => "or",
            Command::Not => "not",
        };
        writeln!(self.writer, "{}", cmd_str)
    }

    fn write_label(&mut self, label: &str) -> io::Result<()> {
        writeln!(self.writer, "label {}", label)
    }

    fn write_goto(&mut self, label: &str) -> io::Result<()> {
        writeln!(self.writer, "goto {}", label)
    }

    fn write_if(&mut self, label: &str) -> io::Result<()> {
        writeln!(self.writer, "if-goto {}", label)
    }

    fn write_call(&mut self, label: &str, n_args: usize) -> io::Result<()> {
        writeln!(self.writer, "call {} {}", label, n_args)
    }

    fn write_function(&mut self, label: &str, n_locals: usize) -> io::Result<()> {
        writeln!(self.writer, "function {} {}", label, n_locals)
    }

    fn write_return(&mut self) -> io::Result<()> {
        writeln!(self.writer, "return")
    }

    fn close(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

