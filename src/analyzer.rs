// src/analyzer.rs

// UPDATED: 引入所有需要的 AST 节点和 semantic 模块
use crate::ast::{
    Program, Statement, Expression, TopLevelStatement, FunctionDeclaration,
    VarDeclaration, ReturnStatement, BlockStatement
};
use crate::semantic::{Symbol, SymbolTable};
use crate::types::Type;

pub struct SemanticAnalyzer {
    pub symbol_table: SymbolTable,
    pub errors: Vec<String>,
    // NEW: 用于在分析函数体时，跟踪当前函数的预期返回类型
    current_return_type: Option<Type>, 
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        SemanticAnalyzer {
            symbol_table: SymbolTable::new(),
            errors: Vec::new(),
            current_return_type: None,
        }
    }

    // REWRITTEN: 分析的入口点，现在执行两遍式分析
    pub fn analyze(&mut self, program: &Program) {
        // --- 第一遍：注册所有函数签名 ---
        for toplevel_stmt in &program.body {
            // 因为 TopLevelStatement 目前只有 Function 变体，所以可以直接解构
            let TopLevelStatement::Function(func_decl) = toplevel_stmt;
            
            if let Err(e) = self.register_function_signature(func_decl) {
                self.errors.push(e);
            }
        }
        
        if !self.errors.is_empty() {
            return;
        }

        // --- 第二遍：分析所有函数体 ---
        for toplevel_stmt in &program.body {
            // 同样，直接解构
            let TopLevelStatement::Function(func_decl) = toplevel_stmt;

            if let Err(e) = self.analyze_function_body(func_decl) {
                self.errors.push(e);
            }
        }
    }
    
    // NEW: 第一遍分析的实现
    fn register_function_signature(&mut self, func_decl: &FunctionDeclaration) -> Result<(), String> {
        let mut param_types = Vec::new();
        for p in &func_decl.params {
            param_types.push(self.string_to_type(&p.param_type)?);
        }
        
        let ret_type = self.string_to_type(&func_decl.return_type)?;
        
        let func_type = Type::Function {
            params: param_types,
            ret: Box::new(ret_type),
        };

        let symbol = Symbol {
            name: func_decl.name.clone(),
            symbol_type: func_type,
            is_mutable: false, // 函数本身是不可变的
        };

        // 将函数定义在全局作用域
        self.symbol_table.define(symbol)
    }
    
    // NEW: 第二遍分析的实现
    fn analyze_function_body(&mut self, func_decl: &FunctionDeclaration) -> Result<(), String> {
        // 进入函数，创建一个新的作用域
        self.symbol_table.enter_scope();
        
        // 设置当前函数的返回类型，以便在 ret 语句中进行检查
        self.current_return_type = Some(self.string_to_type(&func_decl.return_type)?);

        // 将所有函数参数定义为新作用域中的变量
        for p in &func_decl.params {
            let param_type = self.string_to_type(&p.param_type)?;
            let param_symbol = Symbol {
                name: p.name.clone(),
                symbol_type: param_type,
                is_mutable: false, // Tipy 的函数参数默认不可变
            };
            self.symbol_table.define(param_symbol)?;
        }
        
        // 分析函数体中的每一条语句
        self.analyze_block_statement(&func_decl.body)?;
        
        // 离开函数，销毁其作用域
        self.symbol_table.leave_scope();
        self.current_return_type = None; // 清理状态

        Ok(())
    }

    // UPDATED: 现在只用于分析函数体内的语句块
    fn analyze_block_statement(&mut self, block: &BlockStatement) -> Result<(), String> {
        for statement in &block.statements {
            self.analyze_statement(statement)?;
        }
        Ok(())
    }

    fn analyze_statement(&mut self, statement: &Statement) -> Result<(), String> {
        // UPDATED: 使用新的 AST 结构进行模式匹配
        match statement {
            Statement::VarDeclaration(var_decl) => self.analyze_var_declaration(var_decl),
            Statement::Expression(expression) => self.analyze_expression(expression).map(|_| ()), // 忽略表达式结果类型
            Statement::Return(ret_stmt) => self.analyze_return_statement(ret_stmt),
            Statement::Block(block_stmt) => {
                // 对于嵌套的代码块，创建一个新的局部作用域
                self.symbol_table.enter_scope();
                self.analyze_block_statement(block_stmt)?;
                self.symbol_table.leave_scope();
                Ok(())
            }
        }
    }

    // UPDATED: 参数变为 VarDeclaration struct
    fn analyze_var_declaration(&mut self, var_decl: &VarDeclaration) -> Result<(), String> {
        let var_type = self.string_to_type(&var_decl.var_type)?;

        if let Some(initial_value) = &var_decl.value {
            let value_type = self.analyze_expression(initial_value)?;
            if value_type != var_type {
                return Err(format!("Type mismatch: cannot assign value of type {} to variable '{}' of type {}",
                    value_type.to_string(), var_decl.name, var_type.to_string()));
            }
        }

        let symbol = Symbol {
            name: var_decl.name.clone(),
            symbol_type: var_type,
            is_mutable: var_decl.is_mutable,
        };
        self.symbol_table.define(symbol)
    }

    // NEW: 分析返回语句
    fn analyze_return_statement(&mut self, ret_stmt: &ReturnStatement) -> Result<(), String> {
        let expected = self.current_return_type.clone().unwrap_or(Type::Error);

        let actual = match &ret_stmt.value {
            Some(expr) => self.analyze_expression(expr)?,
            None => Type::Void,
        };

        if actual != expected {
            return Err(format!("Return type mismatch: expected {}, but got {}",
                expected.to_string(), actual.to_string()));
        }
        Ok(())
    }
    
    // REWRITTEN: 表达式分析，适配新 AST 并增加了函数调用分析
    fn analyze_expression(&mut self, expression: &Expression) -> Result<Type, String> {
        match expression {
            Expression::Literal(lit) => match lit {
                crate::token::Literal::Integer(_) => Ok(Type::Int32),
                crate::token::Literal::Float(_) => Ok(Type::Float64),
                _ => Err("String literals are not supported yet.".to_string()),
            },
            Expression::Identifier(name) => {
                if let Some(symbol) = self.symbol_table.lookup(name) {
                    Ok(symbol.symbol_type.clone())
                } else {
                    Err(format!("Use of undefined variable or function '{}'", name))
                }
            },
            Expression::Assignment(assign_expr) => {
                let var = self.symbol_table.lookup(&assign_expr.name)
                    .ok_or_else(|| format!("Cannot assign to undefined variable '{}'", assign_expr.name))?;
                if !var.is_mutable {
                    return Err(format!("Cannot assign to immutable variable '{}'", assign_expr.name));
                }
                let expected_type = var.symbol_type.clone();
                let value_type = self.analyze_expression(&assign_expr.value)?;
                if expected_type != value_type {
                    return Err(format!("Type mismatch on assignment. Expected {}, got {}",
                        expected_type.to_string(), value_type.to_string()));
                }
                Ok(expected_type)
            },
            Expression::Infix(infix_expr) => {
                let left_type = self.analyze_expression(&infix_expr.left)?;
                let right_type = self.analyze_expression(&infix_expr.right)?;
                if left_type != right_type {
                    return Err(format!("Type mismatch in binary operation. Found {} and {}",
                        left_type.to_string(), right_type.to_string()));
                }
                // 这里可以添加更复杂的规则，但目前 Tipy 只支持同类型运算
                match left_type {
                    Type::Int32 | Type::Float64 => Ok(left_type),
                    _ => Err(format!("Operator {:?} cannot be applied to type {}",
                        infix_expr.op, left_type.to_string())),
                }
            },
            Expression::Prefix(prefix_expr) => {
                let right_type = self.analyze_expression(&prefix_expr.right)?;
                match right_type {
                    Type::Int32 | Type::Float64 => Ok(right_type),
                    _ => Err(format!("Prefix operator {:?} cannot be applied to type {}",
                        prefix_expr.op, right_type.to_string())),
                }
            },
            Expression::Call(call_expr) => {
                // NEW: 核心的函数调用分析
                let callee_type = self.analyze_expression(&call_expr.function)?;
                
                match callee_type {
                    Type::Function { params: expected_params, ret: ret_type } => {
                        // 1. 检查参数数量 (Arity Check)
                        if call_expr.arguments.len() != expected_params.len() {
                            return Err(format!("Function call error: expected {} arguments, but got {}",
                                expected_params.len(), call_expr.arguments.len()));
                        }
                        // 2. 逐个检查参数类型
                        for (arg_expr, expected_type) in call_expr.arguments.iter().zip(expected_params.iter()) {
                            let arg_type = self.analyze_expression(arg_expr)?;
                            if arg_type != *expected_type {
                                return Err(format!("Function call type mismatch: expected argument of type {}, but got {}",
                                    expected_type.to_string(), arg_type.to_string()));
                            }
                        }
                        // 3. 如果所有检查都通过，则该表达式的类型就是函数的返回类型
                        Ok(*ret_type)
                    },
                    _ => Err(format!("Cannot call non-function type '{}'", callee_type.to_string())),
                }
            }
        }
    }

    // NEW: 一个辅助函数，用于将字符串类型名转换为内部 Type 枚举
    fn string_to_type(&self, type_str: &str) -> Result<Type, String> {
        match type_str {
            "i32" => Ok(Type::Int32),
            "f64" => Ok(Type::Float64),
            "void" => Ok(Type::Void),
            _ => Err(format!("Unknown type: {}", type_str)),
        }
    }
}



