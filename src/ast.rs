use crate::token::Literal;

// 二元运算符
#[derive(Debug, PartialEq, Clone)]
pub enum Operator {
    Plus,
    Minus,
    Multiply,
    Divide,
}

// 前缀操作符
#[derive(Debug, PartialEq, Clone)]
pub enum PrefixOperator {
    Minus,
    // Not, // for future `!` operator
}

#[derive(Debug, PartialEq)]
pub enum Expression {
    Variable(String),

    // 赋值表达式
    Assignment {
        name: String,  // 被赋值的变量名
        value: Box<Expression>, // 赋给它的值
    },

    // 所有的二元运算
    Binary {
        op: Operator,
        left: Box<Expression>,
        right: Box<Expression>,
    },

    Identifier(String),
    Literal(Literal),
    Call {
        function: Box<Expression>,
        arguments: Vec<Expression>,
    },

    Prefix {
        op: PrefixOperator,
        right: Box<Expression>,
    }
}

#[derive(Debug, PartialEq)]
pub enum Statement {
    VarDeclaration {
        name: String,
        // 我们从 Token::Tilde 的存在与否，就能判断出 is_mutable
        is_mutable: bool,
        // var_type 暂时用 String 存储类型名，如 "i32"
        // 后续语义分析阶段会将其解析成真正的类型
        var_type: String, 
        // 初始值是可选的，支持 `x: i32;` 这样的声明
        value: Option<Expression>,
    },

    Function {
        name: Expression,
        body: Vec<Statement>,
    },
    Expression(Expression),
}

#[derive(Debug, PartialEq)]
pub struct Program {
    pub statements: Vec<Statement>,
}

impl Program {
    pub fn new() -> Self {
        Program { 
            statements: Vec::new(), 
        }  
    }
}