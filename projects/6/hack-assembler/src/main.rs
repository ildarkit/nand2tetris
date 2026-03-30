mod parser;
mod code;
mod symbol_table;

use std::env;
use std::path::Path;
use std::fs::File;
use std::io::{Write, BufWriter};
use parser::InstrType;
use crate::parser::Parser;
use crate::symbol_table::SymbolTable;

fn first_pass(path: &Path, symtab: &mut SymbolTable) -> std::io::Result<()> {
    let mut parser = Parser::from_path(path)?;
    let mut rom_addr: u16 = 0;
    while parser.advance() {
        match parser.instruction_type() {
            InstrType::AInstruction | InstrType::CInstruction => {
                rom_addr = rom_addr.wrapping_add(1);
            }
            InstrType::LInstruction => {
                if let Some(sym) = parser.symbol() {
                    // если метка ещё не существует, добавить с текущим ROM-адресом
                    if !symtab.contains(&sym) {
                        symtab.add_entry(&sym, rom_addr);
                    }
                }
            }
            InstrType::NoInstruction => {}
        }
    }
    Ok(())
}

fn second_pass(path: &Path, out_path: &Path, symtab: &mut SymbolTable) -> std::io::Result<()> {
    let mut parser = Parser::from_path(path)?;
    let out_file = File::create(out_path)?;
    let mut writer = BufWriter::new(out_file);
    let mut next_var_addr: u16 = 16;

    while parser.advance() {
        match parser.instruction_type() {
            InstrType::AInstruction => {
                let sym = parser.symbol().unwrap();
                // если числовая константа
                if let Ok(val) = sym.parse::<u16>() {
                    writeln!(writer, "{:016b}", val)?;
                } else {
                    // символическая ссылка: метка или переменная
                    let addr = if symtab.contains(&sym) {
                        symtab.get_address(&sym).unwrap()
                    } else {
                        // новая переменная
                        let a = next_var_addr;
                        symtab.add_entry(&sym, a);
                        next_var_addr = next_var_addr.wrapping_add(1);
                        a
                    };
                    writeln!(writer, "{:016b}", addr)?;
                }
            }
            InstrType::CInstruction => {
                let dest_m = parser.dest().unwrap_or_default();
                let comp_m = parser.comp().unwrap_or_default();
                let jump_m = parser.jump().unwrap_or_default();

                let comp_bits = code::comp(&comp_m);
                let dest_bits = code::dest(&dest_m);
                let jump_bits = code::jump(&jump_m);
                let line = format!("111{}{}{}", comp_bits, dest_bits, jump_bits);
                writeln!(writer, "{}", line)?;
            }
            InstrType::LInstruction | InstrType::NoInstruction => {
                // метки пропускаются при генерации
            }
        }
    }

    writer.flush()?;
    Ok(())
}

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

    let mut symtab = SymbolTable::new();
    first_pass(in_path, &mut symtab)?;
    second_pass(in_path, &out_path, &mut symtab)?;

    Ok(())
}

