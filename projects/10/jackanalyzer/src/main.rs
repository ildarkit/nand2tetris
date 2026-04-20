// src/main.rs
use std::env;
use std::iter::once;
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter};
use std::path::{Path, PathBuf};
use anyhow::Result;
use rayon::prelude::*;
use quick_xml::Writer;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};

const MESSAGE: &str = "usage: jackanalyzer <Dir/File.jack>";

fn process_file(input: &PathBuf, output: PathBuf) -> Result<()> {
    let input_file = File::open(input)?;
    let reader = BufReader::new(input_file);

    let output_file = File::create(output)?;
    let mut writer = Writer::new(BufWriter::new(output_file));

    writer.write_event(Event::Start(BytesStart::new("root")))?;

    for line in reader.lines() {
        let line = line?;
        writer.write_event(Event::Start(BytesStart::new("line")))?;
        writer.write_event(Event::Text(BytesText::new(&line)))?;
        writer.write_event(Event::End(BytesEnd::new("line")))?;
    }

    writer.write_event(Event::End(BytesEnd::new("root")))?;
    Ok(())
}

fn output_file(path: &Path) -> Result<PathBuf> {
    Ok(path
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
        })?)
}

fn main() -> Result<()> {
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

