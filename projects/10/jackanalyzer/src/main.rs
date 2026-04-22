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
use crate::serialize::XmlSerializer;

const MESSAGE: &str = "usage: jackanalyzer <Dir/File.jack>";

fn process_file(input: &PathBuf, output: PathBuf) -> Result<()> {
    let reader = BufReader::new(File::open(input)?);
    let writer = BufWriter::new(File::create(output)?);
    
    let tokenizer = JackTokenizer::new(reader);
    let mut serializer = XmlSerializer::new(writer);

    serializer.serialize_all(tokenizer)
}

fn output_file(path: &Path) -> Result<PathBuf> {
    Ok(path.parent()
        .map(|parent| {
            path
                .file_stem()
                .map(|name| {
                    let mut new_name = name.to_os_string();
                    new_name.push("T");
                    parent.join(Path::new(&new_name).with_extension("xml"))
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

    let jack_files: Vec<_> = paths
        .filter(|path| path.extension() == Some("jack".as_ref()))
        .collect();

    jack_files
        .par_iter()
        .for_each(|path| {
            match output_file(path) {
                Ok(output) => {
                    if let Err(e) = process_file(path, output) {
                        eprintln!("Ошибка при обработке {:?}: {}", path, e);
                    }
                },
                Err(e) => eprintln!("{}", e),
            }
        });

    Ok(())
}

