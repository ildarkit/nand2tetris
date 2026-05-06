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
    fn next(&self) -> Self {
        match self {
            CodeBlock::Class => CodeBlock::ClassVarDec,
            CodeBlock::ClassVarDec => CodeBlock::SubroutineDec,
            CodeBlock::SubroutineDec => CodeBlock::ParameterList,
            CodeBlock::ParameterList => CodeBlock::SubroutineBody,
            CodeBlock::SubroutineBody => CodeBlock::VarDec,
            CodeBlock::VarDec => CodeBlock::Statements,
            CodeBlock::Statements => CodeBlock::Expression,
            CodeBlock::Expression => CodeBlock::Term,
            CodeBlock::LetStatement => CodeBlock::Expression,
            CodeBlock::IfStatement => CodeBlock::Expression,
            CodeBlock::WhileStatement => CodeBlock::Expression,
            CodeBlock::DoStatement => CodeBlock::ExpressionList,
            CodeBlock::ReturnStatement => CodeBlock::Expression,
            CodeBlock::ExpressionList => CodeBlock::Expression,
            CodeBlock::Term => CodeBlock::Expression,
        }
    }

    fn is_outside_closing(&self) -> bool {
        matches!(
            self,
            CodeBlock::LetStatement |
            CodeBlock::DoStatement |
            CodeBlock::ReturnStatement |
            CodeBlock::ClassVarDec |
            CodeBlock::VarDec |
            CodeBlock::SubroutineBody |
            CodeBlock::Class
        )
    }

    fn is_statements(&self) -> bool {
        matches!(
            self,
            CodeBlock::LetStatement |
            CodeBlock::DoStatement |
            CodeBlock::ReturnStatement |
            CodeBlock::IfStatement |
            CodeBlock::WhileStatement
        )
    }

    fn is_term(&self) -> bool {
        matches!(
            self,
            CodeBlock::Term
        )
    }

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
}

pub trait Compiler {
    fn compile_class(&mut self) -> Result<()>;
    fn compile_class_var_dec(&mut self) -> Result<()>;
    fn compile_subroutine(&mut self) -> Result<()>;
    fn compile_parameter_list(&mut self) -> Result<()>;
    fn compile_subroutine_body(&mut self) -> Result<()>;
    fn compile_var_dec(&mut self) -> Result<()>;
    fn compile_statements(&mut self) -> Result<()>;
    fn compile_let(&mut self) -> Result<()>;
    fn compile_if(&mut self) -> Result<()>;
    fn compile_while(&mut self) -> Result<()>;
    fn compile_do(&mut self) -> Result<()>;
    fn compile_return(&mut self) -> Result<()>;
    fn compile_expression(&mut self) -> Result<()>;
    fn compile_term(&mut self) -> Result<()>;
    fn compile_expression_list(&mut self) -> Result<()>;
}

pub struct CompilationEngine<T: Tokenizer, S: Serializer> {
    reader: T,
    writer: S,
    section: CodeBlock,
    token: Option<(TokenType, String)>,
    param_wrapper: Option<CodeBlock>,
    buf_section: Option<CodeBlock>,
    function_completed: bool,
    section_updated: bool,
    statements_tag: bool,
    nesting_count: u32,
}

impl<T: Tokenizer, S: Serializer> CompilationEngine<T, S> {
    pub fn new(reader: T, writer: S) -> Self {
        Self {
            reader,
            writer,
            section: CodeBlock::Class,
            token: None,
            param_wrapper: None,
            buf_section: None,
            function_completed: false,
            section_updated: false,
            statements_tag: false,
            nesting_count: 0,
        }
    }

    fn section_from(&mut self, name: &str) -> Result<bool> {
        Ok(CodeBlock::try_from(name).map(|b| self.section = b).is_ok())
    }

    fn get_token(&mut self) -> Option<(TokenType, String)> {
        let tt = self.reader.token_type();
        match tt {
            TokenType::Keyword => Some((tt, self.reader.keyword().to_string())),
            TokenType::Symbol => Some((tt, self.reader.symbol().to_string())),
            TokenType::Identifier => Some((tt, self.reader.identifier().to_string())),
            TokenType::IntegerConstant => Some((tt, self.reader.int_val().to_string())),
            TokenType::StringConstant => {
                Some((tt, self.reader.string_val().trim_matches('"').to_string()))
            }
            TokenType::EOF => None,
            TokenType::Invalid(tok) => {
                eprintln!("Неверный токен: {}", tok);
                None
            }
        }
    }

    fn compile_next(&mut self) -> Result<()> {
        match self.section {
            CodeBlock::Class => unreachable!(),
            CodeBlock::ClassVarDec => self.compile_class_var_dec(),
            CodeBlock::SubroutineDec => self.compile_subroutine(),
            CodeBlock::ParameterList => self.compile_parameter_list(),
            CodeBlock::SubroutineBody => self.compile_subroutine_body(),
            CodeBlock::VarDec => self.compile_var_dec(),
            CodeBlock::Statements => self.compile_statements(),
            CodeBlock::Expression => self.compile_expression(),
            CodeBlock::Term => self.compile_term(),
            CodeBlock::ExpressionList => self.compile_expression_list(),
            CodeBlock::LetStatement => self.compile_let(),
            CodeBlock::IfStatement => self.compile_if(),
            CodeBlock::WhileStatement => self.compile_while(),
            CodeBlock::DoStatement => self.compile_do(),
            CodeBlock::ReturnStatement => self.compile_return(),
        }
    }

    fn set_params_wrapper(&mut self) {
        self.param_wrapper.replace(self.section.next());
    }

    fn identifier_params(&mut self, name: &TokenType) {
        if self.section.is_term() && *name == TokenType::Identifier {
            self.param_wrapper.replace(CodeBlock::ExpressionList);
        }
    }

    fn params_section(&mut self) {
        let wrapper = self.param_wrapper.take();
        if let Some(section) = wrapper {
            self.buf_section = Some(self.section.clone());
            self.section = section;
        }
    }

    fn restore_section(&mut self) {
        if let Some(section) = self.buf_section.clone() {
            self.section = section;
        }
    }

    fn compile_block(&mut self) -> Result<bool> {
        if let Some((name, value)) = self.token.take() {
            self.writer.write_node(&name.as_ref(), &value)?;
        }
        while self.reader.advance()? {
            if let Some((name, value)) = self.get_token() {
                if value != "class" && self.section_from(&value)? {
                    self.token = Some((name, value));
                    return Ok(false);
                } else {
                    match value.as_str() {
                        ";" => {
                            self.token = Some((name, value));
                            return Ok(true);
                        }
                        "(" => {
                            self.params_section();
                            self.writer.write_node(&name.as_ref(), &value)?;
                            return Ok(false);
                        }
                        "{" => {
                            self.nesting_count += 1;
                            if !self.section.is_function() {
                                self.writer.write_node(&name.as_ref(), &value)?;
                                if !self.section.is_class() {
                                    self.toggle_statements(); // true
                                }
                            } else {
                                self.toggle_statements(); // true
                                self.section = CodeBlock::SubroutineBody;
                                self.token = Some((name, value));
                                return Ok(false);
                            }
                        }
                        ")" => {
                            self.token = Some((name, value));
                            return Ok(true);
                        }
                        "}" => {
                            self.nesting_count -= 1;
                            if self.nesting_count == 1 {
                                self.toggle_function(); // true
                            }
                            self.token = Some((name, value));
                            return Ok(true);
                        }
                        _ => {
                            self.identifier_params(&name);
                            self.writer.write_node(&name.as_ref(), &value)?;
                        }
                    }
                }
            }
        }
        Ok(false)
    }

    fn compile(&mut self) -> Result<()> {
        loop {
            if !self.section_updated && self.compile_block()? {
                break;
            }
            self.start_statements();
            self.compile_next()?;
            if self.function_completed {
                break;
            }
        }
        Ok(())
    }

    fn start_statements(&mut self) {
        if self.section.is_statements() && self.statements_tag {
            self.buf_section = Some(self.section.clone());
            self.section = CodeBlock::Statements;
        }
    }

    fn ending_code_block(&mut self, block: &CodeBlock) -> Result<()> {
        let token = self.token.take();
        if block.is_outside_closing() {
            if let Some((name, value)) = token {
                self.writer.write_node(&name.as_ref(), &value)?;
            }
            self.writer.end_name(block.as_ref())?;
        } else {
            self.writer.end_name(block.as_ref())?;
            if let Some((name, value)) = token {
                self.writer.write_node(&name.as_ref(), &value)?;
            }
        }
        Ok(())
    }

    fn wrap_compiler(&mut self) -> Result<()> {
        let code_block = self.section.clone();
        self.section_updated = false;
        self.writer.write_name(code_block.as_ref())?;
        self.compile()?;
        self.ending_code_block(&code_block)?;
        Ok(())
    }

    fn toggle_function(&mut self) {
        self.function_completed = !self.function_completed;
    }

    fn toggle_statements(&mut self) {
        self.statements_tag = !self.statements_tag;
    }
}

impl<T: Tokenizer, S: Serializer> Compiler for CompilationEngine<T, S> {
    fn compile_class(&mut self) -> Result<()> {
        self.wrap_compiler()?;
        Ok(())
    }

    fn compile_class_var_dec(&mut self) -> Result<()> {
        self.wrap_compiler()?;
        Ok(())
    }

    fn compile_subroutine(&mut self) -> Result<()> {
        self.set_params_wrapper();
        self.wrap_compiler()?;
        self.toggle_function();
        Ok(())
    }

    fn compile_parameter_list(&mut self) -> Result<()> {
        self.wrap_compiler()?;
        self.restore_section();
        Ok(())
    }

    fn compile_subroutine_body(&mut self) -> Result<()> {
        self.wrap_compiler()?;
        Ok(())
    }

    fn compile_var_dec(&mut self) -> Result<()> {
        self.wrap_compiler()?;
        Ok(())
    }

    fn compile_statements(&mut self) -> Result<()> {
        let code_block = self.section.clone();
        self.toggle_statements(); // false
        self.section_updated = true;
        self.writer.write_name(code_block.as_ref())?;
        if let Some(buf_section) = self.buf_section.take() {
            self.section = buf_section;
        }
        self.compile()?;
        self.ending_code_block(&code_block)?;
        Ok(())
    }

    fn compile_let(&mut self) -> Result<()> {
        self.set_params_wrapper();
        self.wrap_compiler()?;
        Ok(())
    }

    fn compile_if(&mut self) -> Result<()> {
        self.set_params_wrapper();
        self.wrap_compiler()?;
        Ok(())
    }

    fn compile_while(&mut self) -> Result<()> {
        self.set_params_wrapper();
        self.wrap_compiler()?;
        Ok(())
    }

    fn compile_do(&mut self) -> Result<()> {
        self.set_params_wrapper();
        self.wrap_compiler()?;
        Ok(())
    }

    fn compile_return(&mut self) -> Result<()> {
        self.set_params_wrapper();
        self.wrap_compiler()?;
        Ok(())
    }

    fn compile_expression(&mut self) -> Result<()> {
        self.wrap_compiler()?;
        Ok(())
    }

    fn compile_term(&mut self) -> Result<()> {
        self.set_params_wrapper();
        self.wrap_compiler()?;
        Ok(())
    }

    fn compile_expression_list(&mut self) -> Result<()> {
        self.wrap_compiler()?;
        Ok(())
    }
}
