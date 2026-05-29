use anyhow::Result;
use crate::vm_writer::{Segment, Command, VMCommandWriter};
use crate::symbol_table::{Kind, SymbolTable};
use crate::parser::{ParserError, Parser};

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
    fn compile_expression_list(&mut self) -> Result<usize>;
}

pub struct CompilationEngine<T: Parser, W: VMCommandWriter> {
    parser: T,
    writer: W,
    symbol_table: SymbolTable,
    class_name: String,
    current_subroutine_name: String,
    current_subroutine_type: String,
    label_index: u32,
}

impl<T: Parser, W: VMCommandWriter> CompilationEngine<T, W> {
    pub fn new(parser: T, writer: W) -> Self {
        Self {
            parser,
            writer,
            symbol_table: SymbolTable::new(),
            class_name: String::new(),
            current_subroutine_name: String::new(),
            current_subroutine_type: String::new(),
            label_index: 0,
        }
    }

    fn push_variable_by_name(&mut self, name: &str) -> Result<()> {
        let kind = self.symbol_table.kind_of(name)
            .ok_or_else(|| ParserError::UndefinedVariable(name.to_string()))
            .map_err(anyhow::Error::from)?;
        let index = self.symbol_table.index_of(name).unwrap();
        self.writer.write_push(self.kind_to_segment(kind), index)?;
        Ok(())
    }

    fn pop_variable_by_name(&mut self, name: &str) -> Result<()> {
        let kind = self.symbol_table.kind_of(name)
            .ok_or_else(|| ParserError::UndefinedVariable(name.to_string()))
            .map_err(anyhow::Error::from)?;
        let index = self.symbol_table.index_of(name).unwrap();
        self.writer.write_pop(self.kind_to_segment(kind), index)?;
        Ok(())
    }

    fn kind_to_segment(&self, kind: Kind) -> Segment {
        match kind {
            Kind::Var => Segment::Local,
            Kind::Arg => Segment::Argument,
            Kind::Field => Segment::This,
            Kind::Static => Segment::Static,
        }
    }
}

impl<T: Parser, W: VMCommandWriter> Compiler for CompilationEngine<T, W> {
    fn compile_class(&mut self) -> Result<()> {
        self.parser.expect_keyword("class")?;
        self.class_name = self.parser.expect_identifier()?;
        self.parser.expect_symbol("{")?;

        while self.parser.peek_keyword_matches(&["static", "field"]) {
            self.compile_class_var_dec()?;
        }

        while self.parser.peek_keyword_matches(&["constructor", "function", "method"]) {
            self.compile_subroutine()?;
        }

        self.parser.expect_symbol("}")?;
        self.writer.close()?;
        Ok(())
    }

    fn compile_class_var_dec(&mut self) -> Result<()> {
        let kind_str = self.parser.expect_keyword_choices(&["static", "field"])?;
        let kind = match kind_str.as_str() {
            "static" => Kind::Static,
            "field" => Kind::Field,
            _ => unreachable!(),
        };

        let var_type = self.parser.parse_type()?;
        let var_name = self.parser.expect_identifier()?;
        self.symbol_table.define(&var_name, &var_type, kind);

        while self.parser.peek_symbol_matches(",") {
            self.parser.expect_symbol(",")?;
            let next_name = self.parser.expect_identifier()?;
            self.symbol_table.define(&next_name, &var_type, kind);
        }

        self.parser.expect_symbol(";")?;
        Ok(())
    }

    fn compile_subroutine(&mut self) -> Result<()> {
        self.symbol_table.reset();

        self.current_subroutine_type = self.parser
            .expect_keyword_choices(&["constructor", "function", "method"])?;
        let _return_type = self.parser.parse_type()?;
        self.current_subroutine_name = self.parser.expect_identifier()?;

        if self.current_subroutine_type == "method" {
            self.symbol_table.define("this", &self.class_name, Kind::Arg);
        }

        self.parser.expect_symbol("(")?;
        self.compile_parameter_list()?;
        self.parser.expect_symbol(")")?;

        self.compile_subroutine_body()?;
        Ok(())
    }

    fn compile_parameter_list(&mut self) -> Result<()> {
        if !self.parser.peek_symbol_matches(")") {
            let var_type = self.parser.parse_type()?;
            let var_name = self.parser.expect_identifier()?;
            self.symbol_table.define(&var_name, &var_type, Kind::Arg);

            while self.parser.peek_symbol_matches(",") {
                self.parser.expect_symbol(",")?;
                let next_type = self.parser.parse_type()?;
                let next_name = self.parser.expect_identifier()?;
                self.symbol_table.define(&next_name, &next_type, Kind::Arg);
            }
        }
        Ok(())
    }

    fn compile_subroutine_body(&mut self) -> Result<()> {
        self.parser.expect_symbol("{")?;

        while self.parser.peek_keyword_matches(&["var"]) {
            self.compile_var_dec()?;
        }

        let n_locals = self.symbol_table.var_count(Kind::Var);
        let full_name = format!("{}.{}", self.class_name, self.current_subroutine_name);
        self.writer.write_function(&full_name, n_locals)?;

        match self.current_subroutine_type.as_str() {
            "constructor" => {
                let fields_count = self.symbol_table.var_count(Kind::Field);
                self.writer.write_push(Segment::Const, fields_count)?;
                self.writer.write_call("Memory.alloc", 1)?;
                self.writer.write_pop(Segment::Pointer, 0)?;
            }
            "method" => {
                self.writer.write_push(Segment::Argument, 0)?;
                self.writer.write_pop(Segment::Pointer, 0)?;
            }
            _ => {}
        }

        self.compile_statements()?;
        self.parser.expect_symbol("}")?;
        Ok(())
    }

    fn compile_var_dec(&mut self) -> Result<()> {
        self.parser.expect_keyword("var")?;
        let var_type = self.parser.parse_type()?;
        let var_name = self.parser.expect_identifier()?;
        self.symbol_table.define(&var_name, &var_type, Kind::Var);

        while self.parser.peek_symbol_matches(",") {
            self.parser.expect_symbol(",")?;
            let next_name = self.parser.expect_identifier()?;
            self.symbol_table.define(&next_name, &var_type, Kind::Var);
        }

        self.parser.expect_symbol(";")?;
        Ok(())
    }

    fn compile_statements(&mut self) -> Result<()> {
        while self.parser.peek_keyword_matches(&["let", "if", "while", "do", "return"]) {
            let kw = self.parser.peek_keyword()?;
            match kw.as_str() {
                "let" => self.compile_let()?,
                "if" => self.compile_if()?,
                "while" => self.compile_while()?,
                "do" => self.compile_do()?,
                "return" => self.compile_return()?,
                _ => unreachable!(),
            }
        }
        Ok(())
    }

    fn compile_let(&mut self) -> Result<()> {
        self.parser.expect_keyword("let")?;
        let var_name = self.parser.expect_identifier()?;
        let is_array = self.parser.peek_symbol_matches("[");

        if is_array {
            self.push_variable_by_name(&var_name)?;
            self.parser.expect_symbol("[")?;
            self.compile_expression()?;
            self.parser.expect_symbol("]")?;
            self.writer.write_arithmetic(Command::Add)?;

            self.parser.expect_symbol("=")?;
            self.compile_expression()?;

            self.writer.write_pop(Segment::Temp, 0)?;
            self.writer.write_pop(Segment::Pointer, 1)?;
            self.writer.write_push(Segment::Temp, 0)?;
            self.writer.write_pop(Segment::That, 0)?;
        } else {
            self.parser.expect_symbol("=")?;
            self.compile_expression()?;
            self.pop_variable_by_name(&var_name)?;
        }

        self.parser.expect_symbol(";")?;
        Ok(())
    }

    fn compile_if(&mut self) -> Result<()> {
        let label_else = format!("IF_ELSE{}", self.label_index);
        let label_end = format!("IF_END{}", self.label_index);
        self.label_index += 1;

        self.parser.expect_keyword("if")?;
        self.parser.expect_symbol("(")?;
        self.compile_expression()?;
        self.parser.expect_symbol(")")?;

        self.writer.write_arithmetic(Command::Not)?;
        self.writer.write_if(&label_else)?;

        self.parser.expect_symbol("{")?;
        self.compile_statements()?;
        self.parser.expect_symbol("}")?;
        self.writer.write_goto(&label_end)?;

        self.writer.write_label(&label_else)?;
        if self.parser.peek_keyword_matches(&["else"]) {
            self.parser.expect_keyword("else")?;
            self.parser.expect_symbol("{")?;
            self.compile_statements()?;
            self.parser.expect_symbol("}")?;
        }
        self.writer.write_label(&label_end)?;
        Ok(())
    }

    fn compile_while(&mut self) -> Result<()> {
        let label_exp = format!("WHILE_EXP{}", self.label_index);
        let label_end = format!("WHILE_END{}", self.label_index);
        self.label_index += 1;

        self.parser.expect_keyword("while")?;
        self.writer.write_label(&label_exp)?;

        self.parser.expect_symbol("(")?;
        self.compile_expression()?;
        self.parser.expect_symbol(")")?;

        self.writer.write_arithmetic(Command::Not)?;
        self.writer.write_if(&label_end)?;

        self.parser.expect_symbol("{")?;
        self.compile_statements()?;
        self.parser.expect_symbol("}")?;

        self.writer.write_goto(&label_exp)?;
        self.writer.write_label(&label_end)?;
        Ok(())
    }

    fn compile_do(&mut self) -> Result<()> {
        self.parser.expect_keyword("do")?;
        let name = self.parser.expect_identifier()?;
        let next_char = self.parser.peek_next_char();
        let mut arg_count = 0;
        let full_name;

        match next_char {
            // Вызов метода текущего класса:
            // do foo(args) -> переводится в ТекущийКласс.foo(this, args)
            "(" => {
                self.parser.expect_symbol("(")?;
                self.writer.write_push(Segment::Pointer, 0)?;
                arg_count += 1;

                arg_count += self.compile_expression_list()?;
                self.parser.expect_symbol(")")?;

                full_name = format!("{}.{}", self.class_name, name);
            }
            // Вызов вида: do X.foo(args)
            "." => {
                self.parser.expect_symbol(".")?;
                let sub_name = self.parser.expect_identifier()?;
                self.parser.expect_symbol("(")?;

                // Проверяем, является ли X переменной (объектом)
                if let Some(var_type) = self.symbol_table.type_of(&name) {
                    self.push_variable_by_name(&name)?;
                    arg_count += 1;
                    full_name = format!("{}.{}", var_type, sub_name);
                } else {
                    // Если в таблице символов нет такого имени,
                    // значит X — это имя Класса (статический вызов)
                    full_name = format!("{}.{}", name, sub_name);
                }

                arg_count += self.compile_expression_list()?;
                self.parser.expect_symbol(")")?;
            }
            _ => return Err(
                ParserError::SyntaxError(
                    "Expected '(' or '.' after identifier in 'do' statement".to_string())
                        .into()
                ),
        }

        self.writer.write_call(&full_name, arg_count)?;
        // Любой вызов функции в Jack возвращает значение на стек (даже void возвращает 0).
        // Так как это оператор do, результат нам не нужен — сбрасываем его в temp 0.
        self.writer.write_pop(Segment::Temp, 0)?;
        self.parser.expect_symbol(";")?;
        Ok(())
    }

    fn compile_return(&mut self) -> Result<()> {
        self.parser.expect_keyword("return")?;

        // Если сразу идет ';', значит это void-возврат
        if self.parser.peek_symbol_matches(";") {
            // Void-функции в Jack всегда возвращают константу 0
            self.writer.write_push(Segment::Const, 0)?;
        } else {
            // Иначе вычисляем выражение и кладем его результат на стек
            self.compile_expression()?;
        }

        self.parser.expect_symbol(";")?;
        self.writer.write_return()?;
        Ok(())
    }

    fn compile_expression_list(&mut self) -> Result<usize> {
        let mut count = 0;
        if !self.parser.peek_symbol_matches(")") {
            self.compile_expression()?;
            count += 1;
            while self.parser.peek_symbol_matches(",") {
                self.parser.expect_symbol(",")?;
                self.compile_expression()?;
                count += 1;
            }
        }
        Ok(count)
    }

    fn compile_term(&mut self) -> Result<()> {
        if self.parser.peek_is_int_const() {
            let val = self.parser.get_int_const()?;
            self.writer.write_push(Segment::Const, val)?;
        } 
        else if self.parser.peek_is_string_const() {
            let string_const = self.parser.get_string_const()?;
            self.writer.write_push(Segment::Const, string_const.len())?;
            self.writer.write_call("String.new", 1)?;
            for c in string_const.chars() {
                self.writer.write_push(Segment::Const, c as usize)?;
                self.writer.write_call("String.appendChar", 2)?;
            }
        } 
        else if self.parser.peek_keyword_matches(&["true", "false", "null", "this"]) {
            let kw = self.parser.get_keyword()?;
            match kw.as_str() {
                "false" | "null" => self.writer.write_push(Segment::Const, 0)?,
                "true" => {
                    self.writer.write_push(Segment::Const, 0)?;
                    self.writer.write_arithmetic(Command::Not)?; // -1 в Jack
                }
                "this" => self.writer.write_push(Segment::Pointer, 0)?,
                _ => return Err(ParserError::UnexpectedKeyword(kw).into()),
            }
        } 
        else if self.parser.peek_symbol_matches("(") {
            self.parser.expect_symbol("(")?;
            self.compile_expression()?;
            self.parser.expect_symbol(")")?;
        } 
        else if self.parser.peek_symbol_matches_choices(&["-", "~"]) {
            let unary_op = self.parser.get_symbol()?;
            self.compile_term()?;
            match unary_op.as_str() {
                "-" => self.writer.write_arithmetic(Command::Neg),
                "~" => self.writer.write_arithmetic(Command::Not),
                _ => unreachable!(),
            }?
        } 
        else if self.parser.peek_is_identifier() {
            let name = self.parser.expect_identifier()?;
            let next_char = self.parser.peek_next_char(); 

            match next_char {
                // Элемент массива: name[expression]
                "[" => {
                    self.push_variable_by_name(&name)?; // Кладем базовый адрес массива на стек
                    self.parser.expect_symbol("[")?;
                    self.compile_expression()?;        // Считаем индекс внутри [ ]
                    self.parser.expect_symbol("]")?;
                    self.writer.write_arithmetic(Command::Add)?; // base + index

                    // Переносим вычисленный адрес в pointer 1 (segment THAT)
                    self.writer.write_pop(Segment::Pointer, 1)?;
                    // Читаем значение из памяти по этому адресу
                    self.writer.write_push(Segment::That, 0)?;
                }
                // Вызов метода текущего класса: name(expressionList)
                "(" => {
                    self.parser.expect_symbol("(")?;
                    // Так как это метод текущего класса, передаем 'this' первым аргументом
                    self.writer.write_push(Segment::Pointer, 0)?;
                    let arg_count = self.compile_expression_list()?;
                    self.parser.expect_symbol(")")?;

                    let full_name = format!("{}.{}", self.class_name, name);
                    self.writer.write_call(&full_name, arg_count + 1)?;
                }
                // Вызов метода другого объекта ИЛИ статической функции: name.subroutine(exprList)
                "." => {
                    self.parser.expect_symbol(".")?;
                    let sub_name = self.parser.expect_identifier()?;
                    self.parser.expect_symbol("(")?;

                    let mut arg_count = 0;
                    let full_name = if let Some(var_type) = self.symbol_table.type_of(&name) {
                        // Если `name` есть в таблице символов, значит это объект (метод).
                        // Нам нужно передать сам объект в качестве первого аргумента.
                        self.push_variable_by_name(&name)?;
                        arg_count += 1;
                        format!("{}.{}", var_type, sub_name) // ИмяКласса.имяМетода
                    } else {
                        // Иначе это имя класса (вызов статической функции, например Math.sqrt)
                        format!("{}.{}", name, sub_name)
                    };

                    arg_count += self.compile_expression_list()?;
                    self.parser.expect_symbol(")")?;
                    self.writer.write_call(&full_name, arg_count)?;
                }
                // Просто переменная
                _ => {
                    self.push_variable_by_name(&name)?;
                }
            }
        } else {
            return Err(ParserError::SyntaxError("Invalid term".to_string()).into());
        }
        Ok(())
    }

    fn compile_expression(&mut self) -> Result<()> {
        self.compile_term()?;

        while self.parser.peek_symbol_matches_choices(
            &["+", "-", "*", "/", "&", "|", "<", ">", "="]) {
            let op = self.parser.get_symbol()?;
            self.compile_term()?;
            match op.as_str() {
                "+" => self.writer.write_arithmetic(Command::Add),
                "-" => self.writer.write_arithmetic(Command::Sub),
                "&" => self.writer.write_arithmetic(Command::And),
                "|" => self.writer.write_arithmetic(Command::Or),
                "<" => self.writer.write_arithmetic(Command::Lt),
                ">" => self.writer.write_arithmetic(Command::Gt),
                "=" => self.writer.write_arithmetic(Command::Eq),
                "*" => self.writer.write_call("Math.multiply", 2),
                "/" => self.writer.write_call("Math.divide", 2),
                _ => return Err(ParserError::InvalidOperator(op).into()),
            }?
        }
        Ok(())
    }
}
