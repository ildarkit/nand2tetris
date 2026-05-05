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
            CodeBlock::SubroutineBody
        )
    }

    fn is_term(&self) -> bool {
        matches!(
            self,
            CodeBlock::Term
        )
    }
}

pub struct CompilationEngine<T: Tokenizer, S: Serializer> {
    reader: T,
    writer: S,
    section: CodeBlock,
    token: Option<(TokenType, String)>,
    param_wrapper: Option<CodeBlock>,
    function_complete: bool,
    section_updated: bool,
}

impl<T: Tokenizer, S: Serializer> CompilationEngine<T, S> {
    pub fn new(reader: T, writer: S) -> Self {
        Self {
            reader,
            writer,
            section: CodeBlock::Class,
            token: None,
            param_wrapper: None,
            function_complete: false,
            section_updated: false,
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
            self.section = section;
        }
    }

    fn section_after_params(&mut self) {
        self.section_updated = true;
        self.section = self.section.next();
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
                            self.writer.write_node(&name.as_ref(), &value)?;
                        }
                        ")" | "}" => {
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
            self.compile_next()?;
            if self.function_complete {
                break;
            }
        }
        Ok(())
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
        self.function_complete = !self.function_complete;
    }

    pub fn compile_class(&mut self) -> Result<()> {
        self.wrap_compiler()?;
        Ok(())
    }

    pub fn compile_class_var_dec(&mut self) -> Result<()> {
        self.wrap_compiler()?;
        Ok(())
    }

    pub fn compile_subroutine(&mut self) -> Result<()> {
        self.set_params_wrapper();
        self.wrap_compiler()?;
        self.toggle_function();
        Ok(())
    }

    pub fn compile_parameter_list(&mut self) -> Result<()> {
        self.wrap_compiler()?;
        self.section_after_params();
        Ok(())
    }

    pub fn compile_subroutine_body(&mut self) -> Result<()> {
        self.wrap_compiler()?;
        self.toggle_function();
        Ok(())
    }

    pub fn compile_var_dec(&mut self) -> Result<()> {
        self.wrap_compiler()?;
        Ok(())
    }

    pub fn compile_statements(&mut self) -> Result<()> {
        self.wrap_compiler()?;
        Ok(())
    }

    pub fn compile_let(&mut self) -> Result<()> {
        self.set_params_wrapper();
        self.wrap_compiler()?;
        Ok(())
    }

    pub fn compile_if(&mut self) -> Result<()> {
        self.set_params_wrapper();
        self.wrap_compiler()?;
        Ok(())
    }

    pub fn compile_while(&mut self) -> Result<()> {
        self.set_params_wrapper();
        self.wrap_compiler()?;
        Ok(())
    }

    pub fn compile_do(&mut self) -> Result<()> {
        self.set_params_wrapper();
        self.wrap_compiler()?;
        Ok(())
    }

    pub fn compile_return(&mut self) -> Result<()> {
        self.set_params_wrapper();
        self.wrap_compiler()?;
        Ok(())
    }

    pub fn compile_expression(&mut self) -> Result<()> {
        self.wrap_compiler()?;
        Ok(())
    }

    pub fn compile_term(&mut self) -> Result<()> {
        self.set_params_wrapper();
        self.wrap_compiler()?;
        Ok(())
    }

    pub fn compile_expression_list(&mut self) -> Result<()> {
        self.wrap_compiler()?;
        Ok(())
    }
}
