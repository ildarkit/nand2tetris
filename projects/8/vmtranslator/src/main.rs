// src/main.rs
use std::env;
use std::fs::File;
use std::io::{self, BufReader};
use std::path::Path;

mod parser;
mod codewriter;

use parser::Parser;
use codewriter::CodeWriter;

const MESSAGE: &str = "usage: vmtranslator <Dir/File.vm>";

fn entry_process(entry_path: &Path, writer: &mut CodeWriter) -> io::Result<()> {
    if let Some(entry) = entry_path.to_str() {
        let input = File::open(entry)?;
        let reader = BufReader::new(input);
        let mut parser = Parser::new(reader);
        while parser.advance()? {
            let arg1 = parser.arg1().unwrap();
            match parser.command_type() {
                parser::CommandType::CArithmetic => {
                    writer.write_arithmetic(arg1)?;
                }
                parser::CommandType::CPush | parser::CommandType::CPop => {
                    let index = parser.arg2().unwrap();
                    writer.write_push_pop(parser.command_type(), arg1, index)?;
                }
                parser::CommandType::CGoto => {
                    writer.write_goto(arg1)?;
                }
                parser::CommandType::CIfGoto => {
                    writer.write_if(arg1)?;
                }
                parser::CommandType::CLabel => {
                    writer.write_label(arg1)?;
                }
                parser::CommandType::CFunction => {
                    let nvars = parser.arg2().unwrap();
                    writer.write_function(arg1, nvars)?;
                },
                parser::CommandType::CCall => {
                    let nargs = parser.arg2().unwrap();
                    writer.write_call(arg1, nargs)?;
                },
                parser::CommandType::CReturn => {
                    writer.write_return()?;
                },
            }
        }
    }
    Ok(())
}

fn file_path_stem(path: &Path) -> Option<&str> {
    path.file_stem().and_then(|s| s.to_str())
}

fn main() -> io::Result<()> {
    let arg = env::args().nth(1).expect(MESSAGE);
    let path_buf = Path::new(&arg).canonicalize()?;
    let path = path_buf.as_path();
    let out_path = if path.is_dir() {
        let file_name = path.file_name().unwrap();
        path.join(Path::new(file_name).with_extension("asm"))
    } else {
        path.with_extension("asm")
    };

    if let Some(file_stem) = file_path_stem(path) {
        let mut writer = CodeWriter::new(&out_path, file_stem)?;
        writer.bootstrap("Sys.init")?;

        if path.is_dir() {
            for entry in path.read_dir()? {
                let entry_path = entry?.path();
                if let Some(ext) = entry_path.extension() {
                    if ext == "vm" {
                        if let Some(entry_file_stem) = file_path_stem(&entry_path) {
                            writer.set_file_name(entry_file_stem);
                            entry_process(&entry_path, &mut writer)?;
                        }
                    }
                }
            }
        } else {
            entry_process(path, &mut writer)?;
        };

        writer.close()?;
    }
    Ok(())
}

