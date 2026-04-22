// src/main.rs
mod jacktokenizer;

use std::env;
use std::iter::once;
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter};
use std::path::{Path, PathBuf};
use either::Either;
use rayon::prelude::*;
use anyhow::Result;
use quick_xml::Writer;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use jacktokenizer::{JackTokenizer, TokenType};

const MESSAGE: &str = "usage: jackanalyzer <Dir/File.jack>";

enum Token<'a> {
    Keyword(&'a str),
    Symbol(&'a str),
    Identifier(&'a str),
    IntConst(&'a str),
    StringConst(&'a str),
}

impl<'a> Token<'a> {
    pub fn from_tokenizer<R: BufRead>(t: &'a mut JackTokenizer<R>) -> Option<Self> {
        match t.token_type() {
            TokenType::Keyword => Some(Token::Keyword(t.keyword())),
            TokenType::Symbol => Some(Token::Symbol(t.symbol())),
            TokenType::Identifier => Some(Token::Identifier(t.identifier())),
            TokenType::IntConst => Some(Token::IntConst(t.int_val())),
            TokenType::StringConst => Some(Token::StringConst(t.string_val())),
            TokenType::EOF => None,
            TokenType::Invalid(token) => {
                eprintln!("Неверный токен: {}", token);
                None
            }
        }
    }

    pub fn write_to<W: std::io::Write>(
        &self,
        writer: &mut quick_xml::Writer<W>
        ) -> Result<()>
    {
        let (tag, value_str);
        match self {
            Self::Keyword(v) => {
                tag = "keyword";
                value_str = v;
            }
            Self::Symbol(v) => {
                tag = "symbol";
                value_str = v;
            }
            Self::Identifier(v) => {
                tag = "identifier";
                value_str = v;
            }
            Self::IntConst(v) => { 
                tag = "integerConstant"; 
                value_str = v; 
            }
            Self::StringConst(v) => {
                tag = "stringConstant";
                value_str = v;
            }
        };

        writer.write_event(Event::Start(BytesStart::new(tag)))?;
        writer.write_event(Event::Text(BytesText::new(value_str)))?;
        writer.write_event(Event::End(BytesEnd::new(tag)))?;
        
        Ok(())
    }
}

fn process_file(input: &PathBuf, output: PathBuf) -> Result<()> {
    let input_file = File::open(input)?;
    let reader = BufReader::new(input_file);
    let output_file = File::create(output)?;
    let mut writer = Writer::new_with_indent(BufWriter::new(output_file), b' ', 4);

    writer.write_event(Event::Start(BytesStart::new("tokens")))?;
    let mut tokenizer = JackTokenizer::new(reader);
    while tokenizer.advance()? {
        if let Some(token) = Token::from_tokenizer(&mut tokenizer) {
            token.write_to(&mut writer)?;
        } else {
            break;
        }
    }
    writer.write_event(Event::End(BytesEnd::new("tokens")))?;
    Ok(())
}

fn output_file(path: &Path) -> Result<PathBuf> {
    let path = path
        .parent()
        .map(|parent| {
            path
                .file_name()
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
        })?;
    Ok(path)
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

