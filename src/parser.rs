// file: src/parser.rs

// --- 模块引入 ---
// 每个 `use` 块都解释了其引入的模块的职责。

// 引入诊断模块，用于创建和收集结构化的错误信息。
use crate::diagnostics::{CompilerError, ParserError, Span}; 

// 引入抽象语法树 (AST) 模块。
// 解析器的最终目标就是将 Token 流转换成这些结构化的 AST 节点。
use crate::ast::{
    // --- 顶层结构 ---
    Program,
    TopLevelStatement,

    // --- 语句 (Statements) ---
    Statement,
    BlockStatement,
    VarDeclaration,
    ReturnStatement,
    WhileStatement,
    BreakStatement,
    ContinueStatement,

    // --- 表达式 (Expressions) ---
    Expression,
    PrefixExpression,
    InfixExpression,
    AssignmentExpression,
    CallExpression,
    IfExpression,
    LoopExpression,
    
    // --- 运算符 ---
    Operator,
    PrefixOperator,

    // --- 函数相关 ---
    FunctionDeclaration,
    FunctionParameter,
};

// 引入词法分析器，它是 Parser 的 Token 来源。
use crate::lexer::Lexer;

// 引入 Token 定义，这是 Parser 直接消费的基本单元。
use crate::token::{Token, Keyword, Literal}; 

/// 定义了 Tipy 语言中运算符的优先级。
///
/// 这是 Pratt 解析器（一种自顶向下的算符优先解析器）的核心。
/// 通过比较当前和下一个 Token 的优先级，解析器能够正确地处理
/// 复杂的表达式，如 `a + b * c`，确保乘法先于加法计算。
///
/// 枚举成员的顺序从低到高排列。
#[derive(PartialEq, PartialOrd, Clone, Copy, Debug)]
pub enum Precedence {
    /// 最低优先级，作为所有表达式解析的起点。
    Lowest,
    /// 赋值表达式的优先级, e.g., `x = y`
    Assign,
    /// 比较表达式的优先级, e.g., `x == y`, `x > y`
    Comparison,
    /// 加减法表达式的优先级, e.g., `x + y`
    Sum,
    /// 乘除法表达式的优先级, e.g., `x * y`
    Product,
    /// 前缀表达式的优先级, e.g., `-x`, `!y`
    Prefix,
    /// 函数调用表达式的优先级, e.g., `my_func(x)`
    Call,
}

/// 解析器结构体，负责将 Token 流转换为 AST。
///
/// 它持有词法分析器 `Lexer` 来获取 Token，
/// 并通过向前“偷看”一个 Token (`peek_token`) 的策略来决定如何构建语法树。
/// 在整个解析过程中，所有遇到的错误都会被收集到 `errors` 向量中。
pub struct Parser<'a> {
    /// 词法分析器实例，为解析器提供源源不断的 Token。
    lexer: Lexer<'a>,
    
    /// 当前正在处理的 Token。解析逻辑的判断依据。
    current_token: Token,

    /// 下一个即将被处理的 Token。Pratt 解析器和许多其他解析策略
    /// 都需要它来决定当前的操作（例如，一个 `+` 后面是数字还是括号）。
    peek_token: Token,
    
    /// 错误收集器。
    ///
    /// 这是我们新的诊断系统的核心部分。解析器在遇到错误时，
    /// 不会立即停止，而是将一个结构化的 `CompilerError` 添加到此向量中，
    /// 然后尝试恢复并继续解析，以便一次性报告多个错误。
    pub errors: Vec<CompilerError>,
}

impl<'a> Parser<'a> {

    /// 创建一个新的 `Parser` 实例。
    ///
    /// 在构造过程中，它会立即从 `Lexer` 中预读取两个 Token，
    /// 以便填充 `current_token` 和 `peek_token`。
    /// 这是解析器能够“向前看”并做出决策的基础。
    ///
    /// # Arguments
    ///
    /// * `lexer` - 一个已经初始化好的 `Lexer` 实例。
    pub fn new(mut lexer: Lexer<'a>) -> Self {
        // 先创建一个包含 lexer 和空错误列表的 "半成品" Parser
        let mut p = Parser {
            lexer,
            // 暂时用 Eof 占位，将立即调用 next_token 来填充它们
            current_token: Token::Eof,
            peek_token: Token::Eof,
            errors: Vec::new(),
        };

        // 调用两次 next_token() 来正确初始化 current 和 peek。
        // next_token() 内部已经包含了处理词法错误并将其记入 p.errors 的逻辑
        p.next_token();
        p.next_token();

        p
    }

    /// 解析整个 Tipy 程序源代码，并返回程序的根节点 `Program` (一个 AST)。
    ///
    /// 这是解析器的主要入口点。它会持续解析顶层声明（目前仅支持函数），
    /// 直到遇到文件结束符 (Eof) 为止。
    ///
    /// 该函数采用**错误恢复**策略：
    /// - 当 `parse_top_level_statement` 成功时，它会将结果添加到程序体中。
    /// - 当遇到一个解析错误 (`Err`) 时，它不会立即停止，而是：
    ///   1. 将错误记录到 `self.errors` 向量中。
    ///   2. 调用 `self.synchronize()` 来尝试跳过出错的 Token，找到下一个
    ///      可能安全的同步点（如一个新的函数声明），然后继续解析。
    ///
    /// 这种机制允许我们一次性报告多个解析错误，极大地提升了用户体验，
    /// 并且从根本上解决了旧代码中的无限循环问题。
    pub fn parse_program(&mut self) -> Program {
        let mut program = Program::new();

        while !self.current_token_is(&Token::Eof) {
            match self.parse_top_level_statement() {
                Ok(stmt) => program.body.push(stmt),
                Err(err) => {
                    // NEW: 集成新的诊断系统
                    self.errors.push(CompilerError::Parser(err));
                    // NEW: 调用错误恢复机制，防止无限循环
                    self.synchronize();
                }
            }
        }
        program
    }

    // --- 内部辅助与错误处理 (Internal Helpers & Error Handling) ---

    /// 错误恢复函数，用于在解析失败后寻找下一个安全的同步点。
    ///
    /// 这是防止无限循环并能一次性报告多个错误的关键。当一个解析函数
    /// 返回 `Err` 后，主循环会调用此函数。
    ///
    /// 它的策略是：
    /// 1. 至少消耗掉引发错误的当前 Token。
    /// 2. 不断向前移动，直到找到一个被认为是新语句开始的标志
    ///    （如分号 `;`，或 `ret`, `if` 等关键字）。
    fn synchronize(&mut self) {
        self.next_token(); // 至少消耗掉引发错误的 token

        while !self.current_token_is(&Token::Eof) {
            // 如果上一个 token 是分号，那么我们很可能在一个新语句的开头，可以安全退出。
            if self.current_token_is(&Token::Semicolon) {
                self.next_token();
                return;
            }

            // 如果下一个 token 是一个常见的语句起始关键字，我们也可以认为找到了同步点。
            match self.peek_token {
                Token::Keyword(
                    Keyword::Class | 
                    Keyword::Ret   | 
                    Keyword::If    | 
                    Keyword::Loop  | 
                    Keyword::While
                ) => return,
                _ => {}
            }
            
            self.next_token();
        }
    }

    /// 检查 `peek_token` 是否为期望的类型，如果是，则向前推进一个 Token，并返回成功。
    /// 如果不是，则返回一个包含详细信息的 `ParserError`。
    ///
    /// 这是解析器中最常用的函数之一，它将 "检查并消耗" 这个动作合二为一，
    /// 并通过返回 `Result` 允许我们使用 `?` 操作符来极大地简化错误处理。
    ///
    /// # Returns
    /// - `Ok(())` 如果 `peek_token` 匹配 `expected`。
    /// - `Err(ParserError)` 如果不匹配。
    fn expect_peek(&mut self, expected: &Token) -> Result<(), ParserError> {
        if self.peek_token_is(expected) {
            self.next_token();
            Ok(())
        } else {
            Err(self.peek_error(format!("Expected next token to be {:?}", expected)))
        }
    }

    /// 检查当前 Token (`current_token`) 是否为指定的类型。
    fn current_token_is(&self, token_type: &Token) -> bool {
        // 使用 std::mem::discriminant 来比较 enum 的变体，而不关心其内部的值。
        // 这对于像 Token::Identifier(_) 这样的情况特别有用。
        std::mem::discriminant(&self.current_token) == std::mem::discriminant(token_type)
    }
    
    /// 检查下一个 Token (`peek_token`) 是否为指定的类型。
    fn peek_token_is(&self, token_type: &Token) -> bool {
        std::mem::discriminant(&self.peek_token) == std::mem::discriminant(token_type)
    }

    /// 获取下一个 Token (`peek_token`) 的优先级。
    fn peek_precedence(&self) -> Precedence {
        Self::token_to_precedence(&self.peek_token)
    }

    /// 获取当前 Token (`current_token`) 的优先级。
    fn current_precedence(&self) -> Precedence {
        Self::token_to_precedence(&self.current_token)
    }
    
    /// 将一个 Token 映射到其对应的运算符优先级。
    ///
    /// 注意：只有作为中缀运算符的 Token 才有高于 `Lowest` 的优先级。
    fn token_to_precedence(token: &Token) -> Precedence {
        match token {
            Token::Assign => Precedence::Assign,
            Token::Equal | Token::NotEqual | Token::LessThan | Token::GreaterThan |
            Token::LessEqual | Token::GreaterEqual => Precedence::Comparison,
            Token::Plus | Token::Minus => Precedence::Sum,
            Token::Star | Token::Slash => Precedence::Product,
            Token::LParen => Precedence::Call,
            _ => Precedence::Lowest,
        }
    }
    
    // --- 错误创建辅助函数 ---
    
    /// 根据当前 Token (`current_token`) 创建一个 `ParserError`。
    fn current_error(&self, message: String) -> ParserError {
        ParserError::UnexpectedToken {
            expected: message,
            found: self.current_token.clone(),
            // TODO: 当 Token 携带 Span 信息后，在这里传递真实的 Span。
            span: Span::default(), 
        }
    }
    
    /// 根据下一个 Token (`peek_token`) 创建一个 `ParserError`。
    fn peek_error(&self, message: String) -> ParserError {
        ParserError::UnexpectedToken {
            expected: message,
            found: self.peek_token.clone(),
            // TODO: 当 Token 携带 Span 信息后，在这里传递真实的 Span。
            span: Span::default(),
        }
    }

    // --- 顶层与声明解析 (Top-Level & Declaration Parsing) ---

    /// 解析一个顶层声明。
    ///
    /// 在 Tipy v0.0.5 中，唯一合法的顶层声明是函数声明。
    /// 未来这里可以扩展，以支持 `class`, `enum` 等。
    ///
    /// # Returns
    /// - `Ok(TopLevelStatement)` 如果成功解析一个函数声明。
    /// - `Err(ParserError)` 如果遇到的 Token 不是一个合法的顶层声明的开始。
    fn parse_top_level_statement(&mut self) -> Result<TopLevelStatement, ParserError> {
        // 一个简单的启发式规则：如果当前是标识符，且下一个是左括号，就认为是函数声明。
        if self.current_token_is(&Token::Identifier("".into())) && self.peek_token_is(&Token::LParen) {
            // `?` 操作符会自动处理 `parse_function_declaration` 可能返回的 Err
            let func_decl = self.parse_function_declaration()?;
            return Ok(TopLevelStatement::Function(func_decl));
        }
        
        // 如果不满足以上条件，则报告一个错误。
        Err(self.current_error("Expected a function declaration".to_string()))
    }

    /// 解析一个完整的函数声明。
    ///
    /// e.g., `my_func(a: i32, b: i32) -> i32 { ... }`
    fn parse_function_declaration(&mut self) -> Result<FunctionDeclaration, ParserError> {
        // 1. 解析函数名
        let name = self.parse_identifier_string()?;
        
        // 2. 解析参数列表
        self.expect_peek(&Token::LParen)?;
        let params = self.parse_function_parameters()?;
        // `parse_function_parameters` 结束时，`current_token` 应该是 ')'
        
        // 3. 解析可选的返回类型
        let return_type = if self.peek_token_is(&Token::Arrow) {
            self.next_token(); // 消耗 '->'
            self.next_token(); // 前进到类型标识符
            self.parse_identifier_string()?
        } else {
            // 如果没有 '->'，则为隐式 void 返回
            "void".to_string()
        };

        // 4. 解析函数体
        self.expect_peek(&Token::LBrace)?;
        let body = self.parse_block_statement()?; // 我们将在下一步实现此函数
        
        // `parse_block_statement` 结束时，`current_token` 应该是 '}'
        // 注意：我们在这里不消耗最后的 '}'，因为 Tipy 的块表达式特性
        // 意味着 `{...}` 本身可以是一个表达式，其调用者需要 '}' 作为结束标志。
        // 但对于函数声明，它的主体是一个语句块，通常需要消耗掉。这是一个需要仔细考虑的设计点。
        // 为保持一致性，我们暂定由 `parse_block_statement` 的调用者负责处理 `{` 和 `}`。
        
        Ok(FunctionDeclaration { name, params, return_type, body })
    }

    /// 解析函数声明中的参数列表 `(p1: T1, p2: T2, ...)`
    fn parse_function_parameters(&mut self) -> Result<Vec<FunctionParameter>, ParserError> {
        let mut params = Vec::new();

        // 处理空参数列表 `()` 的情况
        if self.peek_token_is(&Token::RParen) {
            self.next_token(); // 消耗 ')'
            return Ok(params);
        }

        self.next_token(); // 消耗 '('，前进到第一个参数名

        // 循环解析每个参数
        loop {
            let param_name = self.parse_identifier_string()?;
            self.expect_peek(&Token::Colon)?;
            self.next_token(); // 消耗 ':'，前进到类型名
            let param_type = self.parse_identifier_string()?;
            
            params.push(FunctionParameter { name: param_name, param_type });
            
            // 检查下一个 Token，决定是继续循环还是结束
            if !self.peek_token_is(&Token::Comma) {
                break; // 如果不是逗号，则参数列表应该结束了
            }
            
            self.next_token(); // 消耗 ','
            self.next_token(); // 前进到下一个参数名
        }

        // 循环结束后，必须紧跟一个右括号
        self.expect_peek(&Token::RParen)?;

        Ok(params)
    }

    // --- 语句解析 (Statement Parsing) ---

    /// 解析一个语句。
    ///
    /// 这是语句解析的“调度中心”。它根据当前的 Token 类型，
    /// 来决定应该调用哪个更具体的解析函数（如解析 `ret` 语句或变量声明）。
    fn parse_statement(&mut self) -> Result<Statement, ParserError> {
        match self.current_token {
            Token::Keyword(Keyword::Ret) => self.parse_return_statement(),
            Token::Keyword(Keyword::While) => self.parse_while_statement(),
            Token::Keyword(Keyword::Break) => self.parse_break_statement(),
            Token::Keyword(Keyword::Continue) => self.parse_continue_statement(),
            // `name: type` 形式的变量声明
            Token::Identifier(_) if self.peek_token_is(&Token::Colon) => {
                self.parse_variable_declaration_statement()
            }
            // 如果以上都不是，则它应该是一个表达式语句，例如一个函数调用 `my_func();`
            _ => self.parse_expression_statement(),
        }
    }

    /// 解析一个代码块 `{ ... }`。
    ///
    /// # 解析约定
    /// - **调用者**负责消耗起始的 `{`。
    /// - 此函数会持续解析内部的语句，直到遇到 `}` 或文件末尾 `Eof`。
    /// - 此函数**不会**消耗最后的 `}`，将其留给调用者处理。
    ///   这对于将代码块作为表达式（其值是最后一个表达式）的场景至关重要。
    ///
    /// # 错误恢复
    /// 这是解析器内第二个实现错误恢复循环的地方。如果块内某条语句解析失败，
    /// 它会记录错误，调用 `synchronize()` 跳到下一个安全点，然后继续解析块内的
    /// 其他语句，而不是让整个代码块的解析失败。
    fn parse_block_statement(&mut self) -> Result<BlockStatement, ParserError> {
        let mut statements = Vec::new();

        while !self.current_token_is(&Token::RBrace) && !self.current_token_is(&Token::Eof) {
            match self.parse_statement() {
                Ok(stmt) => statements.push(stmt),
                Err(err) => {
                    self.errors.push(CompilerError::Parser(err));
                    self.synchronize();
                }
            }
            // 在 Tipy 中分号是可选的，我们统一在循环末尾处理 Token 前进，
            // 无论语句后面有没有分号。
            self.next_token();
        }
    
        Ok(BlockStatement { statements })
    }
    
    /// 解析返回语句 `ret <expression>;`
    fn parse_return_statement(&mut self) -> Result<Statement, ParserError> {
        self.next_token(); // 消耗 `ret` 关键字

        let value = if self.current_token_is(&Token::Semicolon) || self.current_token_is(&Token::RBrace) {
            // 处理 `ret;` 或紧跟 `}` 的 `ret`
            None
        } else {
            // 解析 `ret <expression>`
            Some(self.parse_expression(Precedence::Lowest)?)
        };
        
        // 如果后面恰好有个分号，我们也消耗掉它，以保持整洁
        if self.peek_token_is(&Token::Semicolon) {
            self.next_token();
        }

        Ok(Statement::Return(ReturnStatement { value }))
    }

    /// 解析变量声明语句 `name: [~]type [= value];`
    fn parse_variable_declaration_statement(&mut self) -> Result<Statement, ParserError> {
        // `parse_statement` 已经确认了当前是 Identifier
        let name = self.parse_identifier_string()?;
        
        self.expect_peek(&Token::Colon)?; // 消耗 ':'
        self.next_token(); // 前进到类型或 '~'

        let is_mutable = if self.current_token_is(&Token::Tilde) {
            self.next_token(); // 消耗 '~'
            true
        } else {
            false
        };

        let var_type = self.parse_identifier_string()?;
        
        let value = if self.peek_token_is(&Token::Assign) {
            self.next_token(); // 消耗类型, 前进到 '='
            self.next_token(); // 消耗 '=', 前进到表达式的开头
            Some(self.parse_expression(Precedence::Lowest)?)
        } else {
            None // 没有初始值
        };

        // 同样，消耗可选的分号
        if self.peek_token_is(&Token::Semicolon) {
            self.next_token();
        }

        Ok(Statement::VarDeclaration(VarDeclaration { name, is_mutable, var_type, value }))
    }

    /// 解析一个表达式语句。
    ///
    /// 表达式语句就是一个表达式，其计算结果被丢弃。
    /// 例如，一个函数调用 `do_something(a, b);`
    fn parse_expression_statement(&mut self) -> Result<Statement, ParserError> {
        let expr = self.parse_expression(Precedence::Lowest)?;

        // Tipy 语法中分号是可选的，我们在这里检查并消耗它，
        // 这样表达式语句就可以正确地结束。
        if self.peek_token_is(&Token::Semicolon) {
            self.next_token();
        }

        Ok(Statement::Expression(expr))
    }

    /// 解析 `while` 循环语句 `while <condition> { ... }`
    fn parse_while_statement(&mut self) -> Result<Statement, ParserError> {
        self.next_token(); // 消耗 `while`
        
        let condition = self.parse_expression(Precedence::Lowest)?;
        
        self.expect_peek(&Token::LBrace)?;
        let body = self.parse_block_statement()?;
        // parse_block_statement 不消耗 '}'，所以我们在这里消耗
        self.expect_peek(&Token::RBrace)?;
        
        Ok(Statement::While(WhileStatement { condition, body }))
    }
    
    /// 解析 `break` 语句 `break [value];`
    fn parse_break_statement(&mut self) -> Result<Statement, ParserError> {
        self.next_token(); // 消耗 `break`

        let value = if self.current_token_is(&Token::Semicolon) || self.current_token_is(&Token::RBrace) {
            None
        } else {
            Some(self.parse_expression(Precedence::Lowest)?)
        };
        
        if self.peek_token_is(&Token::Semicolon) {
            self.next_token();
        }

        Ok(Statement::Break(BreakStatement { value }))
    }

    /// 解析 `continue` 语句 `continue;`
    fn parse_continue_statement(&mut self) -> Result<Statement, ParserError> {
        // `continue` 后面没有值，所以直接创建节点即可
        if self.peek_token_is(&Token::Semicolon) {
            self.next_token();
        }
        Ok(Statement::Continue(ContinueStatement))
    }

    // --- 表达式解析 (Expression Parsing) ---

    /// 解析一个表达式，这是 Pratt 解析器的核心入口。
    ///
    /// # Arguments
    /// * `precedence` - 当前的运算符优先级。调用者通过这个参数来控制
    ///   解析器应该“吃掉”多高优先级的运算符。
    fn parse_expression(&mut self, precedence: Precedence) -> Result<Expression, ParserError> {
        // --- 1. 前缀解析 (Prefix Parsing) ---
        // 每个表达式都必须由一个前缀部分开始，例如一个数字、一个变量名、一个 `!` 号，或一个 `if` 关键字。
        // 我们根据当前 Token 类型，调用对应的前缀解析函数。
        let mut left_expr = match self.current_token {
            Token::Identifier(_) => Ok(self.parse_identifier_expression()?),
            Token::Literal(_) => Ok(self.parse_literal_expression()?),
            Token::Keyword(Keyword::True) | Token::Keyword(Keyword::False) => Ok(self.parse_boolean_expression()?),
            Token::Bang | Token::Minus => self.parse_prefix_expression(),
            Token::LParen => self.parse_grouped_expression(),
            Token::Keyword(Keyword::If) => self.parse_if_expression(),
            Token::Keyword(Keyword::Loop) => self.parse_loop_expression(),
            Token::LBrace => self.parse_block_expression(),
            _ => Err(self.current_error(format!("Expected an expression, but found {:?}", self.current_token))),
        }?;

        // --- 2. 中缀解析 (Infix Parsing) ---
        // 在解析完前缀表达式后，我们进入一个循环，处理所有优先级比当前 `precedence` 更高的中缀运算符。
        while precedence < self.peek_precedence() {
            // 根据下一个 Token (`peek_token`) 的类型，决定调用哪个中缀解析函数。
            // 例如，如果下一个是 `+`，我们就解析一个加法表达式。
            // 如果下一个是 `(`, 我们就解析一个函数调用。
            match self.peek_token {
                Token::Plus | Token::Minus | Token::Star | Token::Slash | Token::Equal |
                Token::NotEqual | Token::LessThan | Token::LessEqual | Token::GreaterThan | Token::GreaterEqual => {
                    self.next_token();
                    left_expr = self.parse_infix_expression(left_expr)?;
                }
                Token::Assign => {
                    self.next_token();
                    left_expr = self.parse_assignment_expression(left_expr)?;
                }
                Token::LParen => {
                    self.next_token();
                    left_expr = self.parse_call_expression(left_expr)?;
                }
                _ => {
                    // 如果没有更多的中缀运算符，或者下一个运算符的优先级不够高，则循环结束。
                    return Ok(left_expr);
                }
            }
        }

        Ok(left_expr)
    }

    // --- 前缀表达式解析函数 ---

    fn parse_identifier_expression(&mut self) -> Result<Expression, ParserError> {
        self.parse_identifier_string().map(Expression::Identifier)
    }
    
    fn parse_literal_expression(&mut self) -> Result<Expression, ParserError> {
        // 我们已经确认 current_token 是 Literal，所以这里可以安全地 clone
        Ok(Expression::Literal(
            if let Token::Literal(lit) = &self.current_token {
                lit.clone()
            } else { unreachable!() }
        ))
    }

    fn parse_boolean_expression(&mut self) -> Result<Expression, ParserError> {
        let value = self.current_token_is(&Token::Keyword(Keyword::True));
        Ok(Expression::Literal(Literal::Boolean(value)))
    }

    fn parse_prefix_expression(&mut self) -> Result<Expression, ParserError> {
        let op = match self.current_token {
            Token::Minus => PrefixOperator::Minus,
            Token::Bang => PrefixOperator::Not,
            _ => unreachable!(), // 调用者已保证
        };
        self.next_token(); // 消耗前缀操作符
        let right = Box::new(self.parse_expression(Precedence::Prefix)?);
        Ok(Expression::Prefix(PrefixExpression { op, right }))
    }

    fn parse_grouped_expression(&mut self) -> Result<Expression, ParserError> {
        self.next_token(); // 消耗 '('
        let expr = self.parse_expression(Precedence::Lowest)?;
        self.expect_peek(&Token::RParen)?; // 期望并消耗 ')'
        Ok(expr)
    }

    fn parse_if_expression(&mut self) -> Result<Expression, ParserError> {
        self.next_token(); // 消耗 'if'
        let condition = Box::new(self.parse_expression(Precedence::Lowest)?);
        
        self.expect_peek(&Token::LBrace)?;
        let consequence = self.parse_block_statement()?;
        self.expect_peek(&Token::RBrace)?;
        
        let alternative = if self.peek_token_is(&Token::Keyword(Keyword::Else)) {
            self.next_token(); // 消耗 'else'
            // `else if` 链，本质上是解析另一个 if 表达式
            if self.peek_token_is(&Token::Keyword(Keyword::If)) {
                Some(Box::new(self.parse_if_expression()?))
            } 
            // `else { ... }` 分支
            else {
                self.expect_peek(&Token::LBrace)?;
                let alt_block = self.parse_block_expression()?;
                self.expect_peek(&Token::RBrace)?;
                Some(Box::new(alt_block))
            }
        } else {
            None // 没有 else 分支
        };

        Ok(Expression::If(IfExpression { condition, consequence, alternative }))
    }

    fn parse_loop_expression(&mut self) -> Result<Expression, ParserError> {
        self.expect_peek(&Token::LBrace)?;
        let body = self.parse_block_statement()?;
        self.expect_peek(&Token::RBrace)?;
        Ok(Expression::Loop(LoopExpression { body }))
    }
    
    fn parse_block_expression(&mut self) -> Result<Expression, ParserError> {
        let block_stmt = self.parse_block_statement()?;
        Ok(Expression::Block(block_stmt))
    }
    
    // --- 中缀表达式解析函数 ---
    
    fn parse_infix_expression(&mut self, left: Expression) -> Result<Expression, ParserError> {
        let op = match self.current_token {
            Token::Plus => Operator::Plus,
            Token::Minus => Operator::Minus,
            Token::Star => Operator::Multiply,
            Token::Slash => Operator::Divide,
            Token::Equal => Operator::Equal,
            Token::NotEqual => Operator::NotEqual,
            Token::LessThan => Operator::LessThan,
            Token::LessEqual => Operator::LessEqual,
            Token::GreaterThan => Operator::GreaterThan,
            Token::GreaterEqual => Operator::GreaterEqual,
            _ => unreachable!(),
        };
        
        let precedence = self.current_precedence();
        self.next_token(); // 消耗中缀操作符
        let right = Box::new(self.parse_expression(precedence)?);
        
        Ok(Expression::Infix(InfixExpression { op, left: Box::new(left), right }))
    }
    
    fn parse_assignment_expression(&mut self, left: Expression) -> Result<Expression, ParserError> {
        // 我们在 AST 层面已经将赋值目标的类型从 String 改为了 Expression，
        // 这里直接使用即可。至于 left 是否是合法的“左值”，由后续的语义分析阶段判断。
        let value = self.parse_expression(Precedence::Assign)?;
        Ok(Expression::Assignment(AssignmentExpression {
            left: Box::new(left),
            value: Box::new(value),
        }))
    }

    fn parse_call_expression(&mut self, function: Expression) -> Result<Expression, ParserError> {
        let arguments = self.parse_call_arguments()?;
        Ok(Expression::Call(CallExpression { function: Box::new(function), arguments }))
    }
    
    fn parse_call_arguments(&mut self) -> Result<Vec<Expression>, ParserError> {
        let mut args = Vec::new();

        if self.peek_token_is(&Token::RParen) {
            self.next_token(); // 消耗 ')'
            return Ok(args);
        }

        self.next_token(); // 消耗 '('

        args.push(self.parse_expression(Precedence::Lowest)?);

        while self.peek_token_is(&Token::Comma) {
            self.next_token(); // 消耗 ','
            self.next_token(); // 前进到下一个表达式的开头
            args.push(self.parse_expression(Precedence::Lowest)?);
        }

        self.expect_peek(&Token::RParen)?;
        Ok(args)
    }

    // --- 内部辅助函数 ---

    /// 将解析器向前推进一个 Token。
    ///
    /// 这个函数是解析器状态机的核心驱动。它将 `peek_token` 移到
    /// `current_token`，然后从 `lexer` 中请求下一个 Token 来填充 `peek_token`。
    ///
    /// # 错误处理
    /// `lexer.next_token()` 返回的是 `Result<Token, LexerError>`。
    /// 如果词法分析出错 (`Err`)，此函数会：
    /// 1. 将该 `LexerError` 包装成 `CompilerError` 并存入 `self.errors`。
    /// 2. 将 `peek_token` 设置为 `Eof`，以安全地终止后续的解析。
    fn next_token(&mut self) {
        self.current_token = self.peek_token.clone();

        // 从 Lexer 获取下一个 Token，并直接处理可能发生的词法错误
        match self.lexer.next_token() {
            Ok(token) => self.peek_token = token,
            Err(lex_err) => {
                // 如果 Lexer 出错，将错误记录下来
                self.errors.push(CompilerError::Lexer(lex_err));
                // 并将 peek 设置为 Eof，以防解析器继续处理一个无效的流
                self.peek_token = Token::Eof;
            }
        }
    }

    /// 解析一个标识符，并返回其 String 值。
    /// 这是个非常有用的工具函数，被 `parse_function_declaration`,
    /// `parse_variable_declaration` 等多个地方复用。
    fn parse_identifier_string(&mut self) -> Result<String, ParserError> {
        match &self.current_token {
            Token::Identifier(name) => Ok(name.clone()),
            _ => Err(self.current_error("Expected an identifier".to_string())),
        }
    }

    
}
