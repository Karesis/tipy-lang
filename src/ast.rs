use crate::token::Literal;

#[derive(Debug, PartialEq)]
pub enum Expression {
    Identifier(String),
    Literal(Literal),
    Call {
        function: Box<Expression>,
        arguments: Vec<Expression>,
    },
}

#[derive(Debug, PartialEq)]
pub enum Statement {
    Function {
        name: Expression,
        body: Vec<Statement>,
    },
    Expression(Expression),
}

#[derive(Debug, PartialEq)]
pub struct Program {
    pub statements: Vec<Statement>,
}

impl Program {
    pub fn new() -> Self {
        Program { 
            statements: Vec::new(), 
        }  
    }
}