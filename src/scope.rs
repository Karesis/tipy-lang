// file: src/scope.rs

use crate::types::Type;
use crate::diagnostics::{SemanticError, Span}; // 引入我们需要的错误类型
use std::collections::HashMap;

/// 代表在符号表中存储的一个符号（通常是变量或函数）。
#[derive(Debug, Clone)]
pub struct Symbol {
    /// 符号的名称。
    pub name: String,
    /// 符号的类型，使用我们定义的 `Type` 枚举。
    pub symbol_type: Type,
    /// 符号是否是可变的。
    pub is_mutable: bool,
    // 未来可以增加更多信息，如定义的位置 (span)，是否是函数参数等。
}

/// 符号表，用于在编译期间跟踪标识符的定义和作用域。
///
/// 它由一个作用域栈（`Vec<HashMap>`）实现。栈顶代表最内层（当前）作用域。
/// 这种结构天然地支持了词法作用域和变量遮蔽（shadowing）。
pub struct SymbolTable {
    /// 作用域栈。每个元素都是一个 `HashMap`，将符号名映射到 `Symbol` 结构。
    scopes: Vec<HashMap<String, Symbol>>,
}

impl SymbolTable {
    /// 创建一个新的符号表，并自动初始化全局作用域。
    pub fn new() -> Self {
        SymbolTable {
            // 初始化时，栈中已包含全局作用域
            scopes: vec![HashMap::new()],
        }
    }

    /// 进入一个新的作用域（例如，在进入函数体、if 块或 loop 块时调用）。
    ///
    /// 这会在作用域栈的顶部推入一个新的、空的哈希表。
    pub fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// 退出当前作用域（例如，在离开一个代码块时调用）。
    ///
    /// 这会从作用域栈的顶部弹出一个哈希表。为了安全，它会阻止弹出唯一的全局作用域。
    pub fn leave_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// 在**当前作用域**中定义一个新符号。
    ///
    /// # Returns
    /// - `Ok(())` 如果成功定义。
    /// - `Err(SemanticError)` 如果当前作用域中已存在同名符号。
    pub fn define(&mut self, symbol: Symbol) -> Result<(), SemanticError> {
        // .last_mut() 获取栈顶（当前作用域）的可变引用。
        let current_scope = self.scopes.last_mut().unwrap(); // 总会成功，因为总有全局作用域
        let name = symbol.name.clone();

        if current_scope.contains_key(&name) {
            // CHANGED: 返回结构化的错误，而不是 String。
            // TODO: 这里需要一个 Span，暂时用 Default。
            Err(SemanticError::SymbolAlreadyDefined { name, span: Span::default() })
        } else {
            current_scope.insert(name, symbol);
            Ok(())
        }
    }

    /// 从内到外查找一个符号。
    ///
    /// 它会从最内层（当前）作用域开始查找，如果找不到，则向外层作用域继续查找，
    /// 直到全局作用域。这正确地模拟了变量查找和遮蔽的规则。
    ///
    /// # Returns
    /// - `Some(&Symbol)` 如果找到了符号。
    /// - `None` 如果在所有可见作用域中都找不到该符号。
    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        // `.iter().rev()` 从栈顶到栈底反向迭代，完美匹配作用域查找顺序。
        for scope in self.scopes.iter().rev() {
            if let Some(symbol) = scope.get(name) {
                return Some(symbol);
            }
        }
        None
    }
}