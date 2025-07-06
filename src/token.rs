#[derive(Debug, Clone, PartialEq)]
pub enum Keyword {
    Main,
    Print
}

// 字面量
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    String(String),
    Integer(i64),
    Float(f64),
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

    Equal,      // =
    Plus,       // +
    Minus,      // -
    Star,       // *
    Slash,      // /
    
    Tilde,      // ~ (可变性标记)
    Colon,      // :
    Semicolon,  // ; (如果想把多行写道一起)
}