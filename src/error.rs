// 我们可以把它定义在 codegen.rs 的顶部，或者一个专门的 error.rs 文件中

use inkwell::builder::BuilderError;
use std::fmt; // 引入格式化 trait

// 为了能打印错误，我们需要为它实现 Debug 和 Display trait
#[derive(Debug)]
pub enum CompileError {
    Backend(BuilderError),          // 用于包装来自 inkwell 的后端错误
    Semantic(String),               // 用于我们自己的语义错误，用 String 保存错误信息
}

// 实现 Display trait，让我们的错误可以被友好地打印出来
impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CompileError::Backend(e) => write!(f, "Backend Error: {}", e),
            CompileError::Semantic(s) => write!(f, "Semantic Error: {}", s),
        }
    }
}

impl From<BuilderError> for CompileError {
    fn from(error: BuilderError) -> Self {
        CompileError::Backend(error)
    }
}