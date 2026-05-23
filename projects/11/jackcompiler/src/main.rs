// src/main.rs
mod tokenize;
mod compile;
mod grammar;
mod symbol_table;
mod vm_writer;
mod label_generator;

use std::env;
use std::iter::once;
use std::fs::File;
use std::io::{self, BufReader, BufWriter};
use std::path::{Path, PathBuf};
use either::Either;
use rayon::prelude::*;
use anyhow::Result;
use crate::tokenize::JackTokenizer;
use crate::vm_writer::VMWriter;
use crate::compile::{CompilationEngine, Compiler};

const MESSAGE: &str = "usage: jackcompiler <Dir/File.jack>";

fn compile(input: &Path) -> Result<()> {
    let reader = JackTokenizer::new(
        BufReader::new(File::open(input)?)
    );
    let writer = VMWriter::new(
        BufWriter::new(
            File::create(
                output_file(input, "vm")?
            )?
        )
    );
    let mut compiler = CompilationEngine::new(reader, writer);
    compiler.compile_class()?;
    Ok(())
}

fn output_file(path: &Path, extension: &str) -> Result<PathBuf> {
    Ok(path.parent()
        .map(|parent| {
            path
                .file_stem()
                .map(|name| {
                    parent.join(
                        Path::new(&name.to_os_string())
                            .with_extension(extension)
                    )
                })
        })
        .flatten()
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Не удалось получить имя выходного файла из пути: {:?}", path)
            )
        })?
    )
}

fn main() -> Result<()> {
    let arg = env::args().nth(1).expect(MESSAGE);
    let path_arg = Path::new(&arg).canonicalize()?;

    let paths = match path_arg.is_dir() {
        true => {
            Either::Left(
                path_arg.read_dir()?
                    .filter_map(|res| res.ok().map(|e| e.path()))
            )
        },
        false => Either::Right(once(path_arg))
    };

    let files: Vec<_> = paths
        .filter(|path| path.extension() == Some("jack".as_ref()))
        .collect();

    files.par_iter().for_each(|path| {
        if let Err(e) = compile(path) {
            eprintln!("Ошибка при обработке {:?}: {}", path, e);
        }
    });

    Ok(())
}

