use std::convert::AsRef;
use anyhow::Result;
use strum_macros::{EnumString, AsRefStr};
use crate::serialize::Serializer;
use crate::tokenize::{Tokenizer, TokenType};

#[derive(AsRefStr, Debug, PartialEq, Eq, EnumString, Clone)]
#[strum(serialize_all = "camelCase")]
enum CodeBlock {
    #[strum(serialize = "class")]
    Class,
    #[strum(
        serialize = "classVarDec",
        serialize = "static",
        serialize = "field"
    )]
    ClassVarDec,
    #[strum(
        serialize = "subroutineDec",
        serialize = "constructor",
        serialize = "function",
        serialize = "method"
    )]
    SubroutineDec,
    #[strum(serialize = "parameterList")]
    ParameterList,
    #[strum(serialize = "subroutineBody")]
    SubroutineBody,
    #[strum(serialize = "varDec", serialize = "var")]
    VarDec,
    #[strum(serialize = "statements")]
    Statements,
    #[strum(serialize = "letStatement", serialize = "let")]
    LetStatement,
    #[strum(serialize = "ifStatement", serialize = "if")]
    IfStatement,
    #[strum(serialize = "whileStatement", serialize = "while")]
    WhileStatement,
    #[strum(serialize = "doStatement", serialize = "do")]
    DoStatement,
    #[strum(serialize = "returnStatement", serialize = "return")]
    ReturnStatement,
    #[strum(serialize = "expression")]
    Expression,
    #[strum(serialize = "term")]
    Term,
}

impl CodeBlock {
    fn is_function(&self) -> bool {
        matches!(
            self,
            CodeBlock::SubroutineDec
        )
    }

    fn is_var(&self) -> bool {
        matches!(
            self,
            CodeBlock::VarDec
        )
    }

    fn is_statements(&self) -> bool {
        matches!(
            self,
            CodeBlock::LetStatement
                | CodeBlock::IfStatement
                | CodeBlock::WhileStatement
                | CodeBlock::DoStatement
                | CodeBlock::ReturnStatement
        )
    }
}

pub struct CompilationEngine<T: Tokenizer, S: Serializer> {
    reader: T,
    writer: S,
    section: CodeBlock,
}

impl<T: Tokenizer, S: Serializer> CompilationEngine<T, S> {
    pub fn new(reader: T, writer: S) -> Self {
        Self {
            reader,
            writer,
            section: CodeBlock::Class,
        }
    }

    fn next_section(&mut self, name: &str) -> Result<bool> {
        Ok(CodeBlock::try_from(name).map(|b| self.section = b).is_ok())
    }

    fn current_token(&mut self) -> Option<(String, String)> {
        let token;
        let token_type = self.reader.token_type();
        match token_type {
            TokenType::Keyword => token = self.reader.keyword().to_string(),
            TokenType::Symbol => token = self.reader.symbol().to_string(),
            TokenType::Identifier => token = self.reader.identifier().to_string(),
            TokenType::IntegerConstant => token = self.reader.int_val().to_string(),
            TokenType::StringConstant => {
                token = self.reader.string_val().trim_matches('"').to_string()
            }
            TokenType::EOF => {
                return None;
            },
            TokenType::Invalid(token) => {
                eprintln!("Неверный токен: {}", token);
                return None;
            }
        }
        Some((token_type.as_ref().to_string(), token))
    }

    fn compile(&mut self) -> Result<()> {
        while self.reader.advance()? {
            if let Some((name, value)) = self.current_token() {
                let prev_section = self.section.clone();
                if self.next_section(&value)? && prev_section != self.section {
                    // close prev tag if function or statements
                    if self.section.is_function() || self.section.is_statements() {
                        self.writer.end_name()?;
                    }
                    // statements
                    if self.section.is_statements() && (prev_section.is_function() ||
                        prev_section.is_var()) {
                        self.writer.write_name(CodeBlock::Statements.as_ref())?;
                    }
                    self.writer.write_name(self.section.as_ref())?;
                // class
                } else if self.section == CodeBlock::Class && value == "class" {
                    self.writer.write_name(self.section.as_ref())?;
                }
                if value == "}" {
                    self.writer.end_name()?;
                }
                // function body
                if value == "{" && self.section == CodeBlock::SubroutineDec {
                    self.writer.write_name(CodeBlock::SubroutineBody.as_ref())?;
                }
                self.writer.write_node(&name, &value)?;
            }
        }
        self.writer.finish()?;
        Ok(())
    }

    pub fn compile_class(&mut self) -> Result<()> {
        self.compile()?;
        Ok(())
    }
}
