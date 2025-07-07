// file: src/diagnostics.rs

use crate::token::Token;
use crate::types::Type;

// --- 统一的编译器错误类型 ---
#[derive(Debug, Clone, PartialEq)]
pub enum CompilerError {
    /// 词法分析器错误
    Lexer(LexerError),
    /// 语法分析器错误
    Parser(ParserError), 
    /// 语意分析错误
    Semantic(SemanticError),
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