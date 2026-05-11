use std::convert::AsRef;
use anyhow::Result;
use crate::serialize::Serializer;
use crate::tokenize::{Tokenizer, TokenType};
use crate::grammar::{Term, CodeBlock, Operation};

#[derive(PartialEq)]
enum CodeState {
    OpenBlock,
    OpenInnerBlock,
    WriteAndOpenBlock,
    CloseBlock,
    Step,
    CloseWrapperBlock,
}

impl CodeState {
    fn is_closing(&self) -> bool {
        matches!(
            self,
            CodeState::CloseBlock |
            CodeState::CloseWrapperBlock
        )
    }

    fn is_closing_wrapper(&self) -> bool {
        matches!(
            self,
            CodeState::CloseWrapperBlock
        )
    }
}

trait Output {
    fn put_node<N: AsRef<str>>(&mut self, name: N, value: &str) -> Result<()>;
    fn put_start_name<N: AsRef<str>>(&mut self, name: N) -> Result<()>;
    fn put_end_name<N: AsRef<str>>(&mut self, name: N) -> Result<()>;
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
    prev_token: Option<(TokenType, String)>,
    param_wrapper: Option<CodeBlock>,
    buf_section: Option<CodeBlock>,
    function_completed: bool,
    code_state: CodeState,
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
            prev_token: None,
            param_wrapper: None,
            buf_section: None,
            function_completed: false,
            code_state: CodeState::Step,
            section_updated: false,
            statements_tag: false,
            nesting_count: 0,
        }
    }

    fn dispatch_statement(&mut self, token: Option<(TokenType, String)>) {
        let Some((_, ref value)) = token else {
            self.code_state = CodeState::Step;
            return;
        };
        match value.as_str() {
            _ if value != "class" && self.section_from(&value) => {
                self.code_state = CodeState::OpenBlock;
            }
            ")" => {
                self.code_state = CodeState::CloseBlock;
            }
            ";" => {
                self.code_state = CodeState::CloseWrapperBlock;
            }
            "(" | "[" => {
                self.start_expression();
                self.code_state = CodeState::WriteAndOpenBlock;
            }
            "{" => {
                self.nesting_count += 1;
                if !self.section.is_subroutine_dec() {
                    if !self.section.is_class() {
                        self.toggle_statements(); // true
                    }
                    self.code_state = CodeState::Step;
                } else {
                    self.toggle_statements(); // true
                    self.section = CodeBlock::SubroutineBody;
                    self.code_state = CodeState::OpenBlock;
                }
            }
            "}" => {
                self.nesting_count -= 1;
                if self.nesting_count == 1 {
                    self.toggle_function(); // true
                }
                self.code_state = CodeState::CloseBlock;
            }
            "=" => {
                self.start_expression();
                self.code_state = CodeState::WriteAndOpenBlock;
            }
            _ => {
                self.code_state = CodeState::Step;
            }
        }
    }

    fn dispatch_expression(&mut self, prev_token: Option<(TokenType, String)>) {
        let Some((ref name, ref value)) = self.token else {
            self.code_state = CodeState::Step;
            return;
        };
        match value.as_str() {
            ";" => {
                self.code_state = CodeState::CloseWrapperBlock;
            }
            ")" | "]" => {
                self.code_state = CodeState::CloseBlock;
            }
            "(" => {
                if let Some((ref prev_name, _)) = prev_token &&
                    prev_name.is_identifier() {
                    self.section = CodeBlock::ExpressionList;
                    self.code_state = CodeState::WriteAndOpenBlock;
                } else {
                    self.section = CodeBlock::Term;
                    self.code_state = CodeState::OpenBlock;
                }
            }
            "[" => {
                self.section = self.section.next();
                self.code_state = CodeState::WriteAndOpenBlock;
            }
            "." => {
                self.code_state = CodeState::Step;
            }
            _ if Operation::try_from(value.as_str()).is_ok() => {
                if let Some((ref prev_name, _)) = prev_token &&
                    prev_name.is_symbol() {
                        self.section = self.section.next();
                        self.code_state = CodeState::OpenInnerBlock;
                } else {
                    self.code_state = CodeState::CloseBlock;
                }
            }
            _ if Term::try_from(name.as_ref()).is_ok() => {
                if let Some((_, ref prev_value)) = prev_token &&
                    *prev_value == "." {
                    self.code_state = CodeState::Step;
                } else {
                    self.section = CodeBlock::Term;
                    self.code_state = CodeState::OpenBlock;
                }
            }
            _ => {
                if let Some((ref prev_name, _)) = prev_token &&
                    prev_name.is_symbol() {
                        self.section = self.section.next();
                        self.code_state = CodeState::OpenBlock;
                } else {
                    self.code_state = CodeState::Step;
                }
            }
        }
    }

    fn section_from(&mut self, name: &str) -> bool {
        CodeBlock::try_from(name).map(|b| self.section = b).is_ok()
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

    fn start_expression(&mut self) {
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

    fn compile_block(&mut self) -> Result<()> {
        if let Some((name, value)) = self.token.take() {
            self.put_node(&name, &value)?;
        }
        while self.reader.advance()? {
            let current_token = self.get_token();
            if current_token.is_some() {
                self.token = current_token.clone();

                if self.section.is_all_expressions() {
                    let prev_token = self.prev_token.take();
                    self.dispatch_expression(prev_token);
                } else {
                    self.dispatch_statement(current_token);
                }

                self.prev_token = self.token.clone();
                self.write_token()?;

                if self.code_state != CodeState::Step {
                    break;
                }
            }
        }
        Ok(())
    }

    fn write_token(&mut self) -> Result<()> {
        match self.code_state {
            CodeState::WriteAndOpenBlock | CodeState::Step => {
                if let Some((name, value)) = self.token.take() {
                    self.put_node(&name, &value)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn compile(&mut self) -> Result<()> {
        let code_block = self.section.clone();
        let mut started = false;
        loop {
            if !self.section_updated {
                self.compile_block()?;
                if self.code_state.is_closing() {
                    break;
                }
            }
            if self.check_closing_if_statement(&code_block, &mut started) {
                break;
            }
            let section = self.section.clone();
            self.compile_next()?;
            if self.is_closing_block(&section) {
                break;
            }
        }
        Ok(())
    }

    fn is_closing_block(&self, section: &CodeBlock) -> bool {
        if section.is_term() {
            if self.code_state.is_closing_wrapper() {
                return true;
            }
            if let Some((_, ref value)) = self.token && value == ")" {
                return true;
            }
        }
        if (section.is_ending_semicolon() || section.is_all_expressions()) &&
            self.code_state.is_closing_wrapper() {
                return true;
        }
        if self.function_completed {
            return true;
        }
        false
    }

    fn check_closing_if_statement(
        &mut self,
        code_block: &CodeBlock,
        started: &mut bool) -> bool
    {
        if !self.section_updated && self.start_statements() {
            *started = true;
        } else if code_block.is_if_statement() && *started {
            // closing if statement
            self.section_updated = true;
            return true;
        }
        false
    }

    fn start_statements(&mut self) -> bool {
        if self.section.is_all_statements() && self.statements_tag {
            self.buf_section = Some(self.section.clone());
            self.section = CodeBlock::Statements;
            return true;
        }
        false
    }

    fn ending_code_block(&mut self, block: &CodeBlock) -> Result<()> {
        let token = if !(self.section_updated || block.is_term()) {
            self.token.take()
        } else {
            None
        };
        if block.is_outside_closing() {
            if let Some((name, value)) = token {
                self.put_node(&name, &value)?;
            }
            self.put_end_name(block)?;
        } else {
            self.put_end_name(block)?;
            if let Some((name, value)) = token {
                self.put_node(&name, &value)?;
            }
        }
        Ok(())
    }

    fn wrap_compiler(&mut self) -> Result<()> {
        let code_block = self.section.clone();
        self.section_updated = false;
        self.put_start_name(&code_block)?;
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
        self.put_start_name(&code_block)?;
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
        self.code_state = CodeState::Step;
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
        self.code_state = CodeState::Step;
        Ok(())
    }

    fn compile_expression(&mut self) -> Result<()> {
        let code_block = self.section.clone();
        self.section_updated = false;
        self.put_start_name(&code_block)?;
        if self.code_state == CodeState::OpenInnerBlock {
            self.section = self.section.next();
            self.section_updated = true;
        }
        self.compile()?;
        self.ending_code_block(&code_block)?;
        self.restore_section();
        Ok(())
    }

    fn compile_term(&mut self) -> Result<()> {
        self.set_params_wrapper();
        self.wrap_compiler()?;
        Ok(())
    }

    fn compile_expression_list(&mut self) -> Result<()> {
        self.wrap_compiler()?;
        self.restore_section();
        Ok(())
    }
}


impl<T: Tokenizer, S: Serializer> Output for CompilationEngine<T, S> {
    fn put_node<N: AsRef<str>>(&mut self, name: N, value: &str) -> Result<()> {
        self.writer.write_node(name.as_ref(), value)?;
        Ok(())
    }

    fn put_start_name<N: AsRef<str>>(&mut self, name: N) -> Result<()> {
        self.writer.write_name(name.as_ref())?;
        Ok(())
    }

    fn put_end_name<N: AsRef<str>>(&mut self, name: N) -> Result<()> {
        self.writer.end_name(name.as_ref())?;
        Ok(())
    }
}
