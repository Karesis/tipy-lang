// src/parser.rs

use chumsky::prelude::*;
use crate::lexer::Token;
use crate::ast::{Expression, Function, Program};

pub fn program_parser() -> impl Parser<Token, Program, Error = Simple<Token>> {
    let ident = select! { Token::Identifier(s) => s };
    
    let expr = recursive(|expr| {
        let string_literal = select! { Token::String(s) => Expression::StringLiteral(s.clone()) };

        let call = ident
            .then_ignore(just(Token::ParenOpen))
            .then(
                expr.clone()
                    .separated_by(just(Token::Comma))
                    .allow_trailing()
            )
            .then_ignore(just(Token::ParenClose))
            .map(|(callee, args)| Expression::Call { callee, args });

        call.or(string_literal)
    });

    let function = ident
        .then_ignore(just(Token::ParenOpen))
        .then_ignore(just(Token::ParenClose))
        .then(
            expr
                .repeated()
                .delimited_by(just(Token::BraceOpen), just(Token::BraceClose))
        )
        .map(|(name, body)| Function { name, body });

    function.repeated().at_least(1).then_ignore(end())
}