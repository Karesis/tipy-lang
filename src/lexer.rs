// src/lexer.rs

use logos::Logos;

#[derive(Logos, Debug, PartialEq, Clone, Eq, Hash)]
#[logos(skip r"[ \t\n\f]+")]
#[logos(skip r"//[^\n]*")]
pub enum Token {
    #[token("(")]
    ParenOpen,

    #[token(")")]
    ParenClose,

    #[token("{")]
    BraceOpen,

    #[token("}")]
    BraceClose,

    #[token(",")]
    Comma,

    #[regex("\"[^\"]*\"", |lex| lex.slice()[1..lex.slice().len() - 1].to_string())]
    String(String),

    #[regex("[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Identifier(String),
}