// src/codegen.rs

use std::collections::HashMap;

use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::values::{FunctionValue, PointerValue, BasicValueEnum, BasicMetadataValueEnum};
use inkwell::types::{BasicTypeEnum, BasicType};
use inkwell::{AddressSpace, IntPredicate, FloatPredicate};
use crate::ast::{Program, TopLevelStatement, FunctionDeclaration, Statement, Expression, BlockStatement, IfExpression, WhileStatement, LoopExpression};
use crate::types::Type as TipyType;
use crate::error::CompileError;

/// 代码生成器的核心结构体
pub struct CodeGen<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    variables: Vec<HashMap<String, (PointerValue<'ctx>, BasicTypeEnum<'ctx>)>>,
    current_function: Option<FunctionValue<'ctx>>,
    // NEW: 循环上下文栈，用于处理 break/continue
    // 元组: (循环继续的目的地 BasicBlock, 循环退出的目的地 BasicBlock)
    loop_context_stack: Vec<(inkwell::basic_block::BasicBlock<'ctx>, inkwell::basic_block::BasicBlock<'ctx>)>,
}

impl<'ctx> CodeGen<'ctx> {
    /// 构造函数
    pub fn new(context: &'ctx Context, module_name: &str) -> Self {
        CodeGen {
            context,
            module: context.create_module(module_name),
            builder: context.create_builder(),
            variables: vec![HashMap::new()],
            current_function: None,
            loop_context_stack: Vec::new(), // NEW: 初始化
        }
    }

    /// (仅用于调试) 打印生成的 LLVM IR
    pub fn print_ir_to_stderr(&self) {
        self.module.print_to_stderr();
    }

    /// 将生成的 LLVM IR 保存到文件
    pub fn save_ir_to_file(&self, path: &std::path::Path) -> Result<(), &'static str> {
        self.module.print_to_file(path).map_err(|_| "Error writing IR to file")
    }

    /// 新的、两遍式的编译入口点
    pub fn compile(&mut self, program: &Program, analyzer: &crate::analyzer::SemanticAnalyzer) -> Result<(), CompileError> {
        // --- 第一遍：声明所有函数 ---
        for toplevel_stmt in &program.body {
            if let TopLevelStatement::Function(func_decl) = toplevel_stmt {
                self.compile_function_declaration(func_decl, &analyzer.symbol_table)?;
            }
        }
        
        self.declare_externs();

        // --- 第二遍：编译所有函数体 ---
        for toplevel_stmt in &program.body {
            if let TopLevelStatement::Function(func_decl) = toplevel_stmt {
                self.compile_function_body(func_decl)?;
            }
        }
        Ok(())
    }
    
    // --- 作用域管理 ---
    fn enter_scope(&mut self) { self.variables.push(HashMap::new()); }
    fn leave_scope(&mut self) { self.variables.pop(); }
    /// 在作用域栈中从内到外查找变量
    fn lookup_variable(&self, name: &str) -> Option<&(PointerValue<'ctx>, BasicTypeEnum<'ctx>)> {
        for scope in self.variables.iter().rev() {
            if let Some(var) = scope.get(name) {
                return Some(var);
            }
        }
        None
    }

    /// 第一遍：只声明函数签名，不编译函数体
    fn compile_function_declaration(&self, func_decl: &FunctionDeclaration, symbol_table: &crate::scope::SymbolTable) -> Result<(), CompileError> {
        let func_symbol = symbol_table.lookup(&func_decl.name).ok_or_else(|| CompileError::Semantic("Function not found in symbol table".to_string()))?;

        if let TipyType::Function { params, ret } = &func_symbol.symbol_type {
            // **FIX 1 & 2**: 将 Vec 类型改为 BasicMetadataTypeEnum
            let param_types: Vec<inkwell::types::BasicMetadataTypeEnum<'ctx>> = params.iter()
                .map(|p_type| self.to_llvm_basic_type(p_type).into())
                .collect();

            let fn_type = match **ret {
                TipyType::Void => self.context.void_type().fn_type(&param_types, false),
                _ => self.to_llvm_basic_type(ret).fn_type(&param_types, false),
            };
            
            self.module.add_function(&func_decl.name, fn_type, None);
            Ok(())
        } else {
            Err(CompileError::Semantic(format!("'{}' is not a function.", func_decl.name)))
        }
    }

    /// 第二遍：编译函数体
    fn compile_function_body(&mut self, func_decl: &FunctionDeclaration) -> Result<(), CompileError> {
        let function = self.module.get_function(&func_decl.name).unwrap();
        self.current_function = Some(function);
        
        let entry_block = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry_block);
        self.enter_scope();

        // 为所有参数分配空间并存入当前作用域
        for (i, param) in function.get_param_iter().enumerate() {
            let arg_name = &func_decl.params[i].name;
            param.set_name(arg_name); // 给 LLVM IR 中的参数命名，方便调试
            let arg_type = param.get_type();
            let alloca = self.create_entry_block_alloca(arg_type, arg_name)?;
            self.builder.build_store(alloca, param)?;
            self.variables.last_mut().unwrap().insert(arg_name.clone(), (alloca, arg_type));
        }

        self.compile_block_statement(&func_decl.body)?;

        // 检查函数是否已正确返回 (terminator)
        if function.get_last_basic_block().and_then(|bb| bb.get_terminator()).is_none() {
            if function.get_type().get_return_type().is_none() {
                self.builder.build_return(None)?;
            } else if func_decl.name == "main" {
                 let i32_type = self.context.i32_type();
                 self.builder.build_return(Some(&i32_type.const_int(0, false)))?;
            } else {
                return Err(CompileError::Semantic(format!("Function '{}' must return a value.", func_decl.name)));
            }
        }
        
        self.leave_scope();
        Ok(())
    }
    
    // UPDATED: `compile_statement` 调度所有新语句
    fn compile_statement(&mut self, stmt: &Statement) -> Result<(), CompileError> {
        match stmt {
            Statement::VarDeclaration(var_decl) => self.compile_var_declaration(var_decl),
            Statement::Return(ret_stmt) => {
                let ret_val = match &ret_stmt.value {
                    Some(expr) => Some(self.compile_expression(expr)?),
                    None => None,
                };
                self.builder.build_return(ret_val.as_ref().map(|v| v as &dyn inkwell::values::BasicValue))?;
                Ok(()) 
            },
            Statement::Expression(expr) => self.compile_expression(expr).map(|_| ()),
            Statement::Block(block_stmt) => self.compile_block_statement(block_stmt).map(|_| ()), // 忽略块返回值
            // NEW: 调度新的控制流语句
            Statement::While(while_stmt) => self.compile_while_statement(while_stmt),
            Statement::Break(_) => self.compile_break_statement(),
            Statement::Continue(_) => self.compile_continue_statement(),
        }
    }

    // UPDATED: `compile_block_statement` 现在能返回块的值，用于 if/loop 表达式
    fn compile_block_statement(&mut self, block: &BlockStatement) -> Result<Option<BasicValueEnum<'ctx>>, CompileError> {
        self.enter_scope();
        let mut last_val = None;
        for (i, stmt) in block.statements.iter().enumerate() {
            // 如果是块中最后一个语句，并且是表达式语句，我们认为它是块的返回值
            if i == block.statements.len() - 1 {
                if let Statement::Expression(expr) = stmt {
                    last_val = Some(self.compile_expression(expr)?);
                } else {
                    self.compile_statement(stmt)?;
                }
            } else {
                self.compile_statement(stmt)?;
            }
        }
        self.leave_scope();
        Ok(last_val)
    }

    fn compile_expression(&mut self, expr: &Expression) -> Result<BasicValueEnum<'ctx>, CompileError> {
        match expr {
            Expression::Literal(lit) => self.compile_literal(lit),

            Expression::Identifier(name) => {
                let (ptr, var_type) = self.lookup_variable(name)
                    .ok_or_else(|| CompileError::Semantic(format!("Use of undefined variable '{}'", name)))?;
                Ok(self.builder.build_load(*var_type, *ptr, name)?)
            },

            Expression::Infix(infix_expr) => {
                let left = self.compile_expression(&infix_expr.left)?;
                let right = self.compile_expression(&infix_expr.right)?;

                if left.is_int_value() && right.is_int_value() {
                    let l = left.into_int_value();
                    let r = right.into_int_value();
                    // UPDATED: 使用 match 匹配所有运算符
                    match infix_expr.op {
                        // 算术
                        crate::ast::Operator::Plus => Ok(self.builder.build_int_add(l, r, "add")?.into()),
                        crate::ast::Operator::Minus => Ok(self.builder.build_int_sub(l, r, "sub")?.into()),
                        crate::ast::Operator::Multiply => Ok(self.builder.build_int_mul(l, r, "mul")?.into()),
                        crate::ast::Operator::Divide => Ok(self.builder.build_int_signed_div(l, r, "div")?.into()),
                        // NEW: 比较
                        crate::ast::Operator::Equal => Ok(self.builder.build_int_compare(IntPredicate::EQ, l, r, "eq")?.into()),
                        crate::ast::Operator::NotEqual => Ok(self.builder.build_int_compare(IntPredicate::NE, l, r, "ne")?.into()),
                        crate::ast::Operator::LessThan => Ok(self.builder.build_int_compare(IntPredicate::SLT, l, r, "lt")?.into()),
                        crate::ast::Operator::LessEqual => Ok(self.builder.build_int_compare(IntPredicate::SLE, l, r, "le")?.into()),
                        crate::ast::Operator::GreaterThan => Ok(self.builder.build_int_compare(IntPredicate::SGT, l, r, "gt")?.into()),
                        crate::ast::Operator::GreaterEqual => Ok(self.builder.build_int_compare(IntPredicate::SGE, l, r, "ge")?.into()),
                    }
                } else if left.is_float_value() && right.is_float_value() {
                    // ... 类似地，为浮点数添加 build_float_compare ...
                    unimplemented!("Float comparisons not implemented yet")
                } else {
                    Err(CompileError::Semantic("Mismatched types in binary operation.".to_string()))
                }
            },

            Expression::Call(call_expr) => {
                let callee_name = match &*call_expr.function {
                    Expression::Identifier(name) => name,
                    _ => return Err(CompileError::Semantic("Function call target must be a simple name.".to_string())),
                };

                let function = self.module.get_function(callee_name).ok_or_else(|| {
                    CompileError::Semantic(format!("Call to undefined function '{}'", callee_name))
                })?;

                let compiled_args: Vec<BasicMetadataValueEnum> = call_expr.arguments.iter()
                    .map(|arg| self.compile_expression(arg))
                    .collect::<Result<Vec<BasicValueEnum>, CompileError>>()?
                    .into_iter()
                    .map(|val| val.into())
                    .collect();
                
                let call_site = self.builder.build_call(function, &compiled_args, "tmpcall")?;
                
                match call_site.try_as_basic_value().left() {
                    Some(value) => Ok(value),
                    None => Err(CompileError::Semantic(format!("Function '{}' returns void and cannot be used in an expression.", callee_name))),
                }
            },

            // --- NEW: 补全 Assignment 表达式的编译逻辑 ---
            Expression::Assignment(assign_expr) => {
                let compiled_value = self.compile_expression(&assign_expr.value)?;
                let (ptr, _) = self.lookup_variable(&assign_expr.name).ok_or_else(|| {
                    CompileError::Semantic(format!("Assignment to undefined variable '{}'", assign_expr.name))
                })?;

                self.builder.build_store(*ptr, compiled_value)?;
                Ok(compiled_value)
            },

            // --- NEW: 补全 Prefix 表达式的编译逻辑 ---
            Expression::Prefix(prefix_expr) => {
                let value = self.compile_expression(&prefix_expr.right)?;
                match prefix_expr.op {
                    crate::ast::PrefixOperator::Minus => {
                        if value.is_int_value() {
                            Ok(self.builder.build_int_neg(value.into_int_value(), "tmpneg")?.into())
                        } else if value.is_float_value() {
                            Ok(self.builder.build_float_neg(value.into_float_value(), "tmpfneg")?.into())
                        } else {
                            Err(CompileError::Semantic("Unary minus can only be applied to numbers.".to_string()))
                        }
                    },
                    // NEW: 逻辑非 `!`
                    crate::ast::PrefixOperator::Not => {
                        let bool_true = self.context.bool_type().const_int(1, false);
                        let result = self.builder.build_xor(value.into_int_value(), bool_true, "not")?;
                        Ok(result.into())
                    }
                }
            },

            // --- NEW: 新增控制流表达式的编译 ---
            Expression::If(if_expr) => self.compile_if_expression(if_expr),
            Expression::Loop(loop_expr) => self.compile_loop_expression(loop_expr),
            Expression::Block(block_stmt) => self.compile_block_statement(block_stmt)?.ok_or_else(|| CompileError::Semantic("Block used as expression did not return a value".to_string())),
        }
    }
    
    fn compile_var_declaration(&mut self, var_decl: &crate::ast::VarDeclaration) -> Result<(), CompileError> {
        let initial_value = match &var_decl.value {
            Some(expr) => self.compile_expression(expr)?,
            None => return Err(CompileError::Semantic("Variables must be initialized for now.".to_string())),
        };
        
        // **FIX 3 (Application)**: 将类型和指针一起存入
        let var_type = initial_value.get_type();
        let alloca = self.create_entry_block_alloca(var_type, &var_decl.name)?;
        self.builder.build_store(alloca, initial_value)?;
        self.variables.last_mut().unwrap().insert(var_decl.name.clone(), (alloca, var_type));
        Ok(())
    }

    // --- NEW: 所有新增功能的代码生成函数 ---
    
    fn compile_if_expression(&mut self, if_expr: &IfExpression) -> Result<BasicValueEnum<'ctx>, CompileError> {
        let function = self.current_function.unwrap();

        // 1. 编译条件
        let cond_val = self.compile_expression(&if_expr.condition)?;
        let cond = self.builder.build_int_compare(IntPredicate::NE, cond_val.into_int_value(), self.context.bool_type().const_int(0, false), "ifcond")?;

        // 2. 创建基本块
        let then_bb = self.context.append_basic_block(function, "then");
        let else_bb = self.context.append_basic_block(function, "else");
        let merge_bb = self.context.append_basic_block(function, "ifcont");

        // 3. 创建条件分支
        self.builder.build_conditional_branch(cond, then_bb, else_bb)?;

        // 4. 编译 then 块
        self.builder.position_at_end(then_bb);
        let then_val = self.compile_block_statement(&if_expr.consequence)?.unwrap(); // 假设 if 作为表达式时，块必有值
        self.builder.build_unconditional_branch(merge_bb)?;
        let then_bb = self.builder.get_insert_block().unwrap(); // 获取 then 块的最终位置

        // 5. 编译 else 块
        self.builder.position_at_end(else_bb);
        let else_val = self.compile_expression(if_expr.alternative.as_ref().unwrap())?; // 假设 if 作为表达式时必有 else
        self.builder.build_unconditional_branch(merge_bb)?;
        let else_bb = self.builder.get_insert_block().unwrap();

        // 6. 在 merge 块中使用 PHI 节点汇集结果
        self.builder.position_at_end(merge_bb);
        let phi = self.builder.build_phi(then_val.get_type(), "iftmp")?;
        phi.add_incoming(&[(&then_val, then_bb), (&else_val, else_bb)]);

        Ok(phi.as_basic_value())
    }

    fn compile_while_statement(&mut self, while_stmt: &WhileStatement) -> Result<(), CompileError> {
        let function = self.current_function.unwrap();
        
        let cond_bb = self.context.append_basic_block(function, "loopcond");
        let body_bb = self.context.append_basic_block(function, "loopbody");
        let after_bb = self.context.append_basic_block(function, "afterloop");
        
        // break/continue 跳转的目标
        self.loop_context_stack.push((cond_bb, after_bb));
        
        self.builder.build_unconditional_branch(cond_bb)?;
        
        // 编译条件块
        self.builder.position_at_end(cond_bb);
        let cond_val = self.compile_expression(&while_stmt.condition)?;
        let cond = self.builder.build_int_compare(IntPredicate::NE, cond_val.into_int_value(), self.context.bool_type().const_int(0, false), "whilecond")?;
        self.builder.build_conditional_branch(cond, body_bb, after_bb)?;
        
        // 编译循环体
        self.builder.position_at_end(body_bb);
        self.compile_block_statement(&while_stmt.body)?;
        self.builder.build_unconditional_branch(cond_bb)?; // 循环体结束后跳回条件判断
        
        // 编译循环结束后的代码
        self.builder.position_at_end(after_bb);
        self.loop_context_stack.pop();
        Ok(())
    }

    fn compile_loop_expression(&mut self, loop_expr: &LoopExpression) -> Result<BasicValueEnum<'ctx>, CompileError> {
        // ... `loop` 的实现与 `while` 类似，但更简单，因为它没有条件块，直接进入循环体
        // `break` 可以带返回值，同样需要使用 PHI 节点
        unimplemented!("`loop` expression codegen is not implemented yet.");
    }
    
    fn compile_break_statement(&mut self) -> Result<(), CompileError> {
        if let Some((_, break_bb)) = self.loop_context_stack.last() {
            self.builder.build_unconditional_branch(*break_bb)?;
            Ok(())
        } else {
            Err(CompileError::Semantic("`break` used outside of a loop".to_string()))
        }
    }

    fn compile_continue_statement(&mut self) -> Result<(), CompileError> {
        if let Some((continue_bb, _)) = self.loop_context_stack.last() {
            self.builder.build_unconditional_branch(*continue_bb)?;
            Ok(())
        } else {
            Err(CompileError::Semantic("`continue` used outside of a loop".to_string()))
        }
    }

    // --- 辅助函数 ---
    
    fn declare_externs(&mut self) {
        // 声明 C 的 `puts` 函数 `fn(i8*) -> i32`，用于 `print`
        let i32_type = self.context.i32_type();
        let i8_ptr_type = self.context.ptr_type(AddressSpace::default());
        let puts_fn_type = i32_type.fn_type(&[i8_ptr_type.into()], false);
        self.module.add_function("puts", puts_fn_type, None);
    }
    
    /// 在函数入口块的开头创建 alloca 指令，这是 LLVM 的要求/好习惯
    fn create_entry_block_alloca<T: BasicType<'ctx>>(&self, ty: T, name: &str) -> Result<PointerValue<'ctx>, CompileError> {
        let builder = self.context.create_builder();
        let entry = self.current_function.unwrap().get_first_basic_block().unwrap();
        match entry.get_first_instruction() {
            Some(instr) => builder.position_before(&instr),
            None => builder.position_at_end(entry),
        }
        Ok(builder.build_alloca(ty, name)?)
    }
    
    // UPDATED: `to_llvm_basic_type` 支持 bool
    fn to_llvm_basic_type(&self, tipy_type: &TipyType) -> BasicTypeEnum<'ctx> {
        match tipy_type {
            TipyType::Int32 => self.context.i32_type().into(),
            TipyType::Float64 => self.context.f64_type().into(),
            TipyType::Bool => self.context.bool_type().into(), // <-- 新增
            _ => unimplemented!("Type conversion not implemented for this TipyType."),
        }
    }
    
    /// 编译字面量
    fn compile_literal(&self, lit: &crate::token::Literal) -> Result<BasicValueEnum<'ctx>, CompileError> {
        match lit {
            crate::token::Literal::Integer(val) => Ok(self.context.i32_type().const_int(*val as u64, false).into()),
            crate::token::Literal::Float(val) => Ok(self.context.f64_type().const_float(*val).into()),
            crate::token::Literal::String(s) => {
                // 创建一个全局字符串常量，并返回指向它的指针
                Ok(self.builder.build_global_string_ptr(s, ".str")?.as_pointer_value().into())
            }
        }
    }
}