// src/main.rs
use std::env;
use std::fs::File;
use std::io::{self, BufReader};
use std::path::Path;

mod parser;
mod codewriter;

use parser::Parser;
use codewriter::CodeWriter;

fn main() -> io::Result<()> {
    let arg = env::args().nth(1).expect("usage: vmtranslator <File.vm>");
    let path = Path::new(&arg);
    let file_stem = path.file_stem().and_then(|s| s.to_str()).expect("invalid filename");
    let input = File::open(&arg)?;
    let reader = BufReader::new(input);
    let out_path = path.with_extension("asm");
    let mut writer = CodeWriter::new(&out_path, file_stem)?;

    let mut parser = Parser::new(reader);
    while parser.advance()? {
        match parser.command_type() {
            parser::CommandType::CArithmetic => {
                let cmd = parser.arg1().unwrap();
                writer.write_arithmetic(cmd)?;
            }
            parser::CommandType::CPush | parser::CommandType::CPop => {
                let segment = parser.arg1().unwrap();
                let index = parser.arg2().unwrap();
                writer.write_push_pop(parser.command_type(), segment, index)?;
            }
        }
    }

    writer.close()?;
    Ok(())
}

