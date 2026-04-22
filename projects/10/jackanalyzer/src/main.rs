// src/main.rs
mod tokenize;
mod token;
mod serialize;

use std::env;
use std::iter::once;
use std::fs::File;
use std::io::{self, BufReader, BufWriter};
use std::path::{Path, PathBuf};
use either::Either;
use rayon::prelude::*;
use anyhow::Result;
use crate::tokenize::JackTokenizer;
use crate::serialize::{XmlSerializer, TokenSerializer};

const MESSAGE: &str = "usage: jackanalyzer <Dir/File.jack>";
const SERIALIZER: &str = "xml";

fn handle_path(path: &Path) -> anyhow::Result<()> {
    let output = output_file(path, SERIALIZER)?;
    let writer = BufWriter::new(File::create(output)?);
    let mut serializer = XmlSerializer::new(writer);
    
    process_file(path, &mut serializer)
}

fn process_file<S: TokenSerializer>(input: &Path, serializer: &mut S) -> Result<()> { 
    let reader = BufReader::new(File::open(input)?);
    let tokenizer = JackTokenizer::new(reader);

    serializer.serialize_all(tokenizer)
}

fn output_file(path: &Path, extension: &str) -> Result<PathBuf> {
    Ok(path.parent()
        .map(|parent| {
            path
                .file_stem()
                .map(|name| {
                    let mut new_name = name.to_os_string();
                    new_name.push("T");
                    parent.join(Path::new(&new_name).with_extension(extension))
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
                path_arg
                    .read_dir()?
                    .filter_map(|res| res.ok().map(|e| e.path()))
            )
        },
        false => Either::Right(once(path_arg))
    };

    let files: Vec<_> = paths
        .filter(|path| path.extension() == Some("jack".as_ref()))
        .collect();

    files.par_iter().for_each(|path| {
        if let Err(e) = handle_path(path) {
            eprintln!("Ошибка при обработке {:?}: {}", path, e);
        }
    });

    Ok(())
}

