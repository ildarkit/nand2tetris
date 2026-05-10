use strum_macros::{EnumString, AsRefStr, EnumIs};

#[derive(AsRefStr, Debug, PartialEq, Eq, EnumString, Clone)]
#[strum(serialize_all = "camelCase")]
pub enum Term {
    #[strum(serialize = "identifier")]
    Identifier,
    #[strum(serialize = "keyword")]
    Keyword,
    #[strum(serialize = "integerConstant")]
    IntegerConstant,
    #[strum(serialize = "stringConstant")]
    StringConstant,
}

#[derive(AsRefStr, Debug, PartialEq, Eq, EnumIs, EnumString, Clone)]
#[strum(serialize_all = "camelCase")]
pub enum CodeBlock {
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
    pub fn next(&self) -> Self {
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

    pub fn is_outside_closing(&self) -> bool {
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

    pub fn is_all_statements(&self) -> bool {
        matches!(
            self,
            CodeBlock::LetStatement |
            CodeBlock::DoStatement |
            CodeBlock::ReturnStatement |
            CodeBlock::IfStatement |
            CodeBlock::WhileStatement
        )
    }

    pub fn is_all_expressions(&self) -> bool {
        matches!(
            self,
            CodeBlock::Expression |
            CodeBlock::Term |
            CodeBlock::ExpressionList
        )
    }

    pub fn is_ending_semicolon(&self) -> bool {
        matches!(
            self,
            CodeBlock::LetStatement |
            CodeBlock::ReturnStatement
        )
    }
}

