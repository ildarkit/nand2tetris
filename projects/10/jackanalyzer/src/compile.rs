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
    #[strum(serialize = "expressionList")]
    ExpressionList,
}

impl CodeBlock {
    fn is_class(&self) -> bool {
        matches!(
            self,
            CodeBlock::Class
        )
    }

    fn is_function(&self) -> bool {
        matches!(
            self,
            CodeBlock::SubroutineDec
        )
    }

    fn is_function_body(&self) -> bool {
        matches!(
            self,
            CodeBlock::SubroutineBody
        )
    }

    fn is_vars(&self) -> bool {
        matches!(
            self,
            CodeBlock::VarDec
                | CodeBlock::ClassVarDec,
        )
    }

    fn is_class_var(&self) -> bool {
        matches!(
            self,
            CodeBlock::ClassVarDec,
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

    fn is_if_statement(&self) -> bool {
        matches!(
            self,
            CodeBlock::IfStatement
        )
    }
}

pub struct CompilationEngine<T: Tokenizer, S: Serializer> {
    reader: T,
    writer: S,
    section: CodeBlock,
    if_statement: bool,
}

impl<T: Tokenizer, S: Serializer> CompilationEngine<T, S> {
    pub fn new(reader: T, writer: S) -> Self {
        Self {
            reader,
            writer,
            section: CodeBlock::Class,
            if_statement: false,
        }
    }

    fn next_section(&mut self, name: &str) -> Result<bool> {
        Ok(self.get_section(name).map(|b| self.section = b).is_ok())
    }

    fn get_section(&mut self, name: &str) -> Result<CodeBlock> {
        Ok(CodeBlock::try_from(name)?)
    }

    fn get_token(&mut self) -> Option<(String, String)> {
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
        let outter = self.section.clone();
        let mut statements_block = false;
        let mut if_statement_closed = false;
        if outter.is_class() {
            self.writer.write_name(outter.as_ref())?;
        }
        while self.reader.advance()? {
            if let Some((name, value)) = self.get_token() {
                if self.next_section(&value)? {
                    if self.section.is_class() {
                        self.writer.write_node(&name, &value)?;
                    }
                    // ClassVarDec or VarDec
                    if self.section.is_vars() {
                        let var_type = if self.section.is_class_var() {
                            CodeBlock::ClassVarDec
                        } else {
                            CodeBlock::VarDec
                        };
                        self.writer.write_name(var_type.as_ref())?;
                        self.writer.write_node(&name, &value)?;
                        self.compile()?;
                        self.writer.end_name(var_type.as_ref())?;
                    }
                    // SubroutineDec
                    if self.section.is_function() {
                        let function = self.section.clone();
                        if let Ok(func_type) = self.get_section(&value) &&
                            func_type.is_function() {
                            self.writer.write_name(function.as_ref())?;
                        }
                        self.writer.write_node(&name, &value)?;
                    }
                    // Statements
                    if self.section.is_statements() {
                        if if_statement_closed {
                            self.writer.end_name(CodeBlock::IfStatement.as_ref())?;
                            if_statement_closed = false;
                        }
                        if !statements_block {
                            self.writer.write_name(CodeBlock::Statements.as_ref())?;
                            statements_block = true;
                        }
                        let statement = self.section.clone();
                        if let Ok(statement) = self.get_section(&value) &&
                            statement.is_statements() {
                            self.writer.write_name(statement.as_ref())?;
                        }
                        self.writer.write_node(&name, &value)?;
                        self.compile()?;
                        
                        if !statement.is_if_statement() {
                            self.writer.end_name(statement.as_ref())?;
                        } else {
                            self.if_statement = true;
                        }
                    }
                    if !(outter.is_class() || outter.is_function_body() ||
                        outter.is_if_statement()) {
                        self.writer.end_name(outter.as_ref())?;
                    }
                } else {
                    if value == "{" && self.section.is_function() {
                        self.section = CodeBlock::SubroutineBody;
                        self.writer.write_name(CodeBlock::SubroutineBody.as_ref())?;
                        self.writer.write_node(&name, &value)?;
                        self.compile()?;
                    } else if value == "{" && !(outter.is_function() || outter.is_class()) {
                        // else branch -> statements
                        self.section = CodeBlock::IfStatement;
                        self.compile()?;
                    } else if value == ";" || value == "}" {
                        if value == "}" {
                            if !if_statement_closed {
                                self.writer.end_name(CodeBlock::Statements.as_ref())?;
                            }
                            if outter.is_function_body() {
                                self.writer.end_name(CodeBlock::SubroutineBody.as_ref())?;
                                self.writer.end_name(CodeBlock::SubroutineDec.as_ref())?;
                            }
                        }
                        // exit from function body 
                        if if_statement_closed {
                            self.writer.end_name(CodeBlock::SubroutineBody.as_ref())?;
                            self.writer.end_name(CodeBlock::SubroutineDec.as_ref())?;
                            self.writer.write_node(&name, &value)?;
                            return Ok(());
                        } else {
                            self.writer.write_node(&name, &value)?;
                        }
                        if !outter.is_if_statement() {
                            return Ok(());
                        } else {
                            if_statement_closed = true;
                        }
                    } else {
                        if self.if_statement && value != "else" {
                            if value != "{" {
                                self.writer.end_name(CodeBlock::IfStatement.as_ref())?;
                            }
                            self.writer.write_node(&name, &value)?;
                        } else {
                            self.writer.write_node(&name, &value)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn compile_class(&mut self) -> Result<()> {
        self.compile()?;
        self.writer.end_name(CodeBlock::Class.as_ref())?;
        Ok(())
    }
}
