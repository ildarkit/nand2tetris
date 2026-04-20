// src/main.rs
use std::env;
use std::iter::once;
use std::io;
use std::path::{Path, PathBuf};
use rayon::prelude::*;

const MESSAGE: &str = "usage: jackanalyzer <Dir/File.jack>";

fn output_file(path: &Path) -> io::Result<PathBuf> {
    path
        .parent()
        .map(|p| {
            path
                .file_name()
                .map(|name| {
                    let mut new_name = name.to_os_string();
                    new_name.push("T");
                    p.join(Path::new(&new_name).with_extension("xml"))
                })
        })
        .flatten()
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Не удалось получить имя выходного файла из пути: {:?}", path)
            )
        })
}

fn main() -> io::Result<()> {
    let arg = env::args().nth(1).expect(MESSAGE);
    let path_arg = Path::new(&arg).canonicalize()?;

    let paths: Box<dyn Iterator<Item=PathBuf>> = if path_arg.is_dir() {
        Box::new(
             path_arg.read_dir()?
                .filter_map(|res| res.ok().map(|e| e.path()))
        )
    } else {
        Box::new(once(path_arg))
    };
    let jack_files: Vec<_> = paths
        .filter(|path| path.extension() == Some("jack".as_ref()))
        .collect();

    jack_files
        .par_iter()
        .for_each(|path| {
            match output_file(path) {
                Ok(_output) => {
                    todo!();
                },
                Err(e) => eprintln!("{}", e),
            }
        });

    Ok(())
}

