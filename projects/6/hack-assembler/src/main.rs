mod parser;
mod code;

use std::env;
use std::path::Path;
use std::fs::File;
use std::io::{Write, BufWriter};
use parser::InstrType;
use crate::parser::Parser;

fn main() -> std::io::Result<()> {
    let mut args = env::args();
    let _prog = args.next();
    let path = match args.next() {
        Some(p) => p,
        None => {
            eprintln!("Usage: hack-assembler <file.asm>");
            std::process::exit(2);
        }
    };
    let in_path = Path::new(&path);
    let out_path = in_path.with_extension("hack");

    let mut parser = Parser::from_path(&in_path)?;
    let out_file = File::create(&out_path)?;
    let mut writer = BufWriter::new(out_file);

    while parser.advance() {
        match parser.instruction_type() {
            InstrType::AInstruction => {
                let sym = parser.symbol().unwrap();
                // only decimal numeric allowed
                let val: u16 = match sym.parse() {
                    Ok(v) => v,
                    Err(_) => {
                        eprintln!("Non-numeric A-instruction encountered: @{}", sym);
                        std::process::exit(1);
                    }
                };
                let bits = format!("{:016b}", val);
                writeln!(writer, "{}", bits)?;
            }
            InstrType::CInstruction => {
                let dest_m = parser.dest().unwrap_or_default();
                let comp_m = parser.comp().unwrap_or_default();
                let jump_m = parser.jump().unwrap_or_default();

                let comp_bits = code::comp(&comp_m);
                let dest_bits = code::dest(&dest_m);
                let jump_bits = code::jump(&jump_m);
                // C-instruction: 111 a c1..c6 d1 d2 d3 j1 j2 j3
                let line = format!("111{}{}{}", comp_bits, dest_bits, jump_bits);
                writeln!(writer, "{}", line)?;
            }
            InstrType::LInstruction => {
                // ignore labels in this basic assembler (no symbol table)
            }
            InstrType::NoInstruction => {}
        }
    }

    writer.flush()?;
    Ok(())
}
