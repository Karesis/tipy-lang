// src/token.rs

/// 代表 Tipy 语言中的一个关键字。
/// 关键字是语言保留的标识符，不能用作变量名或函数名。
#[derive(Debug, Clone, PartialEq)]
pub enum Keyword {
    // --- v0.0.3 新增/核心关键字 ---
    /// `ret` 关键字，用于从函数中提前返回。
    Ret,
    /// `if` 关键字，用于条件分支。
    If,
    /// `else` 关键字，用于条件分支。
    Else,
    /// `elif` 关键字，用于条件分支
    Elif,
    /// `true` 关键字，布尔值真。
    True,
    /// `false` 关键字，布尔值假。
    False,

    // --- 规范中定义的、为未来版本准备的关键字 ---
    /// `loop` 关键字，用于无限循环。
    Loop,
    /// `while` 关键字，用于条件循环。
    While,
    /// `break` 关键字，用于跳出循环。
    Break,
    /// `continue` 关键字，用于跳到下一次循环。
    Continue,
    /// `class` 关键字，用于定义类。
    Class,
    /// `enum` 关键字，用于定义枚举。
    Enum,
    /// `match` 关键字，用于模式匹配。
    Match,
    /// `new` 关键字，用于在堆上分配内存。
    New,
    /// `free` 关键字，用于安全释放内存。
    Free,
    /// `None` 关键字，用于表示 Option 类型的空值。
    None,
}

/// 代表一个字面量值。
/// 字面量是源代码中表示固定值的表示法。
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    /// 字符串字面量, e.g., "Hello, Tipy!"
    String(String),
    /// 整数类型字面量, e.g., 10, 42
    Integer(i64),
    /// 浮点数类型字面量, e.g., 3.14, 0.5
    Float(f64),
}

/// 代表 Tipy 源代码经过词法分析后产生的单个 Token。
/// 这是构成语言语法结构的基本单元。
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// 文件结束符 (End of File)，表示源代码已读取完毕。
    Eof,
    /// 非法字符，表示词法分析器遇到了无法识别的字符。
    Illegal(char),

    // --- 标识与值 ---
    /// 标识符，如变量名、函数名 `my_var`, `add`。
    Identifier(String),
    /// 字面量，如 `123`, `"hello"`。
    Literal(Literal),
    /// 关键字，如 `if`, `ret`。
    Keyword(Keyword),

    // --- 分隔符 (Delimiters) ---
    /// 左圆括号 `(`.
    LParen,
    /// 右圆括号 `)`.
    RParen,
    /// 左花括号 `{`.
    LBrace,
    /// 右花括号 `}`.
    RBrace,
    /// 逗号 `,`.
    Comma,
    /// 分号 `;`.
    Semicolon,

    // --- 运算符与特殊符号 ---
    /// 赋值符号 `=`.
    Equal,
    /// 加号 `+`.
    Plus,
    /// 减号 `-`.
    Minus,
    /// 乘号 `*`.
    Star,
    /// 除号 `/`.
    Slash,
    /// 类型声明冒号 `:`.
    Colon,
    /// 可变性标记 `~`.
    Tilde,
    
    // --- v0.0.3 新增，用于函数定义 ---
    /// 函数返回类型箭头 `->`.
    Arrow,

    // --- 为未来版本准备的符号 ---
    /// 指针类型符号 `^`.
    Caret,
    /// 枚举变体分隔符 `|`.
    Pipe,
}