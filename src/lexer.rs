use crate::token::{Token, Keyword, Literal};

pub struct Lexer<'a> {
    input: &'a [u8],
    position: usize,
    read_position: usize,
    ch: u8,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        let mut lexer = Lexer {
            input: input.as_bytes(),
            position: 0,
            read_position: 0,
            ch: 0,
        };
        lexer.read_char();
        lexer
    }

    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace(); // 在解析下一个 Token 前，先跳过所有空白

        let token = match self.ch {
            b'=' => Token::Equal,
            b'+' => Token::Plus,
            b'-' => Token::Minus,
            b'*' => Token::Star,
            b'/' => Token::Slash,
            b'~' => Token::Tilde,
            b':' => Token::Colon,
            b';' => Token::Semicolon,
            b',' => Token::Comma,
            b'(' => Token::LParen,
            b')' => Token::RParen,
            b'{' => Token::LBrace,
            b'}' => Token::RBrace,
            b'"' => self.read_string(), 
            0 => Token::Eof, // 0 (NUL) 代表文件结束
            _ => {
                if is_letter(self.ch) {
                    // 如果是字母，读取整个标识符
                    let identifier = self.read_identifier();
                    // 然后查找这个标识符是不是一个关键字
                    return lookup_ident(&identifier);
                } else if is_digit(self.ch) {
                    return self.read_number();
                } else {
                    // 如果都不是，那就是一个非法字符
                    Token::Illegal(self.ch as char)
                }
            }
        };

        if token != Token::Eof {
            self.read_char();
        }
        
        token
    }

    // --- 私有函数 ---

    fn read_char(&mut self) {
        if self.read_position >= self.input.len() {
            self.ch = 0; // 0 is just a NUL for u8
        } else {
            self.ch = self.input[self.read_position];
        };
        self.position = self.read_position;
        self.read_position += 1;
    }

    // 跳过空白
    fn skip_whitespace(&mut self) {
        while self.ch.is_ascii_whitespace() {
            self.read_char();
        }
    }

    // 识别标识符
    fn read_identifier(&mut self) -> String {
        let start_pos = self.position;
        while is_letter(self.ch) || is_digit(self.ch) {
            self.read_char();
        }
        String::from_utf8_lossy(&self.input[start_pos..self.position]).to_string()
    }

    // 识别字符串字面量
    fn read_string(&mut self) -> Token {
        let start_pos = self.position + 1;

        loop {
            self.read_char();
            if self.ch == b'"' || self.ch == 0 {
                break;
            }
        }

        if self.ch == 0 {
            return Token::Illegal('"');
        }

        let content = String::from_utf8_lossy(&self.input[start_pos..self.position]).to_string();
        Token::Literal(Literal::String(content))
    }

    // 识别一个整数或者浮点数
    fn read_number(&mut self) -> Token {
        let start_pos = self.position;
        // 1. 读取数字的整数部分
        while is_digit(self.ch) {
            self.read_char();
        }

        // 2. 检查是否有小数部分
        if self.ch == b'.' {
            self.read_char(); // 跳过 '.'
            while is_digit(self.ch) {
                self.read_char();
            }
            // 此时，从 start_pos 到 self.position 是一个完整的浮点数字符串
            let full_number_str = std::str::from_utf8(&self.input[start_pos..self.position]).unwrap();
            // 解析成 f64
            let float_val = full_number_str.parse::<f64>().unwrap();
            return Token::Literal(Literal::Float(float_val));
        }

        // 如果没有小数部分，它就是一个整数
        let full_number_str = std::str::from_utf8(&self.input[start_pos..self.position]).unwrap();
        let int_val = full_number_str.parse::<i64>().unwrap();
        Token::Literal(Literal::Integer(int_val))
    }
}

// 辅助识别的一些函数
fn is_letter(ch: u8) -> bool {
    ch.is_ascii_alphabetic() || ch == b'_'
}

fn is_digit(ch: u8) -> bool {
    b'0' <= ch && ch <= b'9'
}

fn lookup_ident(ident: &str) -> Token {
    match ident {
        "main" => Token::Keyword(Keyword::Main),
        "print" => Token::Keyword(Keyword::Print),
        _ => Token::Identifier(ident.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::{Keyword, Literal};

    #[test]
    fn test_hello_world() {
        let input = r#"
        main() {
            print("Hello, Tipy World!")
        }
        "#;

        let expected_tokens = vec![
            Token::Keyword(Keyword::Main),
            Token::LParen,
            Token::RParen,
            Token::LBrace,
            Token::Keyword(Keyword::Print),
            Token::LParen,
            Token::Literal(Literal::String("Hello, Tipy World!".to_string())),
            Token::RParen,
            Token::RBrace,
            Token::Eof,
        ];

        let mut lexer = Lexer::new(input);

        for expected_token in expected_tokens {
            let token = lexer.next_token();
            println!("Generated: {:?}, Expected: {:?}", token, expected_token); // 打印出来方便调试
            assert_eq!(token, expected_token);
        }
    }
}

#[test]
fn test_calculator_tokens() {
    let input = "v: ~i32 = 5 + 10 * 2.5 / 1;";

    let expected_tokens = vec![
        Token::Identifier("v".to_string()),
        Token::Colon,
        Token::Tilde,
        Token::Identifier("i32".to_string()), // 假设 i32 被当作标识符
        Token::Equal,
        Token::Literal(Literal::Integer(5)),
        Token::Plus,
        Token::Literal(Literal::Integer(10)),
        Token::Star,
        Token::Literal(Literal::Float(2.5)),
        Token::Slash,
        Token::Literal(Literal::Integer(1)),
        Token::Semicolon, // 如果你实现了分号
        Token::Eof,
    ];

    let mut lexer = Lexer::new(input);

    for expected_token in expected_tokens {
        let token = lexer.next_token();
        assert_eq!(token, expected_token);
    }
}
