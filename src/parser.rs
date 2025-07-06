use crate::ast::{Program, Statement, Expression}; 
use crate::lexer::Lexer;
use crate::token::{Keyword, Token}; 

pub struct Parser<'a> {
    lexer: Lexer<'a>,
    current_token: Token,
    peek_token: Token,
}

impl<'a> Parser<'a> {
    pub fn new(mut lexer: Lexer<'a>) -> Self {
        // 初始化时，我们连读两次，来同时填充 current_token 和 peek_token
        let current_token = lexer.next_token();
        let peek_token = lexer.next_token();

        Parser {
            lexer,
            current_token,
            peek_token,
        }
    }

    fn next_token(&mut self) {
        self.current_token = self.peek_token.clone(); 
        self.peek_token = self.lexer.next_token();    
    }

    pub fn parse_program(&mut self) -> Program {
        let mut program = Program::new();

        while self.current_token != Token::Eof {
            let stmt = self.parse_statement();
            if let Some(s) = stmt {
                program.statements.push(s);
            }
            // 前进到下一个 token，准备解析下一条语句
            self.next_token();
        }

        program
    }

    // === 解析语句的核心分派函数 ===

    // 根据当前的 token，决定调用哪个具体的语句解析函数
    fn parse_statement(&mut self) -> Option<Statement> {
        match self.current_token {
            Token::Keyword(Keyword::Main) => self.parse_function_statement(),
            _ => self.parse_expression_statement(),
        }
    }

    fn expect_peek(&mut self, expected: &Token) -> bool {
        if &self.peek_token == expected {
            self.next_token();
            true
        } else {
            println!("Expected next token to be {:?}, but got {:?} instead", expected, &self.peek_token);
            false
        }
    }

    fn parse_function_statement(&mut self) -> Option<Statement> {
        let name = Expression::Identifier("main".to_string());
    
        if !self.expect_peek(&Token::LParen) { return None; }
        if !self.expect_peek(&Token::RParen) { return None; }
        if !self.expect_peek(&Token::LBrace) { return None; }

        let body = self.parse_block_statement();
    
        Some(Statement::Function { name, body })
    }
    
    /// 解析花括号中的语句块
    fn parse_block_statement(&mut self) -> Vec<Statement> {
        let mut statements = Vec::new();
        self.next_token(); // eat `{`
    
        // 循环解析语句，直到遇到 `}` 或者文件末尾
        while self.current_token != Token::RBrace && self.current_token != Token::Eof {
            if let Some(stmt) = self.parse_statement() {
                statements.push(stmt);
            }
            self.next_token();
        }
        statements
    }

    /// 解析一个表达式语句，它仅仅是单个表达式构成的语句
    fn parse_expression_statement(&mut self) -> Option<Statement> {
        // 调用主表达式解析函数
        let expression = self.parse_expression()?; // `?` 如果是 None 就提前返回
        // 将结果包装成 Expression 语句
        Some(Statement::Expression(expression))
    }

    /// 主表达式解析函数（目前非常简化）
    fn parse_expression(&mut self) -> Option<Expression> {
        // 首先，解析一个“前缀”表达式，比如一个标识符或一个字面量
        let mut left_expr = match &self.current_token {
            Token::Keyword(Keyword::Print) => Some(Expression::Identifier("print".to_string())),
            Token::Literal(lit) => Some(Expression::Literal(lit.clone())),
            _ => None,
        }?;

        while self.peek_token == Token::LParen {
            self.next_token(); // 前进到 `(`
            // 如果是 `(`，说明这是一个函数调用，调用专门的函数来解析
            left_expr = self.parse_call_expression(left_expr)?;
        }

        Some(left_expr)
    }

    /// 解析函数调用表达式，比如 `(arg1, arg2)`
    fn parse_call_expression(&mut self, function: Expression) -> Option<Expression> {
        let arguments = self.parse_expression_list(&Token::RParen)?;
        Some(Expression::Call {
            function: Box::new(function),
            arguments,
        })
    }

    /// 解析一个由逗号分隔的表达式列表，直到遇到 `end_token`
    fn parse_expression_list(&mut self, end_token: &Token) -> Option<Vec<Expression>> {
        let mut list = Vec::new();

        // 处理空参数列表的情况，比如 `()`
        if &self.peek_token == end_token {
            self.next_token();
            return Some(list);
        }
        
        self.next_token(); // 跳过 `(`
        list.push(self.parse_expression()?); // 解析第一个参数

        // 循环解析其他由逗号分隔的参数
        while self.peek_token == Token::Comma { // 暂时用 Illegal 代表逗号
            self.next_token();
            self.next_token();
            list.push(self.parse_expression()?);
        }

        // 最后必须是结束符 `)`
        if !self.expect_peek(end_token) {
            return None;
        }

        Some(list)
    }
}