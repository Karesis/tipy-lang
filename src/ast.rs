// src/ast.rs

#[derive(Debug, Clone)]
pub enum Expression {
    StringLiteral(String),
    Call {
        callee: String,
        args: Vec<Expression>,
    },
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub body: Vec<Expression>,
}

pub type Program = Vec<Function>;