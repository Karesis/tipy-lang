// file: src/lexer.rs

use crate::token::{Token, Keyword, Literal};
use crate::diagnostics::{LexerError, Span}; 

/// 词法分析器
pub struct Lexer<'a> {
    // 源代码字符串
    source: &'a str, 
    // 跟踪字节位置用于切片
    position: usize,
    // 跟踪行列号用于 Span
    line: u32,
    column: u32,
    // 使用 char 来支持 Unicode
    ch: char, 
}

/// 词法分析器的具体实现
impl<'a> Lexer<'a> {

    // 创建一个新的词法分析器
    pub fn new(source: &'a str) -> Self {
        let mut lexer = Lexer {
            source,
            position: 0,
            line: 1,
            column: 0, // 将在 read_char 中首次变为 1
            ch: '\0',
        };
        lexer.read_char(); // 初始化第一个字符
        lexer
    }

    // 核心接口，会返回 Result，需要后续解包
    pub fn next_token(&mut self) -> Result<Token, LexerError> {
        // 跳过空白和注释
        self.skip_whitespace_and_comments();
        
        // 在处理 token 前记录起始位置，方便报错
        let start_pos = self.position; 
        let start_line = self.line;
        let start_col = self.column;
        
        // 主解析与匹配逻辑
        let token_result = match self.ch {
            
            // 双字符
            '=' => {
                if self.peek_char() == '=' {
                    self.read_char();
                    Ok(Token::Equal)
                } else {
                    Ok(Token::Assign)
                }
            }
            '!' => {
                if self.peek_char() == '=' {
                    self.read_char();
                    Ok(Token::NotEqual)
                } else {
                    Ok(Token::Bang)
                }
            }
            '<' => {
                if self.peek_char() == '=' { 
                    self.read_char(); 
                    Ok(Token::LessEqual) 
                } else { 
                    Ok(Token::LessThan) 
                }
            }
            '>' => {
                if self.peek_char() == '=' { 
                    self.read_char(); 
                    Ok(Token::GreaterEqual) 
                } else { 
                    Ok(Token::GreaterThan) 
                }
            }
            '-' => {
                if self.peek_char() == '>' {
                    self.read_char();
                    Ok(Token::Arrow)
                } else {
                    Ok(Token::Minus)
                }
            }
            
            // 单字符
            '+' => Ok(Token::Plus),
            '*' => Ok(Token::Star),
            '/' => Ok(Token::Slash), // 注释已在 skip 中处理
            '~' => Ok(Token::Tilde),
            ':' => Ok(Token::Colon),
            ';' => Ok(Token::Semicolon),
            ',' => Ok(Token::Comma),
            '(' => Ok(Token::LParen),
            ')' => Ok(Token::RParen),
            '{' => Ok(Token::LBrace),
            '}' => Ok(Token::RBrace),
            '^' => Ok(Token::Caret),
            '|' => Ok(Token::Pipe),

            // 处理字符串字面量("hello")
            '"' => self.read_string(), 
            // 处理字符字面量('a')
            '\'' => self.read_char_literal(), 

            // 文件末尾
            '\0' => Ok(Token::Eof),

            // 其他非符号token
            _ => {
                // 处理标识符
                if self.ch.is_ascii_alphabetic() || self.ch == '_' {
                    // 先由read_identifier()处理成String
                    let ident = self.read_identifier();

                    // 然后再由lookup_indent查看是否为关键字
                    return Ok(lookup_ident(&ident)); // 直接返回，因为它已消耗所有字符
                
                // 处理数字字面量
                } else if self.ch.is_ascii_digit() {
                    return self.read_number(); // read_number 返回 Result<Token, LexerError>

                // 处理未知错误
                } else {
                    // 处理未知字符，返回结构化错误
                    let span = Span { line: start_line, column: start_col, start_byte: start_pos, end_byte: self.position };
                    Err(LexerError::UnknownCharacter { char: self.ch, span })
                }
            }
        };
        
        // 对于所有通过 Ok() 分支的 token，向前移动一个字符
        // 注意：返回 Ok 或 Err 的分支需要自行处理 read_char
        if token_result.is_ok() {
            self.read_char();
        }

        // 返回最终得到的token_result
        token_result
    }

    // --- 辅助函数 ---

    fn read_char(&mut self) {
        let current_len = self.ch.len_utf8();
        self.position += current_len;
        
        if self.position >= self.source.len() {
            self.ch = '\0';
            return;
        }

        self.ch = self.source[self.position..].chars().next().unwrap_or('\0');

        if self.ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
    }

    fn peek_char(&self) -> char {
        let current_len = self.ch.len_utf8();
        if self.position + current_len >= self.source.len() {
            '\0'
        } else {
            self.source[self.position + current_len..].chars().next().unwrap_or('\0')
        }
    }

    // 跳过所有的空白和单行注释
    fn skip_whitespace_and_comments(&mut self) {
        // 主循环开始
        loop {
            // 如果是空白就一直跳过
            if self.ch.is_whitespace() {
                self.read_char();

            // 如果检测到连续的两个'/'，说明是单行注释
            } else if self.ch == '/' && self.peek_char() == '/' {
                // 只要没有遇到换行和文件末尾，一直跳过
                while self.ch != '\n' && self.ch != '\0' {
                    self.read_char();
                }
            
            // 这里说明上面俩种情况都不是，逻辑走完了，loop结束
            } else {
                break;
            }
        }
    }
    
    // 在处理标识符中使用，读取一个标识符并转换成String
    fn read_identifier(&mut self) -> String {
        let start_pos = self.position;
        while self.ch.is_ascii_alphanumeric() || self.ch == '_' {
            self.read_char();
        }
        self.source[start_pos..self.position].to_string()
    }

    // 处理字符串字面量（"hello world")
    fn read_string(&mut self) -> Result<Token, LexerError> {
        // 记录起始位置，方便传出错误
        let start_pos = self.position;
        let start_line = self.line;
        let start_col = self.column;
        
        self.read_char(); // 消耗起始的 "
        let content_start = self.position;
        
        while self.ch != '"' && self.ch != '\0' {
            self.read_char();
        }
        
        // 直接到结尾说明字符串未关闭
        if self.ch == '\0' {
            let span = Span { 
                line: start_line, 
                column: start_col, 
                start_byte: start_pos, 
                end_byte: self.position 
            };
            return Err(LexerError::UnterminatedString { start_span: span });
        }
        
        // 截取字符串并转化为String
        let content = self.source[content_start..self.position].to_string();

        // 能到这里就可以直接返回字面量了
        Ok(Token::Literal(Literal::String(content)))
    }
    
    // 读取字符字面量 e.g. 'a'
    fn read_char_literal(&mut self) -> Result<Token, LexerError> {
        let start_pos = self.position;
        let start_line = self.line;
        let start_col = self.column;

        self.read_char(); // 消耗起始的 '
        let char_val = self.ch;
        self.read_char(); // 消耗字符本身

        // 如果不是以'结尾，则说明出错了，需要记录
        if self.ch != '\'' {
            let span = Span { 
                line: start_line, 
                column: start_col, 
                start_byte: start_pos, 
                end_byte: self.position 
            };
            return Err(LexerError::MalformedCharLiteral { span }); 
        }
        
        // 返回正确识别的字符
        Ok(Token::Literal(Literal::Char(char_val)))
    }

    // 处理数字字面量，包含整数和浮点数
    fn read_number(&mut self) -> Result<Token, LexerError> {
        let start_pos = self.position;
        let start_line = self.line;
        let start_col = self.column;

        while self.ch.is_ascii_digit() {
            self.read_char();
        }

        // 处理浮点数
        if self.ch == '.' && self.peek_char().is_ascii_digit() {
            self.read_char(); // 消耗 '.'
            while self.ch.is_ascii_digit() {
                self.read_char();
            }
            let num_str = &self.source[start_pos..self.position];
            return match num_str.parse::<f64>() {
                Ok(val) => Ok(Token::Literal(Literal::Float(val))),
                Err(_) => {
                    let span = Span { 
                        line: start_line, 
                        column: start_col, 
                        start_byte: start_pos, 
                        end_byte: self.position 
                    };
                    Err(
                        LexerError::MalformedNumberLiteral { 
                            reason: "Invalid float".to_string(), 
                            span 
                        }
                    )
                }
            };
        }
        
        // 处理整数
        let num_str = &self.source[start_pos..self.position];
        match num_str.parse::<i64>() {
            Ok(val) => Ok(Token::Literal(Literal::Integer(val))),
            Err(_) => {
                 let span = Span { 
                    line: start_line, 
                    column: start_col, 
                    start_byte: start_pos, 
                    end_byte: self.position 
                };
                Err(
                    LexerError::MalformedNumberLiteral { 
                        reason: "Invalid integer".to_string(), 
                        span 
                    }
                )
            }
        }
    }
}

// --- 辅助函数 ---

// 检查一个字符是否符合标识符命名标准
fn is_letter(ch: u8) -> bool {
    ch.is_ascii_alphabetic() || ch == b'_'
}

// 检查一个字符是否是数字
fn is_digit(ch: u8) -> bool {
    ch >= b'0' && ch <= b'9'
}

// 处理一个标识符
// 如果是一个关键字，则返回关键字；
// 如果是普通标识符，就返回普通标识符
fn lookup_ident(ident: &str) -> Token {
    let keyword = match ident {

        // 匹配所有关键字
        "ret" => Keyword::Ret,
        "if" => Keyword::If,
        "else" => Keyword::Else,
        "elif" => Keyword::Elif,
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

        // 如果不是以上任何关键字，它就是一个普通的标识符，提前返回
        _ => return Token::Identifier(ident.to_string()),
    };

    // 返回关键字
    Token::Keyword(keyword)
}

