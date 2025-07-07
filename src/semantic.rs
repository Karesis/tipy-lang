use crate::types::Type;

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub symbol_type: Type,
    pub is_mutable: bool,
}

pub struct SymbolTable {
    scopes: Vec<HashMap<String, Symbol>>,
}

impl SymbolTable {
    pub fn new() -> Self {
        // 初始化时，先创建一个全局作用域
        let mut scopes = Vec::new();
        scopes.push(HashMap::new());
        SymbolTable { scopes }
    }

    // 进入新作用域（比如进入一个函数体）
    pub fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    // 退出当前作用域
    pub fn leave_scope(&mut self) {
        if self.scopes.len() > 1 { // 不能移除全局作用域
            self.scopes.pop();
        }
    }

    pub fn define(&mut self, symbol: Symbol) -> Result<(), String> {
        // .last_mut() 获取栈顶元素的可变引用，也就是我们当前的、最内层的作用域
        if let Some(current_scope) = self.scopes.last_mut() {
            let name = symbol.name.clone();
            if current_scope.contains_key(&name) {
                // 变量重定义错误
                return Err(format!("Symbol '{}' is already defined in this scope.", name));
            }
            current_scope.insert(name, symbol);
            Ok(())
        } else {
            // 理论上不会发生，因为我们总是有至少一个全局作用域
            Err("Cannot define symbol: no scope exists.".to_string())
        }
    }

    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        // .iter().rev() 会创建一个反向迭代器，实现从栈顶到栈底的遍历
        // 这完美地模拟了从内到外的作用域查找规则
        for scope in self.scopes.iter().rev() {
            if let Some(symbol) = scope.get(name) {
                // 在某个作用域找到了，立即返回
                return Some(symbol);
            }
        }
        // 遍历完所有作用域都没找到
        None
    }
}