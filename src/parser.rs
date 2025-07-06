use crate::ast::{Program, Statement, Expression, Operator, PrefixOperator}; 
use crate::lexer::Lexer;
use crate::token::{Token, Literal}; 

// 运算符优先级
#[derive(PartialEq, PartialOrd, Clone, Copy)] 
pub enum Precedence {
    Lowest,
    Equals,      // = (赋值)
    Sum,         // + or -
    Product,     // * or /
    Prefix,      // -X or !X
    Call,        // myFunction(X)
}

pub struct Parser<'a> {
    lexer: Lexer<'a>,
    current_token: Token,
    peek_token: Token,
    pub errors: Vec<String>,
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
            errors: Vec::new(),
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

    // 获取优先级
    fn peek_precedence(&self) -> Precedence {
        Self::token_to_precedence(&self.peek_token)
    }

    fn current_precedence(&self) -> Precedence {
        Self::token_to_precedence(&self.current_token)
    }

    // 实现 token 到 precedence 的转换
    fn token_to_precedence(token: &Token) -> Precedence {
        match token {
            Token::Plus | Token::Minus => Precedence::Sum,
            Token::Star | Token::Slash => Precedence::Product,
            Token::Equal => Precedence::Equals,
            Token::LParen => Precedence::Call,
            // 其他 Token 暂时没有中缀优先级
            _ => Precedence::Lowest,
        }
    }

    // === 语句解析 (Statement Parsing) ===


    fn parse_statement(&mut self) -> Option<Statement> {
        match self.current_token {
            Token::Identifier(_) => {
                if self.peek_token == Token::Colon {
                    self.parse_variable_declaration_statement()
                } else {
                    self.parse_expression_statement()
                }
            }
            _ => self.parse_expression_statement(),
        }
    }

    fn parse_variable_declaration_statement(&mut self) -> Option<Statement> {
        let name = match &self.current_token {
            Token::Identifier(name) => name.clone(),
            _ => return None,
        };

        if !self.expect_peek(Token::Colon) { return None; }
        self.next_token(); // 消耗 ':'

        let is_mutable = if self.current_token == Token::Tilde {
            self.next_token(); // 消耗 '~'
            true
        } else { false };

        let var_type = match &self.current_token {
            Token::Identifier(type_name) => type_name.clone(),
            _ => { self.errors.push("Expected type name.".to_string()); return None; }
        };

        let value = if self.peek_token == Token::Equal {
            self.next_token(); // 消耗类型, 前进到 =
            self.next_token(); // 消耗 =, 前进到表达式开头
            self.parse_expression(Precedence::Lowest)?
        } else {
            self.errors.push("Variable declaration requires an initial value.".to_string());
            return None;
        };
        
        if self.peek_token == Token::Semicolon { self.next_token(); }

        Some(Statement::VarDeclaration { name, is_mutable, var_type, value: Some(value) })
    }

    fn parse_expression_statement(&mut self) -> Option<Statement> {
        let expr = self.parse_expression(Precedence::Lowest)?;

        if self.peek_token == Token::Semicolon {
            self.next_token();
        }
        Some(Statement::Expression(expr))
    }

    // === 表达式解析 (Expression Parsing) - Pratt Engine ===

    fn parse_expression(&mut self, precedence: Precedence) -> Option<Expression> {
        // 1. 获取当前 Token 对应的前缀处理函数
        let mut left_expr = match self.current_token {
            Token::Identifier(_) => self.parse_identifier(),
            Token::Literal(Literal::Integer(_)) => self.parse_integer_literal(),
            Token::Literal(Literal::Float(_)) => self.parse_float_literal(), // 新增浮点数
            Token::Minus => self.parse_prefix_expression(), // 处理 -5
            Token::LParen => self.parse_grouped_expression(), // 处理 (5 + 2)
            _ => {
                self.errors.push(format!("No prefix parse function for {:?}", self.current_token));
                None
            }
        }?;
    
        // 2. 循环处理中缀表达式
        while self.peek_token != Token::Semicolon && precedence < self.peek_precedence() {
            // 根据下一个 Token 的类型决定如何处理
            match self.peek_token {
                Token::Plus | Token::Minus | Token::Star | Token::Slash | Token::Equal => {
                    self.next_token();
                    left_expr = self.parse_infix_expression(left_expr)?;
                },
                Token::LParen => { // 函数调用
                    self.next_token();
                    left_expr = self.parse_call_expression(left_expr)?;
                },
                _ => return Some(left_expr),
            }
        }
        Some(left_expr)
    }

    // --- 前缀处理函数 ---

    fn parse_identifier(&mut self) -> Option<Expression> {
        if let Token::Identifier(name) = &self.current_token {
            Some(Expression::Variable(name.clone()))
        } else { None }
    }

    fn parse_integer_literal(&mut self) -> Option<Expression> {
        if let Token::Literal(Literal::Integer(value)) = self.current_token {
            Some(Expression::Literal(Literal::Integer(value)))
        } else { None }
    }

    fn parse_float_literal(&mut self) -> Option<Expression> {
        if let Token::Literal(Literal::Float(value)) = self.current_token {
            Some(Expression::Literal(Literal::Float(value)))
        } else { None }
    }

    // 新增：处理前缀表达式，如 -5
    fn parse_prefix_expression(&mut self) -> Option<Expression> {
        let operator = match self.current_token {
            Token::Minus => PrefixOperator::Minus,
            _ => return None, // Or handle other prefixes like `!`
        };
        self.next_token(); // 消耗掉 `-`
        // 递归调用 parse_expression，使用较高的 Prefix 优先级
        let right = self.parse_expression(Precedence::Prefix)?;
        Some(Expression::Prefix { op: operator, right: Box::new(right) })
    }

    // 新增：处理分组表达式，如 (5 + 2)
    fn parse_grouped_expression(&mut self) -> Option<Expression> {
        self.next_token(); // 消耗 '('
        let expr = self.parse_expression(Precedence::Lowest)?;
        if !self.expect_peek(Token::RParen) {
            return None;
        }
        Some(expr)
    }

    // --- 中缀处理函数 ---

    // 这是一个分派器，根据操作符类型调用具体的处理函数
    fn parse_infix_expression(&mut self, left: Expression) -> Option<Expression> {
        match self.current_token {
            Token::Plus | Token::Minus | Token::Star | Token::Slash => {
                self.parse_binary_expression(left)
            }
            Token::Equal => self.parse_assignment_expression(left),
            _ => {
                self.errors.push(format!("No infix parse function for {:?}", self.current_token));
                None
            }
        }
    }

    fn parse_binary_expression(&mut self, left: Expression) -> Option<Expression> {
        let operator = match self.current_token {
            Token::Plus => Operator::Plus,
            Token::Minus => Operator::Minus,
            Token::Star => Operator::Multiply,
            Token::Slash => Operator::Divide,
            _ => return None,
        };
        let precedence = self.current_precedence();
        self.next_token();
        let right = self.parse_expression(precedence)?;
        Some(Expression::Binary { op: operator, left: Box::new(left), right: Box::new(right) })
    }

    fn parse_assignment_expression(&mut self, left: Expression) -> Option<Expression> {
        let name = match left {
            Expression::Variable(name) => name,
            _ => { self.errors.push("Invalid assignment target".to_string()); return None; }
        };
        self.next_token(); // 消耗 `=`
        let value = self.parse_expression(Precedence::Lowest)?;
        Some(Expression::Assignment { name, value: Box::new(value) })
    }

    // 加回来！处理函数调用的参数列表
    fn parse_call_expression(&mut self, function: Expression) -> Option<Expression> {
        let mut arguments = Vec::new();
        if self.peek_token == Token::RParen {
            self.next_token(); // 消耗 ')'，处理空参数列表的情况
        } else {
            self.next_token(); // 消耗 '(', 前进到第一个参数
            arguments.push(self.parse_expression(Precedence::Lowest)?);
            while self.peek_token == Token::Comma {
                self.next_token();
                self.next_token();
                arguments.push(self.parse_expression(Precedence::Lowest)?);
            }
            if !self.expect_peek(Token::RParen) {
                return None;
            }
        }
        Some(Expression::Call { function: Box::new(function), arguments })
    }

    // 辅助函数，用于检查下一个 Token 并前进
    fn expect_peek(&mut self, expected: Token) -> bool {
        if self.peek_token == expected {
            self.next_token();
            true
        } else {
            self.errors.push(format!("Expected next token to be {:?}, got {:?} instead", expected, self.peek_token));
            false
        }
    }

}

#[test]
fn test_variable_declaration_statement() {
    let input = "x: i32 = 5;";

    let mut parser = Parser::new(Lexer::new(input));
    let program = parser.parse_program();

    // 检查解析过程中没有错误
    assert_eq!(parser.errors.len(), 0, "Parser has errors: {:?}", parser.errors);

    // 检查只生成了一条语句
    assert_eq!(program.statements.len(), 1);

    // 构建我们期望的 AST 节点
    let expected_statement = Statement::VarDeclaration {
        name: "x".to_string(),
        is_mutable: false,
        var_type: "i32".to_string(),
        value: Some(Expression::Literal(Literal::Integer(5))),
    };

    // 断言解析出的语句与期望的一致
    assert_eq!(program.statements[0], expected_statement);
}

#[test]
fn test_assignment_expression() {
    let input = "x = 10;";

    let mut parser = Parser::new(Lexer::new(input));
    let program = parser.parse_program();

    assert_eq!(parser.errors.len(), 0, "Parser has errors: {:?}", parser.errors);
    assert_eq!(program.statements.len(), 1);

    // 赋值是一个表达式语句
    let expected_statement = Statement::Expression(
        Expression::Assignment {
            name: "x".to_string(),
            value: Box::new(Expression::Literal(Literal::Integer(10))),
        }
    );

    assert_eq!(program.statements[0], expected_statement);
}

#[test]
fn test_operator_precedence() {
    let input = "5 + 2 * 10";

    let mut parser = Parser::new(Lexer::new(input));
    let program = parser.parse_program();

    assert_eq!(parser.errors.len(), 0, "Parser has errors: {:?}", parser.errors);
    assert_eq!(program.statements.len(), 1);

    // 我们期望的 AST 结构应该是 (5 + (2 * 10))
    let expected_statement = Statement::Expression(
        Expression::Binary {
            op: Operator::Plus,
            left: Box::new(Expression::Literal(Literal::Integer(5))),
            right: Box::new(Expression::Binary {
                op: Operator::Multiply,
                left: Box::new(Expression::Literal(Literal::Integer(2))),
                right: Box::new(Expression::Literal(Literal::Integer(10))),
            }),
        }
    );
    
    assert_eq!(program.statements[0], expected_statement);
}

// In src/parser.rs -> tests 模块

#[test]
fn test_complex_multiline_statements() {
    // 这段代码测试了多个变量声明，以及一个包含变量和括号的复杂表达式
    let input = "
        a: i32 = 10
        b: ~i32 = (5 + a) * 2
        b = a + b / 2
    ";

    let mut parser = Parser::new(Lexer::new(input));
    let program = parser.parse_program();

    // 确保没有解析错误
    assert_eq!(parser.errors.len(), 0, "Parser has errors: {:?}", parser.errors);

    // 期望有 3 条语句
    assert_eq!(program.statements.len(), 3);
    
    // 你可以根据需要，像之前的测试一样，精确地构建出期望的 AST 结构
    // 并用 assert_eq! 进行比较。这里我们暂时只检查语句数量，
    // 以确认解析器能正确地按行分割语句。
    // 如果你想进行精确断言，这是一个很好的练习！
}

// In src/parser.rs -> tests 模块

#[test]
fn test_single_line_with_semicolons() {
    let input = "x: i32 = 5; y = x + 1;"; // 注意末尾可选的分号

    let mut parser = Parser::new(Lexer::new(input));
    let program = parser.parse_program();

    assert_eq!(parser.errors.len(), 0, "Parser has errors: {:?}", parser.errors);
    assert_eq!(program.statements.len(), 2); // 期望解析出两条语句

    // 精确检查第一条语句
    let expected_stmt1 = Statement::VarDeclaration {
        name: "x".to_string(),
        is_mutable: false,
        var_type: "i32".to_string(),
        value: Some(Expression::Literal(Literal::Integer(5))),
    };
    assert_eq!(program.statements[0], expected_stmt1);

    // 精确检查第二条语句
    let expected_stmt2 = Statement::Expression(
        Expression::Assignment {
            name: "y".to_string(),
            value: Box::new(Expression::Binary {
                op: Operator::Plus,
                left: Box::new(Expression::Variable("x".to_string())),
                right: Box::new(Expression::Literal(Literal::Integer(1))),
            }),
        }
    );
    assert_eq!(program.statements[1], expected_stmt2);
}

#[test]
fn test_ultimate_expression() {
    let input = "result: i32 = -5 + my_func(2, 3 + 4) * 10;";
    
    // --- 步骤 1 & 2: 准备输入并执行解析 ---
    let mut parser = Parser::new(Lexer::new(input));
    let program = parser.parse_program();

    // 检查解析器在过程中是否记录了任何错误
    if !parser.errors.is_empty() {
        // 如果有错误，打印出来并让测试失败，这样调试起来更清晰
        panic!("Parser has errors: {:?}", parser.errors);
    }
    
    assert_eq!(program.statements.len(), 1, "Program should have 1 statement");

    // --- 步骤 3: 构建期望的 AST 并断言 ---

    // 这是我们期望从 "result: i32 = -5 + my_func(2, 3 + 4) * 10;" 中得到的 AST
    let expected_statement = Statement::VarDeclaration {
        name: "result".to_string(),
        is_mutable: false,
        var_type: "i32".to_string(),
        value: Some(Expression::Binary {
            op: Operator::Plus,
            // 左边是 (-5)
            left: Box::new(Expression::Prefix {
                op: crate::ast::PrefixOperator::Minus,
                right: Box::new(Expression::Literal(Literal::Integer(5))),
            }),
            // 右边是 (my_func(2, 3 + 4) * 10)
            right: Box::new(Expression::Binary {
                op: Operator::Multiply,
                // 乘法的左边是 my_func(...)
                left: Box::new(Expression::Call {
                    function: Box::new(Expression::Variable("my_func".to_string())),
                    arguments: vec![
                        Expression::Literal(Literal::Integer(2)),
                        // 函数的第二个参数是 (3 + 4)
                        Expression::Binary {
                            op: Operator::Plus,
                            left: Box::new(Expression::Literal(Literal::Integer(3))),
                            right: Box::new(Expression::Literal(Literal::Integer(4))),
                        },
                    ],
                }),
                // 乘法的右边是 10
                right: Box::new(Expression::Literal(Literal::Integer(10))),
            }),
        }),
    };
    
    // 断言解析器生成的 AST 和我们期望的完全一样
    assert_eq!(program.statements[0], expected_statement);
}