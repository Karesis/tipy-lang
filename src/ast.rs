// src/ast.rs

use crate::token::Literal;

// 整个程序的根节点
// 一个 Tipy 程序是由一系列顶层声明构成的集合。
// 目前，我们只支持函数声明。未来可以加入 class, enum 等。
#[derive(Debug, PartialEq, Clone)]
pub struct Program {
    pub body: Vec<TopLevelStatement>,
}

impl Program {
    pub fn new() -> Self {
        Program { body: Vec::new() }
    }
}

// 顶层声明
#[derive(Debug, PartialEq, Clone)]
pub enum TopLevelStatement {
    Function(FunctionDeclaration),
    // Future: Class(ClassDeclaration),
    // Future: Enum(EnumDeclaration),
}

// 语句 (Statement) - 构成代码块的基本单元，本身不返回值。
#[derive(Debug, PartialEq, Clone)]
pub enum Statement {
    /// 变量声明语句, e.g., `my_var: ~i32 = 10`
    VarDeclaration(VarDeclaration),
    /// 表达式语句，即一个表达式单独作为一行，其结果被丢弃, e.g., `add(1, 2);`
    Expression(Expression),
    /// 返回语句, e.g., `ret 10;`
    Return(ReturnStatement),
    /// 代码块, e.g., `{ ... }`
    Block(BlockStatement),
    /// while 循环语句, e.g., `while condition { ... }`
    While(WhileStatement),
    /// break 语句, e.g., `break;` or `break value;`
    Break(BreakStatement),
    /// continue 语句, e.g., `continue;`
    Continue(ContinueStatement),
}

// 表达式 (Expression) - 可以被求值的代码片段，总会产生一个值。
#[derive(Debug, PartialEq, Clone)]
pub enum Expression {
    /// 标识符, e.g., `my_var`
    Identifier(String),
    /// 字面量, e.g., `123`, `"hello"`
    Literal(Literal),
    /// 前缀表达式, e.g., `-10`
    Prefix(PrefixExpression),
    /// 二元运算表达式, e.g., `a + b`
    Infix(InfixExpression),
    /// 赋值表达式, e.g., `x = 5`
    Assignment(AssignmentExpression),
    /// 函数调用表达式, e.g., `add(1, 2)`
    Call(CallExpression),
    /// if-elif-else 表达式, e.g., `if condition { ... } else { ... }`
    If(IfExpression),
    /// loop 表达式, e.g., `loop { ... }`
    Loop(LoopExpression),
    /// 代码块本身也可以是一个表达式，其值为块中最后一条表达式的值
    Block(BlockStatement),
}

// --- 具体的 AST 节点定义 ---

/// 函数声明节点
/// e.g., `add(a: i32, b: i32) -> i32 { ... }`
#[derive(Debug, PartialEq, Clone)]
pub struct FunctionDeclaration {
    pub name: String, // 函数名，就是一个简单的标识符
    pub params: Vec<FunctionParameter>,
    // 返回类型，使用 String 存储类型名，语义分析时再解析
    // 如果没有返回箭头 `->`，则为 "void" 或类似的内部表示
    pub return_type: String, 
    pub body: BlockStatement, // 函数体总是一个代码块
}

/// 函数参数节点
/// e.g., `a: i32`
#[derive(Debug, PartialEq, Clone)]
pub struct FunctionParameter {
    pub name: String,
    // 参数类型，同样用 String 存储
    pub param_type: String,
}

/// 变量声明节点
#[derive(Debug, PartialEq, Clone)]
pub struct VarDeclaration {
    pub name: String,
    pub is_mutable: bool,
    pub var_type: String,
    pub value: Option<Expression>, // 初始值可选
}

/// 返回语句节点
#[derive(Debug, PartialEq, Clone)]
pub struct ReturnStatement {
    // `ret;` -> None, `ret value;` -> Some(value)
    pub value: Option<Expression>,
}

/// 代码块节点
#[derive(Debug, PartialEq, Clone)]
pub struct BlockStatement {
    pub statements: Vec<Statement>,
}

/// 前缀表达式节点
#[derive(Debug, PartialEq, Clone)]
pub struct PrefixExpression {
    pub op: PrefixOperator,
    pub right: Box<Expression>,
}

/// 二元(中缀)运算表达式节点
#[derive(Debug, PartialEq, Clone)]
pub struct InfixExpression {
    pub op: Operator,
    pub left: Box<Expression>,
    pub right: Box<Expression>,
}

/// 赋值表达式节点
#[derive(Debug, PartialEq, Clone)]
pub struct AssignmentExpression {
    pub left: Box<Expression>, 
    pub value: Box<Expression>,
}

/// 函数调用表达式节点
#[derive(Debug, PartialEq, Clone)]
pub struct CallExpression {
    // 被调用的函数可以是一个标识符 `foo()`，也可以是另一个表达式 `get_func()()`
    pub function: Box<Expression>, 
    pub arguments: Vec<Expression>,
}

/// If 表达式节点
/// e.g., `if condition { ... } else { ... }`
/// `elif` 会被解析为嵌套的 IfExpression，放在 alternative 字段中。
#[derive(Debug, PartialEq, Clone)]
pub struct IfExpression {
    pub condition: Box<Expression>,
    pub consequence: BlockStatement,
    // `else` 分支是可选的。如果存在，它也是一个表达式。
    // 这允许 `else if ...` 链式结构。
    pub alternative: Option<Box<Expression>>, 
}

/// loop 表达式节点
#[derive(Debug, PartialEq, Clone)]
pub struct LoopExpression {
    pub body: BlockStatement,
}

/// while 语句节点
#[derive(Debug, PartialEq, Clone)]
pub struct WhileStatement {
    pub condition: Expression,
    pub body: BlockStatement,
}

/// break 语句节点
#[derive(Debug, PartialEq, Clone)]
pub struct BreakStatement {
    // `break;` -> None, `break value;` -> Some(value)
    pub value: Option<Expression>,
}

/// continue 语句节点 (它没有额外数据)
#[derive(Debug, PartialEq, Clone)]
pub struct ContinueStatement;

// --- 操作符枚举 ---

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Operator {
    // 算术
    Plus,
    Minus,
    Multiply,
    Divide,
    // 比较
    Equal,        // ==
    NotEqual,     // !=
    LessThan,     // <
    LessEqual,    // <=
    GreaterThan,  // >
    GreaterEqual, // >=
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum PrefixOperator {
    Minus, // -
    Not,   // !
}