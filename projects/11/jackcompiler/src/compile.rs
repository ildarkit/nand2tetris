use std::convert::AsRef;
use anyhow::Result;
use strum_macros::{EnumString, AsRefStr, EnumIs};
use crate::serialize::Serializer;
use crate::tokenize::{Tokenizer, TokenType};
use crate::grammar::{Term, CodeBlock, Operation};

#[derive(PartialEq, AsRefStr, EnumIs, EnumString)]
#[strum(serialize_all = "camelCase")]
enum CodeState {
    OpenBlock,
    OpenInnerBlock,
    WriteAndOpenBlock,
    CloseBlock,
    Step,
    CloseWrapperBlock,
    CloseStatement,
    CloseExpression,
}

impl CodeState {
    fn is_closing(&self) -> bool {
        matches!(
            self,
            CodeState::CloseBlock |
            CodeState::CloseWrapperBlock |
            CodeState::CloseStatement |
            CodeState::CloseExpression
        )
    }

    fn is_closing_upper(&self) -> bool {
        matches!(
            self,
            CodeState::CloseWrapperBlock |
            CodeState::CloseStatement |
            CodeState::CloseExpression
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
    closing_token: Option<(TokenType, String)>,
    param_wrapper: Option<CodeBlock>,
    buf_section: Option<CodeBlock>,
    function_completed: bool,
    code_state: CodeState,
    section_updated: bool,
    statements_tag: bool,
    statement_block_count: u32,
    expression_bracket_count: u32,
}

impl<T: Tokenizer, S: Serializer> CompilationEngine<T, S> {
    pub fn new(reader: T, writer: S) -> Self {
        Self {
            reader,
            writer,
            section: CodeBlock::Class,
            token: None,
            prev_token: None,
            closing_token: None,
            param_wrapper: None,
            buf_section: None,
            function_completed: false,
            code_state: CodeState::Step,
            section_updated: false,
            statements_tag: false,
            statement_block_count: 0,
            expression_bracket_count: 0,
        }
    }

    fn inc_bracket_count(&mut self) {
        self.expression_bracket_count += 1;
    }

    fn dec_bracket_count(&mut self) {
        self.expression_bracket_count -= 1;
    }

    fn dispatch_statement(&mut self, token: Option<(TokenType, String)>) {
        let prev_section = self.section.clone();
        let Some((ref name, ref value)) = token else {
            self.code_state = CodeState::Step;
            return;
        };
        match value.as_str() {
            _ if value != "class" && self.section_from(&value) => {
                self.code_state = CodeState::OpenBlock;
                if prev_section.is_subroutine_body() && self.section.is_all_statements() {
                    self.toggle_statements(); // true
                    self.buf_section = Some(self.section.clone());
                    self.section = CodeBlock::Statements;
                }
            }
            ";" => {
                self.code_state = CodeState::CloseBlock;
            }
            ")" => {
                self.dec_bracket_count();
                if self.section.is_while_statement() {
                    self.code_state = CodeState::Step;
                    return;
                }
                self.code_state = CodeState::CloseBlock;
            }
            "(" | "[" => {
                self.inc_bracket_count();
                self.start_expression();
                self.code_state = CodeState::WriteAndOpenBlock;
            }
            "=" => {
                self.start_expression();
                self.code_state = CodeState::WriteAndOpenBlock;
            }
            "{" => {
                self.statement_block_count += 1;
                if self.section.is_subroutine_dec() {
                    self.section = CodeBlock::SubroutineBody;
                    self.code_state = CodeState::OpenBlock;
                } else if self.section.is_class() {
                    self.code_state = CodeState::Step;
                } else {
                    self.toggle_statements(); // true
                    self.section = CodeBlock::Statements;
                    self.code_state = CodeState::WriteAndOpenBlock;
                }
            }
            "}" => {
                self.statement_block_count -= 1;
                if self.statement_block_count == 1 {
                    self.toggle_function(); // true
                }
                self.code_state = CodeState::CloseBlock;
            }
            _ => {
                if Term::try_from(name.as_ref()).is_ok() &&
                    self.section.is_return_statement() {
                    self.start_expression();
                    self.code_state = CodeState::OpenInnerBlock;
                    return;
                }
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
                self.code_state = CodeState::CloseStatement;
            }
            "," => {
                self.code_state = CodeState::CloseBlock;
            }
            ")" | "]" => {
                self.dec_bracket_count();
                if self.expression_bracket_count > 0 {
                    self.code_state = CodeState::CloseExpression;
                } else {
                    self.code_state = CodeState::CloseWrapperBlock;
                }
            }
            "(" => {
                self.inc_bracket_count();
                if let Some((ref prev_name, _)) = prev_token &&
                    prev_name.is_identifier() {
                    self.section = CodeBlock::ExpressionList;
                    self.code_state = CodeState::WriteAndOpenBlock;
                    return;
                }
                if let Some((_, ref prev_value)) = prev_token &&
                    *prev_value == "(" {
                    self.section = self.section.next();
                    self.code_state = CodeState::OpenInnerBlock;
                    return;
                }
                self.section = CodeBlock::Term;
                self.code_state = CodeState::OpenBlock;
            }
            "[" => {
                self.inc_bracket_count();
                self.section = self.section.next();
                self.code_state = CodeState::WriteAndOpenBlock;
            }
            "." => {
                self.code_state = CodeState::Step;
            }
            _ if Operation::try_from(value.as_str()).is_ok() => {
                if let Some((ref prev_name, _)) = prev_token &&
                    prev_name.is_symbol() {
                        // if next section is a term - just open term
                        // else next one is an expression - open inner block
                        self.section = self.section.next();
                        self.code_state = CodeState::OpenInnerBlock;
                } else if let Some((ref prev_name, _)) = prev_token &&
                    Term::try_from(prev_name.as_ref()).is_ok() {
                    self.code_state = CodeState::CloseBlock;
                } else {
                    self.code_state = CodeState::Step;
                }
            }
            _ if Term::try_from(name.as_ref()).is_ok() => {
                if let Some((_, ref prev_value)) = prev_token &&
                    *prev_value == "." {
                    self.code_state = CodeState::Step;
                } else if let Some((_, ref prev_value)) = prev_token &&
                    *prev_value == "(" {
                        self.section = self.section.next();
                        self.code_state = CodeState::OpenInnerBlock;
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

    fn dispatch_expression_list(&mut self, prev_token: Option<(TokenType, String)>) {
        let Some((ref name, ref value)) = self.token else {
            self.code_state = CodeState::Step;
            return;
        };
        match value.as_str() {
            _ if Term::try_from(name.as_ref()).is_ok() => {
                self.buf_section = Some(self.section.clone());
                self.section = CodeBlock::Expression;
                self.code_state = CodeState::OpenInnerBlock; 
            }
            ")" => {
                self.dec_bracket_count();
                self.code_state = CodeState::CloseWrapperBlock;
            }
            "(" => {
                self.inc_bracket_count();
                self.buf_section = Some(self.section.clone());
                self.section = CodeBlock::Expression;
                self.code_state = CodeState::OpenInnerBlock;
            }
            _ if Operation::try_from(value.as_str()).is_ok() => {
                if let Some((ref prev_name, _)) = prev_token &&
                    prev_name.is_symbol() {
                        self.buf_section = Some(self.section.clone());
                        self.section = self.section.next();
                        self.code_state = CodeState::OpenInnerBlock;
                } else if let Some((ref prev_name, _)) = prev_token &&
                    Term::try_from(prev_name.as_ref()).is_ok() {
                    self.code_state = CodeState::CloseBlock;
                } else {
                    self.code_state = CodeState::Step;
                }
            }
            _ => {
                unreachable!();
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

                match self.section {
                    CodeBlock::ExpressionList => {
                        let prev_token = self.prev_token.take();
                        self.dispatch_expression_list(prev_token);
                        if self.code_state.is_close_wrapper_block() {
                            self.closing_token = self.token.take();
                        }
                    }
                    _ if self.section.is_all_expressions() => {
                        let prev_token = self.prev_token.take();
                        self.dispatch_expression(prev_token);
                        if self.code_state.is_closing_upper() {
                            self.closing_token = self.token.take();
                        }
                    }
                    _ => {
                        self.dispatch_statement(current_token);
                    }
                }

                self.prev_token = self.token.clone();
                self.write_token()?;

                if !self.code_state.is_step() {
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
            if self.is_if_statement_closing_statements(&code_block, &mut started) {
                break;
            }
            let prev_section = self.section.clone();
            self.compile_next()?;
            if !self.section_updated {
                self.section = code_block.clone();
            }
            if self.closing_block(&prev_section) {
                break;
            }
        }
        Ok(())
    }

    fn closing_block(&mut self, prev_section: &CodeBlock) -> bool {
        if let Some((_, ref value)) = self.token && value == "," && prev_section.is_term() {
            return true;
        }
        if self.function_completed {
            return true;
        }
        match self.section {
            CodeBlock::ExpressionList => {
                if self.closing_token.is_some() {
                    self.code_state = CodeState::Step;
                    return true;
                }
            }
            CodeBlock::Expression => {
                if self.code_state.is_closing_upper() {
                    return true;
                }
            }
            CodeBlock::Term => {
                if self.code_state.is_closing_upper() {
                    if prev_section.is_expression() {
                        self.code_state = CodeState::Step;
                    }
                    return true;
                }
            }
            CodeBlock::WhileStatement => {
                if self.code_state.is_close_block() {
                    self.code_state = CodeState::Step;
                    return true;
                }
            }
            _ if self.section.is_ending_semicolon() &&
                self.code_state.is_close_statement() => {
                self.code_state = CodeState::Step;
                return true;
            }
            _ => {}
        }
        false
    }

    fn is_if_statement_closing_statements(
        &mut self,
        code_block: &CodeBlock,
        started: &mut bool) -> bool
    {
        if !self.section_updated && self.is_start_statements() {
            *started = true;
        } else if code_block.is_if_statement() && *started {
            // closing if statement
            self.section_updated = true;
            return true;
        }
        false
    }

    fn is_start_statements(&mut self) -> bool {
        if self.section.is_all_statements() && self.statements_tag {
            return true;
        }
        false
    }

    fn expression_closing_token(&mut self) {
        let mut is_closing = false;
        if let Some(ref wrapper) = self.buf_section {
            if (!wrapper.is_expression_list() || self.code_state.is_close_statement()) ||
                (wrapper.is_expression_list() && self.code_state.is_close_expression()) {
                is_closing = true;
            }
        }
        if is_closing && self.closing_token.is_some() {
            self.token = self.closing_token.take();
        }
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
        if code_block.is_expression_list() {
            self.token = self.closing_token.take();
        }
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
        self.put_start_name(&code_block)?;
        if self.code_state.is_open_block() {
            self.section_updated = true;
            if let Some(buf_section) = self.buf_section.take() {
                self.section = buf_section;
            }
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
        let wrapper_block = self.buf_section.clone();
        self.section_updated = false;
        self.put_start_name(&code_block)?;
        if self.code_state == CodeState::OpenInnerBlock {
            self.section = self.section.next();
            self.section_updated = true;
        }
        self.compile()?;
        self.expression_closing_token();
        self.ending_code_block(&code_block)?;
        self.buf_section = wrapper_block;
        self.restore_section();
        Ok(())
    }

    fn compile_term(&mut self) -> Result<()> {
        self.set_params_wrapper();
        self.wrap_compiler()?;
        Ok(())
    }

    fn compile_expression_list(&mut self) -> Result<()> {
        self.set_params_wrapper();
        self.wrap_compiler()?;
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
