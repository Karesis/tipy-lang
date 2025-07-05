#[derive(Debug, Clone, PartialEq)]
pub enum Keyword {
    Main,
    Print
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    String(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Eof, // end of file
    Illegal(char), // illegal charactor

    Identifier(String),
    Literal(Literal),
    Keyword(Keyword),

    LParen,  // (
    RParen,  // )
    LBrace,  // {
    RBrace,  // }
    Comma,   // ,
}