// file: src/types.rs

use std::fmt;

/// 代表 Tipy 语言在语义分析阶段的内部类型表示。
///
/// 这是类型的“范畴”或“模板”，而不是具体的值。
/// 例如，`Type::I32` 代表了所有32位有符号整数的类型。
#[derive(Debug, Clone, PartialEq, Eq, Hash)] // <-- Eq 和 Hash 在未来使用 HashMap<Type, ...> 时会很有用
pub enum Type {
    // --- 原生类型 ---
    I8, I16, I32, I64, I128, Isize,
    U8, U16, U32, U64, U128, Usize,
    F32, F64,
    Bool,
    Char,
    /// 内置的、拥有所有权的字符串类型。
    Str,

    // --- 复合类型 ---
    /// 指针类型
    Pointer {
        /// 指针本身是否可变 (`~^T`)
        is_mutable_ptr: bool,
        /// 指针指向的数据是否可变 (`^~T`)
        is_mutable_pointee: bool,
        /// 指向的类型
        pointee: Box<Type>,
    },
    /// 函数类型
    Function {
        params: Vec<Type>,
        ret: Box<Type>,
    },

    // --- 用户自定义类型 (为未来预留) ---
    Struct { name: String },
    Enum { name: String },

    // --- 特殊类型 ---
    /// 代表没有值的类型，通常用作不返回任何东西的函数的返回类型。
    Void,
    /// 一个特殊的错误类型，用于在类型检查失败时防止连锁错误。
    /// 当一个表达式的类型无法确定时，可以赋予它此类型，
    /// 之后所有使用到这个表达式的地方都可以识别出这是一个已知错误，从而避免报告大量无关的后续错误。
    Error,
}

/// 实现 Display trait，使得类型可以被方便地打印成用户友好的格式。
impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::I8 => write!(f, "i8"),
            Type::I32 => write!(f, "i32"),
            // ... 其他所有原生类型 ...
            Type::Str => write!(f, "str"),
            Type::Pointer { is_mutable_ptr, is_mutable_pointee, pointee } => {
                let mut s = String::new();
                if *is_mutable_ptr { s.push('~'); }
                s.push('^');
                if *is_mutable_pointee { s.push('~'); }
                write!(f, "{}{}", s, pointee)
            }
            Type::Function { params, ret } => {
                let param_types = params.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", ");
                write!(f, "fn({}) -> {}", param_types, ret)
            }
            Type::Struct { name } => write!(f, "{}", name),
            Type::Enum { name } => write!(f, "{}", name),
            Type::Void => write!(f, "void"),
            Type::Error => write!(f, "<type error>"),
            // ... 为了简洁，省略了所有原生类型的匹配臂，但实际中应全部实现 ...
            _ => write!(f, "unimplemented_type"), // 临时占位
        }
    }
}