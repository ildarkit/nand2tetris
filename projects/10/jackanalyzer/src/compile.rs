use std::convert::AsRef;
use anyhow::Result;
use crate::serialize::Serializer;
use crate::tokenize::{Tokenizer, TokenType};
use crate::grammar::{Term, CodeBlock};

enum CodeState {
    OpenBlock,
    WriteAndOpenBlock,
    CloseBlock,
    Step,
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

    fn dispatch_statement(&mut self) -> CodeState {
        let Some((_, ref value)) = self.token else {
            return CodeState::Step;
        };
        match value.as_str() {
            ";" => {
                CodeState::CloseBlock
            }
            "(" | "[" => {
                self.start_expression();
                CodeState::WriteAndOpenBlock
            }
            "{" => {
                self.nesting_count += 1;
                if !self.section.is_subroutine_dec() {
                    if !self.section.is_class() {
                        self.toggle_statements(); // true
                    }
                    CodeState::Step
                } else {
                    self.toggle_statements(); // true
                    self.section = CodeBlock::SubroutineBody;
                    CodeState::OpenBlock
                }
            }
            "}" => {
                self.nesting_count -= 1;
                if self.nesting_count == 1 {
                    self.toggle_function(); // true
                }
                CodeState::CloseBlock
            }
            "=" => {
                self.start_expression();
                CodeState::WriteAndOpenBlock
            }
            _ => {
                CodeState::Step
            }
        }
    }

    fn dispatch_expression(&mut self, prev_token: Option<(TokenType, String)>) -> CodeState {
        let Some((ref name, ref value)) = self.token else {
            return CodeState::Step;
        };
        match value.as_str() {
            ";" | ")" | "]" => {
                CodeState::CloseBlock
            }
            "(" => {
                if let Some((ref prev_name, _)) = prev_token && prev_name.is_identifier() {
                    self.section = CodeBlock::ExpressionList;
                } else {
                    self.section = self.section.next();
                }
                CodeState::WriteAndOpenBlock
            }
            "[" => {
                self.section = self.section.next();
                CodeState::WriteAndOpenBlock
            }
            "." => {
                CodeState::Step
            }
            _ => {
                if let Some((_, ref prev_value)) = prev_token && prev_value == "." {
                    CodeState::Step
                } else if Term::try_from(name.as_ref()).is_ok() {
                    self.section = self.section.next();
                    CodeState::OpenBlock
                } else {
                    CodeState::Step
                }
            }
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

    fn compile_block(&mut self) -> Result<bool> {
        let mut prev_token = None;
        if let Some((name, value)) = self.token.take() {
            self.writer.write_node(&name.as_ref(), &value)?;
        }
        while self.reader.advance()? {
            if let Some((name, value)) = self.get_token() {
                let code_state;
                self.token = Some((name, value.clone()));
                if value != "class" && self.section_from(&value)? {
                    return Ok(false);
                } else if self.section.is_all_expressions() {
                    code_state = self.dispatch_expression(prev_token);
                } else {
                    code_state = self.dispatch_statement();
                }

                prev_token = self.token.clone();
                self.write_token(&code_state)?;

                match code_state {
                    CodeState::WriteAndOpenBlock | CodeState::OpenBlock => {
                        return Ok(false);
                    }
                    CodeState::CloseBlock => {
                        return Ok(true);
                    }
                    _ => {}
                }
            }
        }
        Ok(false)
    }

    fn write_token(&mut self, code_state: &CodeState) -> Result<()> {
        match code_state {
            CodeState::WriteAndOpenBlock | CodeState::Step => {
                if let Some((name, value)) = self.token.take() {
                    self.writer.write_node(&name.as_ref(), &value)?;
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
            if !self.section_updated && self.compile_block()? {
                break;
            }
            if self.check_closing_if_statement(&code_block, &mut started) {
                break;
            }
            self.compile_next()?;
            if self.function_completed {
                break;
            }
        }
        Ok(())
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
        let token = if !self.section_updated {
            self.token.take()
        } else {
            None
        };
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
        Ok(())
    }
}
