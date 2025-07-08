// file: src/diagnostics.rs

use crate::token::Token;
use crate::types::Type;

use std::fmt; // 引入格式化 trait
use inkwell::builder::BuilderError;

// --- 统一的编译器错误类型 ---
#[derive(Debug)]
pub enum CompilerError {
    /// 词法分析器错误
    Lexer(LexerError),
    /// 语法分析器错误
    Parser(ParserError), 
    /// 语意分析错误
    Semantic(SemanticError),
    /// 代码生成错误
    Codegen(CodegenError),
}
// 为 CompilerError 实现 Display,从而方便打印
impl fmt::Display for CompilerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompilerError::Lexer(e) => e.fmt(f),
            CompilerError::Parser(e) => e.fmt(f),
            CompilerError::Semantic(e) => e.fmt(f),
            CompilerError::Codegen(e) => e.fmt(f),
        }
    }
}

// --- 词法分析阶段的错误 ---
// UPDATED: 完善了所有 Lexer 可能产生的错误类型
#[derive(Debug, Clone, PartialEq)]
pub enum LexerError {
    /// 词法分析器遇到了一个无法识别的、不属于任何合法 Token 起始部分的字符。
    /// 例如，在 Tipy 语言中遇到 `@` 或 `$` 符号。
    UnknownCharacter { char: char, span: Span },
    
    /// 字符串字面量没有找到闭合的双引号 `"`，一直到了文件末尾。
    UnterminatedString { start_span: Span },

    /// 数字字面量的格式不正确。
    /// 例如，"1.2.3" (多个小数点) 或者一个无法解析为 i64/f64 的巨大数字。
    MalformedNumberLiteral { reason: String, span: Span },
    
    /// 字符字面量的格式不正确。
    /// 例如，`'ab'` (包含多个字符) 或者 `'a` (没有找到闭合的单引号)。
    MalformedCharLiteral { span: Span },

    // --- 为未来准备 ---
    // /// 块注释 /* ... */ 没有找到闭合的 */
    // UnterminatedBlockComment { start_span: Span },
}
/// 为LexerError实现方便的打印trait
impl fmt::Display for LexerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LexerError::UnknownCharacter { char, span } => {
                write!(f, "Lexical Error: Unknown character '{}' at line {}, column {}.", char, span.line, span.column)
            }
            LexerError::UnterminatedString { start_span } => {
                write!(f, "Lexical Error: Unterminated string starting at line {}, column {}.", start_span.line, start_span.column)
            }
            LexerError::MalformedNumberLiteral { reason, span } => {
                write!(f, "Lexical Error: Malformed number literal '{}' at line {}, column {}.", reason, span.line, span.column)
            }
            LexerError::MalformedCharLiteral { span } => {
                write!(f, "Lexical Error: Malformed character literal at line {}, column {}.", span.line, span.column)
            }
            // ... 未来可以添加更多 ...
        }
    }
}

// --- 解析阶段的错误 ---
#[derive(Debug, Clone, PartialEq)]
pub enum ParserError {
    /// 这是最常见的解析错误。
    /// "我期望在这里看到一个分号，但却找到了一个 `if` 关键字"
    UnexpectedToken {
        expected: String, // 描述期望的是什么，例如 "an expression", "a semicolon ';'"
        found: Token,     // 实际找到的不匹配的 Token
        span: Span,
    },
    
    /// 当表达式不完整时使用。
    /// "在解析 `a +` 时，发现代码意外结束了"
    UnexpectedEof {
        expected: String,
    },

    /// 用于赋值表达式，当 `=` 左边不是一个合法的赋值目标时。
    InvalidAssignmentTarget {
        span: Span,
    },

    // 以后可以添加更多，例如：
    // TooManyParameters { span: Span },
    // DuplicateParameterName { name: String, span: Span },
}
/// 为ParserError实现方便的打印trait
impl fmt::Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParserError::UnexpectedToken { expected, found, span } => {
                write!(f, "Syntax Error: Expected {}, but found {:?} at line {}, column {}.", expected, found, span.line, span.column)
            }
            ParserError::UnexpectedEof { expected } => {
                write!(f, "Syntax Error: Unexpected end of file. Expected {}.", expected)
            }
            ParserError::InvalidAssignmentTarget { span } => {
                write!(f, "Syntax Error: Invalid assignment target at line {}, column {}. You can only assign to variables.", span.line, span.column)
            }
            // ... 未来可以添加更多 ...
        }
    }
}

// --- NEW: 语义分析阶段的错误 ---
#[derive(Debug, Clone, PartialEq)]
pub enum SemanticError {
    /// 符号（变量、函数等）在当前作用域已被定义。
    SymbolAlreadyDefined { name: String, span: Span },

    /// 尝试使用一个未定义的符号。
    SymbolNotFound { name: String, span: Span },

    /// 类型不匹配错误。
    /// e.g., `x: i32 = true;` (期望 i32, 得到 bool)
    TypeMismatch {
        expected: Type,
        found: Type,
        span: Span,
    },

    /// 条件表达式不为布尔类型错误。
    /// e.g., `if 10 { ... }` (if 的条件必须是 bool)
    ConditionNotBoolean {
        found: Type,
        span: Span,
    },
    
    /// 在循环外使用了 `break` 语句。
    IllegalBreak { span: Span },
    
    /// 在循环外使用了 `continue` 语句。
    IllegalContinue { span: Span },

    /// 对一个非函数类型的值进行函数调用。
    /// e.g., `x: i32 = 10; x();`
    NotAFunction {
        found: Type,
        span: Span,
    },
    
    /// 函数调用时的参数数量不匹配。
    ArityMismatch {
        expected: usize,
        found: usize,
        span: Span,
    },
    
    /// 无效的赋值目标。
    /// e.g., `5 = x;` 或 `(a+b) = c;`
    InvalidAssignmentTarget { span: Span },

    /// 运算符无法应用于给定的类型。
    /// e.g., `!10` (逻辑非不能用于整数) or `-true` (负号不能用于布尔值)
    InvalidOperatorForType {
        operator: String,
        the_type: Type, // a more neutral name than 'found'
        span: Span,
    },
}
/// 为SemanticError实现方便的打印trait
impl fmt::Display for SemanticError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SemanticError::SymbolAlreadyDefined { name, span } => {
                write!(f, "Semantic Error: Symbol '{}' is already defined in this scope. (at line {})", name, span.line)
            }
            SemanticError::SymbolNotFound { name, span } => {
                write!(f, "Semantic Error: Use of undefined symbol '{}' at line {}.", name, span.line)
            }
            SemanticError::TypeMismatch { expected, found, span } => {
                write!(f, "Semantic Error: Type mismatch at line {}. Expected type '{}', but found '{}'.", span.line, expected, found)
            }
            SemanticError::ConditionNotBoolean { found, span } => {
                write!(f, "Semantic Error: Condition expression must be a boolean, but got '{}' at line {}.", found, span.line)
            }
            SemanticError::IllegalBreak { span } => {
                write!(f, "Semantic Error: 'break' can only be used inside a loop (at line {}).", span.line)
            }
            SemanticError::IllegalContinue { span } => {
                write!(f, "Semantic Error: 'continue' can only be used inside a loop (at line {}).", span.line)
            }
            SemanticError::NotAFunction { found, span } => {
                write!(f, "Semantic Error: Cannot call a non-function type '{}' at line {}.", found, span.line)
            }
            SemanticError::ArityMismatch { expected, found, span } => {
                write!(f, "Semantic Error: Function call at line {} expected {} arguments, but got {}.", span.line, expected, found)
            }
            SemanticError::InvalidAssignmentTarget { span } => {
                write!(f, "Semantic Error: Invalid assignment target at line {}.", span.line)
            }
            SemanticError::InvalidOperatorForType { operator, the_type, span } => {
                write!(f, "Semantic Error: Operator '{}' cannot be applied to type '{}' at line {}.", operator, the_type, span.line)
            }
        }
    }
}

// --- 代码生成阶段的错误 ---
#[derive(Debug)] // inkwell 的错误类型不支持 Clone 和 PartialEq，所以我们这里也去掉
pub enum CodegenError {
    /// 包装了来自 LLVM 后端 (inkwell) 的底层错误。
    Backend(BuilderError),

    /// 语义分析阶段本应捕获但遗漏的问题，作为最后的防线。
    /// 例如，尝试为一个在符号表中找不到的变量生成代码。
    SymbolNotFound(String),

    /// 尝试为一个非左值（L-Value）的表达式生成赋值操作。
    /// 例如，`5 = x;`
    InvalidLValue,

    /// 用于包装一个简单的、基于字符串的错误信息。
    /// 在某些不值得为其创建一个专属错误类型的场景下非常有用。
    Message(String),

    // 未来可以添加更多，例如：
    // UnsupportedExpression(String),
}
// 为了能友好地打印错误
impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CodegenError::Backend(e) => write!(f, "LLVM Backend Error: {}", e),
            CodegenError::SymbolNotFound(name) => write!(f, "Codegen Error: Symbol '{}' not found.", name),
            CodegenError::InvalidLValue => write!(f, "Codegen Error: Expression is not a valid L-Value for assignment."),
            CodegenError::Message(msg) => write!(f, "Codegen Error: {}", msg),
        }
    }
}
/// 允许 `BuilderError` 自动转换为 `CodegenError`。
///
/// 这样，在返回 `Result<_, CodegenError>` 的函数中，
/// 我们可以对 inkwell 的调用使用 `?`，错误会被自动包装。
impl From<BuilderError> for CodegenError {
    fn from(error: BuilderError) -> Self {
        CodegenError::Backend(error)
    }
}

/// 允许任何一种具体的错误类型自动提升为顶层的 `CompilerError`。
/// 这样，无论在哪个阶段，我们都可以方便地将错误传递出去。
impl From<LexerError> for CompilerError {
    fn from(e: LexerError) -> Self { CompilerError::Lexer(e) }
}
impl From<ParserError> for CompilerError {
    fn from(e: ParserError) -> Self { CompilerError::Parser(e) }
}
impl From<SemanticError> for CompilerError {
    fn from(e: SemanticError) -> Self { CompilerError::Semantic(e) }
}
impl From<CodegenError> for CompilerError {
    fn from(e: CodegenError) -> Self { CompilerError::Codegen(e) }
}

// --- 位置信息 ---
// Span 代表了源代码中的一个范围，(Copy trait 让它在函数间传递更方便)
#[derive(Debug, Clone, PartialEq, Copy, Default)]
pub struct Span {
    pub line: u32,
    pub column: u32,
    pub start_byte: usize, // 在源文件中的起始字节位置
    pub end_byte: usize,   // 在源文件中的结束字节位置
}