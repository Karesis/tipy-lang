/// 代表 Tipy 语言在语义分析阶段的内部类型表示。
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int32,
    Float64,
    // ... 未来可以加入 Bool, String, etc.

    /// 代表一个函数类型。
    Function {
        /// 参数类型的列表。
        params: Vec<Type>,
        /// 返回类型。对于不返回值的函数，这里是 Box<Type::Void>。
        /// 使用 Box 是因为类型定义是递归的，需要一个间接层。
        ret: Box<Type>,
    },

    /// 代表没有值的类型，主要用于函数返回值。
    Void, 
    
    /// 一个特殊的错误类型，用于在类型检查失败时防止连锁错误。
    Error,
}

// (可选) 为 Type 添加一些辅助方法，未来会很方便
impl Type {
    /// 用于在日志或错误信息中打印类型名称。
    pub fn to_string(&self) -> String {
        match self {
            Type::Int32 => "i32".to_string(),
            Type::Float64 => "f64".to_string(),
            Type::Void => "void".to_string(),
            Type::Error => "error".to_string(),
            Type::Function { params, ret } => {
                let param_types = params
                    .iter()
                    .map(|p| p.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("fn({}) -> {}", param_types, ret.to_string())
            }
        }
    }
}