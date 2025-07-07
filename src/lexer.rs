// src/lexer.rs

use crate::token::{Token, Keyword, Literal};

pub struct Lexer<'a> {
    input: &'a [u8],
    position: usize,      // 当前正在检查的字符的位置 (points to current char)
    read_position: usize, // 即将读取的下一个字符的位置 (points to next char)
    ch: u8,               // 当前正在检查的字符
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

    // NEW: 添加一个辅助函数 peek_char，用于“偷看”下一个字符而不移动指针。
    // 这对于解析像 `->` 这样的多字符 Token 至关重要。
    fn peek_char(&self) -> u8 {
        if self.read_position >= self.input.len() {
            0
        } else {
            self.input[self.read_position]
        }
    }

    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace_and_comments(); // UPDATED: 函数名变更，功能增强

        // UPDATED: 调整了 match 表达式，以处理新的 Token 和多字符 Token
        let token = match self.ch {
            b'=' => Token::Equal,
            b'+' => Token::Plus,
            b'*' => Token::Star,
            b'~' => Token::Tilde,
            b':' => Token::Colon,
            b';' => Token::Semicolon,
            b',' => Token::Comma,
            b'(' => Token::LParen,
            b')' => Token::RParen,
            b'{' => Token::LBrace,
            b'}' => Token::RBrace,
            b'^' => Token::Caret, // NEW: 增加了对 '^' 的支持
            b'|' => Token::Pipe,  // NEW: 增加了对 '|' 的支持
            b'"' => self.read_string(),

            // UPDATED: 增强了对 `-` 和 `/` 的处理
            b'-' => {
                if self.peek_char() == b'>' {
                    self.read_char(); // 消耗当前的 '-'
                    Token::Arrow // 返回 Arrow Token
                } else {
                    Token::Minus
                }
            }
            b'/' => {
                // 注释被 skip_whitespace_and_comments 处理，这里只处理除法
                Token::Slash
            }
            
            0 => Token::Eof,
            _ => {
                if is_letter(self.ch) {
                    let identifier = self.read_identifier();
                    // lookup_ident 现在会返回正确的 Token 类型
                    return lookup_ident(&identifier);
                } else if is_digit(self.ch) {
                    // read_number 不需要返回，因为它内部已经消耗了所有数字字符
                    return self.read_number();
                } else {
                    Token::Illegal(self.ch as char)
                }
            }
        };

        // 为单字符 token 和处理过的多字符 token 前进指针
        self.read_char();
        
        token
    }

    fn read_char(&mut self) {
        if self.read_position >= self.input.len() {
            self.ch = 0; // 0 (NUL) 代表文件结束
        } else {
            self.ch = self.input[self.read_position];
        };
        self.position = self.read_position;
        self.read_position += 1;
    }

    // UPDATED: 此函数现在也能跳过单行注释
    fn skip_whitespace_and_comments(&mut self) {
        loop {
            if self.ch.is_ascii_whitespace() {
                self.read_char();
            } else if self.ch == b'/' && self.peek_char() == b'/' { // NEW: 检查 `//`
                // 如果是单行注释，一直读到行尾
                while self.ch != b'\n' && self.ch != 0 {
                    self.read_char();
                }
            } else {
                break;
            }
        }
    }

    fn read_identifier(&mut self) -> String {
        let start_pos = self.position;
        // 标识符可以包含字母、数字和下划线，但必须以字母或下划线开头
        // is_letter 已经包含了下划线
        while is_letter(self.ch) || is_digit(self.ch) {
            self.read_char();
        }
        // 使用 from_utf8_lossy 是安全的，因为 is_letter/is_digit 保证了是 ASCII
        String::from_utf8_lossy(&self.input[start_pos..self.position]).to_string()
    }

    fn read_string(&mut self) -> Token {
        let start_pos = self.position + 1;
        loop {
            self.read_char();
            if self.ch == b'"' || self.ch == 0 {
                break;
            }
        }
        if self.ch == 0 { // 未闭合的字符串
            return Token::Illegal('"');
        }
        let content = String::from_utf8_lossy(&self.input[start_pos..self.position]).to_string();
        Token::Literal(Literal::String(content))
    }

    fn read_number(&mut self) -> Token {
        let start_pos = self.position;
        while is_digit(self.ch) {
            self.read_char();
        }

        if self.ch == b'.' && is_digit(self.peek_char()) {
            self.read_char(); // 消耗 '.'
            while is_digit(self.ch) {
                self.read_char();
            }
            let full_str = std::str::from_utf8(&self.input[start_pos..self.position]).unwrap();
            let val = full_str.parse::<f64>().unwrap();
            return Token::Literal(Literal::Float(val));
        }

        let full_str = std::str::from_utf8(&self.input[start_pos..self.position]).unwrap();
        let val = full_str.parse::<i64>().unwrap();
        Token::Literal(Literal::Integer(val))
    }
}

// 辅助函数
fn is_letter(ch: u8) -> bool {
    ch.is_ascii_alphabetic() || ch == b'_'
}

fn is_digit(ch: u8) -> bool {
    ch >= b'0' && ch <= b'9'
}

// UPDATED: lookup_ident 现在包含了所有新的关键字，并移除了 `main` 和 `print`
fn lookup_ident(ident: &str) -> Token {
    let keyword = match ident {
        "ret" => Keyword::Ret,
        "if" => Keyword::If,
        "else" => Keyword::Else,
        "true" => Keyword::True,
        "false" => Keyword::False,
        "loop" => Keyword::Loop,
        "while" => Keyword::While,
        "break" => Keyword::Break,
        "continue" => Keyword::Continue,
        "class" => Keyword::Class,
        "enum" => Keyword::Enum,
        "match" => Keyword::Match,
        "new" => Keyword::New,
        "free" => Keyword::Free,
        "None" => Keyword::None,
        // 如果不是以上任何关键字，它就是一个普通的标识符
        _ => return Token::Identifier(ident.to_string()),
    };
    Token::Keyword(keyword)
}

// --- 测试模块 ---
#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::{Keyword, Literal};

    // UPDATED: 更新 `hello_world` 测试，因为 `main` 和 `print` 现在是标识符
    #[test]
    fn test_hello_world_as_identifiers() {
        let input = r#"
        main() { // a comment here
            print("Hello, Tipy World!")
        }
        "#;

        let expected_tokens = vec![
            Token::Identifier("main".to_string()), // Was: Keyword(Keyword::Main)
            Token::LParen,
            Token::RParen,
            Token::LBrace,
            Token::Identifier("print".to_string()), // Was: Keyword(Keyword::Print)
            Token::LParen,
            Token::Literal(Literal::String("Hello, Tipy World!".to_string())),
            Token::RParen,
            Token::RBrace,
            Token::Eof,
        ];

        let mut lexer = Lexer::new(input);
        for expected in expected_tokens {
            let token = lexer.next_token();
            assert_eq!(token, expected);
        }
    }
    
    // 保持原有的 calculator 测试不变，它依然有效
    #[test]
    fn test_calculator_tokens() {
        let input = "v: ~i32 = 5 + 10 * 2.5 / 1;";
        let expected_tokens = vec![
            Token::Identifier("v".to_string()),
            Token::Colon,
            Token::Tilde,
            Token::Identifier("i32".to_string()),
            Token::Equal,
            Token::Literal(Literal::Integer(5)),
            Token::Plus,
            Token::Literal(Literal::Integer(10)),
            Token::Star,
            Token::Literal(Literal::Float(2.5)),
            Token::Slash,
            Token::Literal(Literal::Integer(1)),
            Token::Semicolon,
            Token::Eof,
        ];
        let mut lexer = Lexer::new(input);
        for expected in expected_tokens {
            assert_eq!(lexer.next_token(), expected);
        }
    }

    // NEW: 添加一个专门的测试来验证函数定义和新关键字
    #[test]
    fn test_function_definition_tokens() {
        let input = r#"
        add(a: i32, b: i32) -> i32 {
            ret a + b
        }
        "#;

        let expected_tokens = vec![
            Token::Identifier("add".to_string()),
            Token::LParen,
            Token::Identifier("a".to_string()),
            Token::Colon,
            Token::Identifier("i32".to_string()),
            Token::Comma,
            Token::Identifier("b".to_string()),
            Token::Colon,
            Token::Identifier("i32".to_string()),
            Token::RParen,
            Token::Arrow, // 关键的新 Token
            Token::Identifier("i32".to_string()),
            Token::LBrace,
            Token::Keyword(Keyword::Ret), // 关键的新关键字
            Token::Identifier("a".to_string()),
            Token::Plus,
            Token::Identifier("b".to_string()),
            Token::RBrace,
            Token::Eof,
        ];

        let mut lexer = Lexer::new(input);
        for expected in expected_tokens {
            let token = lexer.next_token();
            // println!("Generated: {:?}", token); // 取消注释以进行调试
            assert_eq!(token, expected);
        }
    }
}