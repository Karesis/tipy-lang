// file: src/codegen.rs

use std::collections::HashMap;
use std::path::Path;

// --- LLVM 后端库 (Inkwell) 引入 ---
// 这里引入了与 LLVM IR 生成直接相关的核心类型。
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::{BasicMetadataValueEnum, BasicValueEnum, FunctionValue, PointerValue};
use inkwell::{AddressSpace, FloatPredicate, IntPredicate};

// --- Tipy 编译器内部模块引入 ---

// 引入抽象语法树 (AST)。代码生成器将遍历这些 AST 节点来生成代码。
use crate::ast::{
    BlockStatement, Expression, FunctionDeclaration, IfExpression, LoopExpression, Program, Statement,
    TopLevelStatement, WhileStatement,VarDeclaration, BreakStatement, ContinueStatement,
};

// 引入运算符，编译中缀表达式需要用到
use crate::ast::Operator;

// 引入我们为后端错误处理定义的新类型。
// CRITICAL: 移除了旧的 `error::CompileError`，换用统一的诊断系统。
use crate::diagnostics::{CodegenError, CompilerError};

// 引入我们内部的类型系统，并使用 `as` 关键字重命名，以避免与 LLVM 的类型定义冲突。
// e.g., TipyType::I32 (我们的) vs inkwell::types::IntType (LLVM 的)
use crate::types::Type as TipyType;

// 引入字面量用于转换和生成
use crate::token::Literal;

/// 代码生成器的核心结构体。
///
/// `CodeGen` 负责将经过语义分析验证后的、语义正确的 AST
/// 转换为 LLVM 中间代码 (IR)。它是一个访问者 (Visitor)，会深度优先
/// 遍历 AST，并使用 `inkwell` 库提供的 builder 来逐条生成指令。
///
/// `'ctx` 生命周期参数是 `inkwell` 库的要求，它确保所有与 LLVM 相关的
/// 对象（如 `Module`, `Builder`, `Type`, `Value`）都存活在同一个
/// `Context` 的生命周期内，保证内存安全。
pub struct CodeGen<'ctx> {
    /// LLVM 的全局上下文，所有 LLVM 对象都与它关联。
    context: &'ctx Context,

    /// 当前正在构建的 LLVM 模块。一个模块可以看作是一个翻译单元，
    /// 它包含了所有的函数、全局变量等。最终会被编译成 `.o` 文件。
    module: Module<'ctx>,

    /// LLVM 指令构建器。它是我们的“画笔”，用于在基本块 (BasicBlock)
    /// 中创建和插入 LLVM 指令（如加法、跳转、函数调用等）。
    builder: Builder<'ctx>,

    /// 用于代码生成的“符号表”。
    ///
    /// 与语义分析的 `SymbolTable` 不同，这里存储的不是类型信息，
    /// 而是变量名到其在栈上分配的内存位置 (`PointerValue`) 的映射。
    /// `Vec<HashMap>` 结构同样用于支持词法作用域。
    /// 同时，存储一个元组，包含指针和类型
    variables: Vec<HashMap<String, (PointerValue<'ctx>, BasicTypeEnum<'ctx>)>>,
    /// 一个指向当前正在生成的 `FunctionValue` 的引用。
    ///
    /// 这对于生成 `ret` 指令至关重要，因为 `ret` 指令需要知道
    /// 它属于哪个函数。
    current_function: Option<FunctionValue<'ctx>>,

    /// 循环上下文栈，用于正确生成 `break` 和 `continue` 的跳转指令。
    ///
    /// 每当进入一个循环 (`loop` 或 `while`)，我们会将该循环的
    /// **继续块 (continue_block)** 和 **退出块 (exit_block)** 的标签
    /// 压入栈中。遇到 `break` 时，就无条件跳转到栈顶的 `exit_block`；
    /// 遇到 `continue` 时，就跳转到 `continue_block`。
    /// 元组中包含的第三个元素，是一个可选的 PointerValue，
    /// 用于存放 `loop` 表达式的返回值内存地址。
    ///
    /// 使用栈结构可以正确处理嵌套循环。
    loop_context_stack: Vec<(
        inkwell::basic_block::BasicBlock<'ctx>, // continue_block (循环体或条件)
        inkwell::basic_block::BasicBlock<'ctx>, // exit_block (循环结束后的块)
        Option<PointerValue<'ctx>>,             // result_alloca (存放 loop 返回值的地方)
    )>,
}

impl<'ctx> CodeGen<'ctx> {

    /// 创建一个新的 `CodeGen` 实例。
    ///
    /// # Arguments
    /// * `context` - 一个指向 `inkwell` 全局上下文的引用。
    /// * `module_name` - 将要创建的 LLVM 模块的名称。
    ///
    /// # Returns
    /// 一个全新的 `CodeGen` 实例，它内部已经创建好了 `Module` 和 `Builder`，
    /// 并初始化了一个包含全局作用域的变量表。
    pub fn new(context: &'ctx Context, module_name: &str) -> Self {
        let module = context.create_module(module_name);
        let builder = context.create_builder();
        
        CodeGen {
            context,
            module,
            builder,
            variables: vec![HashMap::new()], // 初始化全局作用域
            current_function: None,
            loop_context_stack: Vec::new(),
        }
    }

    /// 将代码生成器的主入口点，负责将整个程序的 AST 编译成 LLVM IR。
    ///
    /// 它采用两遍式编译策略，以正确处理函数的前向引用。
    ///
    /// **第一遍**: 声明 (Declaration Pass)。遍历所有函数，只在 LLVM 模块中
    /// 创建它们的签名（函数头），但不生成函数体。这确保了在编译任何函数体时，
    /// 所有其他函数的引用（`FunctionValue`）都是可用的。
    ///
    /// **第二遍**: 实现 (Implementation Pass)。再次遍历所有函数，这次进入函数体
    /// 内部，为它们生成实际的指令。
    ///
    /// # Arguments
    /// * `program` - 指向由 Parser 生成的程序 AST 的引用。
    /// * `analyzer` - 指向已完成分析的语义分析器的引用，用于查询类型信息。
    ///
    /// # Returns
    /// - `Ok(())` 如果整个编译过程成功。
    /// - `Err(CodegenError)` 如果在代码生成期间发生任何错误。
    pub fn compile(
        &mut self,
        program: &Program,
        analyzer: &crate::analyzer::SemanticAnalyzer,
    ) -> Result<(), CodegenError> {
        // --- 第一遍：声明所有函数 ---
        for toplevel_stmt in &program.body {
            if let TopLevelStatement::Function(func_decl) = toplevel_stmt {
                // compile_function_declaration 现在应返回 Result<(), CodegenError>
                self.compile_function_declaration(func_decl, &analyzer.symbol_table)?;
            }
        }
        
        // (可选) 在这里声明所有外部函数，如 C 的 printf
        // self.declare_externs();

        // --- 第二遍：编译所有函数体 ---
        for toplevel_stmt in &program.body {
            if let TopLevelStatement::Function(func_decl) = toplevel_stmt {
                // compile_function_body 现在应返回 Result<(), CodegenError>
                self.compile_function_body(func_decl)?;
            }
        }
        
        Ok(())
    }

    /// **[调试辅助]** 将当前生成的 LLVM IR 打印到标准错误输出。
    ///
    /// 这是一个非常有用的调试工具，可以让你在开发过程中随时查看
    /// 生成的中间代码是否符合预期。
    pub fn print_ir_to_stderr(&self) {
        self.module.print_to_stderr();
    }

    /// 将最终生成的 LLVM IR 保存到指定的文件路径。
    ///
    /// # Arguments
    /// * `path` - 指向目标文件的 `Path`。
    ///
    /// # Returns
    /// - `Ok(())` 如果文件写入成功。
    /// - `Err(CodegenError)` 如果发生 I/O 错误。
    pub fn save_ir_to_file(&self, path: &Path) -> Result<(), CodegenError> {
        self.module.print_to_file(path).map_err(|e| {
            // CHANGED: 将 inkwell 返回的字符串错误包装成我们自己的结构化错误。
            CodegenError::Message(format!("Error writing IR to file: {}", e.to_string()))
        })
    }
    
    // --- 作用域与变量管理 (Scope & Variable Management) ---

    /// 进入一个新的作用域。
    ///
    /// 在代码生成期间，每当遇到会创建新作用域的 AST 节点时（如 BlockStatement），
    /// 就应调用此方法。它会在变量栈 `self.variables` 的顶部推入一个新的空 HashMap。
    fn enter_scope(&mut self) {
        self.variables.push(HashMap::new());
    }

    /// 离开当前作用域。
    ///
    /// 当离开一个代码块时调用此方法，它会弹出变量栈顶的作用域。
    fn leave_scope(&mut self) {
        // 我们不应该弹出唯一的全局作用域
        if self.variables.len() > 1 {
            self.variables.pop();
        }
    }

    /// 从内到外查找一个已在栈上分配了内存的变量。
    ///
    /// # Returns
    /// - `Some(PointerValue)`: 如果找到，返回指向该变量内存位置的指针。
    /// - `None`: 如果在所有可见作用域中都找不到。
    fn lookup_variable(&self, name: &str) -> Option<&(PointerValue<'ctx>, BasicTypeEnum<'ctx>)> {
        for scope in self.variables.iter().rev() {
            if let Some(var_tuple) = scope.get(name) {
                return Some(var_tuple);
            }
        }
        None
    }

    /// 在当前函数的入口块中创建一个 `alloca` 指令，用于在栈上为变量分配内存。
    ///
    /// 这是一个重要的 LLVM 优化实践。将所有 `alloca` 指令放在函数入口块
    /// 的最顶部，可以极大地帮助 LLVM 的 `mem2reg` (Memory to Register)
    /// 优化通道将栈上的变量提升为 SSA 寄存器，从而显著提高性能。
    ///
    /// # Arguments
    /// * `llvm_type` - 要分配的变量的 LLVM 类型。
    /// * `name` - 变量名，用于在生成的 LLVM IR 中添加调试信息。
    ///
    /// # Returns
    /// - `Ok(PointerValue)`: 指向新分配的栈内存的指针。
    /// - `Err(CodegenError)`: 如果当前不在一个函数上下文中。
    fn create_entry_block_alloca(
        &self,
        llvm_type: BasicTypeEnum<'ctx>,
        name: &str,
    ) -> Result<PointerValue<'ctx>, CodegenError> {
        let function = self.current_function.ok_or(CodegenError::Message(
            "Cannot create alloca: not in a function context.".to_string(),
        ))?;
        
        // 创建一个临时的 builder，专门用于在函数入口块的开头插入 alloca
        let builder = self.context.create_builder();
        let entry_block = function.get_first_basic_block().unwrap();

        // 将 builder 定位到入口块的第一条指令之前
        match entry_block.get_first_instruction() {
            Some(first_instr) => builder.position_before(&first_instr),
            None => builder.position_at_end(entry_block),
        }

        // 创建 alloca 指令
        Ok(builder.build_alloca(llvm_type, name)?)
    }

    // --- 类型转换 (Type Conversion) ---

    /// 将 Tipy 的内部类型 (`TipyType`) 转换为 `inkwell` 的基础 LLVM 类型 (`BasicTypeEnum`)。
    ///
    /// 这是连接我们的类型系统和 LLVM 类型系统的核心桥梁。
    /// 注意：此函数不处理 `Void` 或 `Function` 类型，因为它们不是“基础类型”。
    fn to_llvm_basic_type(&self, tipy_type: &TipyType) -> BasicTypeEnum<'ctx> {
        match tipy_type {
            TipyType::I8 => self.context.i8_type().as_basic_type_enum(),
            TipyType::I16 => self.context.i16_type().as_basic_type_enum(),
            TipyType::I32 => self.context.i32_type().as_basic_type_enum(),
            TipyType::I64 => self.context.i64_type().as_basic_type_enum(),
            // ... 其他整数类型 ...
            TipyType::F32 => self.context.f32_type().as_basic_type_enum(),
            TipyType::F64 => self.context.f64_type().as_basic_type_enum(),
            TipyType::Bool => self.context.bool_type().as_basic_type_enum(),
            // 对于指针类型，我们统一使用泛型指针
            TipyType::Pointer { .. } => self.context.i8_type().ptr_type(AddressSpace::default()).as_basic_type_enum(),
            // 其他类型...
            _ => unimplemented!("LLVM type conversion for {:?} is not implemented.", tipy_type),
        }
    }

    // --- 两遍式编译核心 (Two-Pass Compilation Core) ---

    /// **[第一遍]** 声明一个函数的签名，但不编译其函数体。
    ///
    /// 此函数从语义分析器的符号表中获取函数的类型信息 (`TipyType::Function`)，
    /// 将其转换为 LLVM 的函数类型 (`FunctionType`)，然后在当前 `Module` 中
    /// 声明该函数。这确保了在第二遍编译任何函数体之前，所有函数的
    /// `FunctionValue` 都是可用的，从而可以正确处理函数间的相互调用。
    fn compile_function_declaration(
        &self,
        func_decl: &FunctionDeclaration,
        symbol_table: &crate::scope::SymbolTable,
    ) -> Result<(), CodegenError> {
        let func_symbol = symbol_table.lookup(&func_decl.name).ok_or_else(|| {
            // 这通常不应该发生，因为 analyzer 应该已经确保了函数存在
            CodegenError::SymbolNotFound(func_decl.name.clone())
        })?;

        if let TipyType::Function { params, ret } = &func_symbol.symbol_type {
            // 将 Tipy 的参数类型列表转换为 LLVM 的类型列表
            let param_types: Vec<BasicTypeEnum<'ctx>> = params
                .iter()
                .map(|p_type| self.to_llvm_basic_type(p_type))
                .collect();
            
            // inkwell 需要一个 `BasicMetadataTypeEnum` 的Vec数组
            let param_types_as_metadata: Vec<inkwell::types::BasicMetadataTypeEnum<'ctx>> =
                param_types.iter().map(|&t| t.into()).collect();

            // 根据 Tipy 的返回类型，创建 LLVM 的函数类型
            let fn_type = if **ret == TipyType::Void {
                self.context.void_type().fn_type(&param_types_as_metadata, false)
            } else {
                self.to_llvm_basic_type(ret).fn_type(&param_types_as_metadata, false)
            };
            
            // 在模块中添加函数声明
            self.module.add_function(&func_decl.name, fn_type, None);

            Ok(())
        } else {
            Err(CodegenError::Message(format!(
                "Internal Error: Symbol '{}' was expected to be a function, but was not.",
                func_decl.name
            )))
        }
    }

    /// **[第二遍]** 编译一个函数的函数体。
    ///
    /// 此函数为已声明的函数生成实际的 LLVM IR 指令。
    ///
    /// # 执行流程
    /// 1. 获取已声明的 `FunctionValue` 并创建入口基本块 `entry`。
    /// 2. 为所有函数参数在栈上分配空间 (`alloca`)，并将传入的参数值存入其中。
    /// 3. 将参数的 `PointerValue` 注册到代码生成器的变量表中。
    /// 4. 递归地调用 `compile_block_statement` 来编译函数体内的所有语句。
    /// 5. 检查函数是否被正确地“终结”（例如，有 `ret` 指令），如果没有，则为其添加一个隐式的返回。
    fn compile_function_body(
        &mut self,
        func_decl: &FunctionDeclaration
    ) -> Result<(), CodegenError> {
        // CHANGED: 移除 unwrap()，使用安全的错误处理
        let function = self.module.get_function(&func_decl.name).ok_or_else(||
            CodegenError::SymbolNotFound(func_decl.name.clone())
        )?;
        self.current_function = Some(function);
        
        // 创建函数入口块并定位 builder
        let entry_block = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry_block);
        
        // 进入函数，创建新的作用域
        self.enter_scope();

        // 为所有参数在函数入口的栈帧上分配空间
        for (i, param) in function.get_param_iter().enumerate() {
            let arg_name = &func_decl.params[i].name;
            param.set_name(arg_name); // 给 LLVM IR 中的参数命名，方便调试
            
            let arg_type = param.get_type();
            let alloca = self.create_entry_block_alloca(arg_type, arg_name)?;
            
            // 将参数的初始值存入栈中
            self.builder.build_store(alloca, param)?;
            
            // 在 codegen 的变量表中注册这个局部变量（参数）
            // .last_mut().unwrap() 是安全的，因为我们总是有全局作用域
            self.variables.last_mut().unwrap().insert(arg_name.clone(), (alloca, arg_type));
        }

        // 编译函数体
        // 我们将在下一步实现 compile_block_statement
        self.compile_block_statement(&func_decl.body)?;

        // 检查函数是否在所有路径上都有返回
        if function.get_last_basic_block().and_then(|bb| bb.get_terminator()).is_none() {
            // 如果函数是 void 返回，且最后没有 ret，我们隐式添加一个
            if function.get_type().get_return_type().is_none() {
                self.builder.build_return(None)?;
            } else if func_decl.name == "main" {
                // 特殊处理 main 函数，使其默认返回 0
                let i32_type = self.context.i32_type();
                self.builder.build_return(Some(&i32_type.const_int(0, false)))?;
            } else {
                // 如果一个非 void 函数没有在所有代码路径上返回值，这是一个错误
                return Err(CodegenError::Message(format!(
                    "Function '{}' must return a value on all code paths.",
                    func_decl.name
                )));
            }
        }
        
        // 离开函数作用域
        self.leave_scope();
        self.current_function = None; // 清理状态

        Ok(())
    }
    
    // --- 语句与块编译 (Statement & Block Compilation) ---

    /// 编译一个语句 AST 节点。
    ///
    /// 这是语句编译的“调度中心”，它根据语句的类型，
    /// 调用相应的、更具体的 `compile_` 函数。
    /// 对于作为语句使用的表达式，其计算结果将被丢弃。
    fn compile_statement(&mut self, stmt: &Statement) -> Result<(), CodegenError> {
        match stmt {
            Statement::VarDeclaration(var_decl) => self.compile_var_declaration(var_decl),
            Statement::Return(ret_stmt) => {
                let ret_val = match &ret_stmt.value {
                    Some(expr) => Some(self.compile_expression(expr)?), // 编译表达式
                    None => None, // void 返回
                };
                
                // .as_ref().map(...) 是处理 Option<T> 到 Option<&T> 的标准方法
                self.builder.build_return(ret_val.as_ref().map(|v| v as &dyn inkwell::values::BasicValue))?;
                Ok(())
            }
            Statement::Expression(expr) => {
                // 表达式作为语句使用时，我们只关心它的编译过程（及其副作用，如函数调用），
                // 而不关心其返回值。
                self.compile_expression(expr).map(|_| ())
            }
            Statement::Block(block_stmt) => {
                // 代码块作为语句使用时，我们同样不关心其返回值。
                self.compile_block_statement(block_stmt).map(|_| ())
            }
            Statement::While(while_stmt) => self.compile_while_statement(while_stmt),
            Statement::Break(break_stmt) => self.compile_break_statement(break_stmt),
            Statement::Continue(cont_stmt) => self.compile_continue_statement(cont_stmt),
        }
    }

    /// 编译一个代码块 `{...}`，并返回该块作为表达式时的值。
    ///
    /// # 主要职责
    /// 1. 创建并进入一个新的变量作用域。
    /// 2. 编译块内的每一条语句。
    /// 3. 根据 Tipy 规则，如果最后一个语句是表达式，则该表达式的值成为整个块的值。
    /// 4. 离开作用域。
    ///
    /// # Returns
    /// - `Ok(Some(BasicValueEnum))` 如果块以表达式结尾。
    /// - `Ok(None)` 如果块为空，或以非表达式语句结尾。
    fn compile_block_statement(
        &mut self,
        block: &BlockStatement,
    ) -> Result<Option<BasicValueEnum<'ctx>>, CodegenError> {
        self.enter_scope();
        let mut last_val = None;

        // 编译除最后一条语句外的所有语句
        if let Some((last_stmt, other_stmts)) = block.statements.split_last() {
            for stmt in other_stmts {
                self.compile_statement(stmt)?;
            }
            
            // 特别处理最后一个语句
            if let Statement::Expression(expr) = last_stmt {
                // 如果是表达式，它的值就是块的返回值
                last_val = Some(self.compile_expression(expr)?);
            } else {
                // 如果是其他语句，正常编译，块没有返回值
                self.compile_statement(last_stmt)?;
            }
        }
        // 如果块为空，last_val 保持为 None

        self.leave_scope();
        Ok(last_val)
    }

    // --- 具体语句编译 (Specific Statement Compilation) ---

    /// 编译一个变量声明语句 `name: [~]type [= value];`
    fn compile_var_declaration(&mut self, var_decl: &VarDeclaration) -> Result<(), CodegenError> {
        // 从符号表或分析器获取变量的 Tipy 类型
        // (这里我们假设可以通过某种方式获取，或直接从 AST 解析)
        let var_tipy_type = TipyType::I32; // 简化：应从 analyzer 获取
        let var_llvm_type = self.to_llvm_basic_type(&var_tipy_type);

        // 在当前函数的入口块为变量分配栈空间
        let alloca = self.create_entry_block_alloca(var_llvm_type, &var_decl.name)?;

        // 如果有初始值，编译它并存入分配好的空间
        if let Some(initial_value) = &var_decl.value {
            let compiled_value = self.compile_expression(initial_value)?;
            self.builder.build_store(alloca, compiled_value)?;
        }

        // 在当前的 codegen 作用域中注册这个变量的指针
        self.variables
            .last_mut()
            .unwrap()
            .insert(var_decl.name.clone(), (alloca, var_llvm_type));
            
        Ok(())
    }

    /// 编译 `while` 循环语句。
    fn compile_while_statement(&mut self, while_stmt: &WhileStatement) -> Result<(), CodegenError> {
        let function = self.current_function.ok_or(CodegenError::Message(
            "Cannot compile while loop: not in a function context.".to_string(),
        ))?;

        // 创建 `while` 循环需要的三个基本块
        let cond_block = self.context.append_basic_block(function, "while.cond");
        let loop_block = self.context.append_basic_block(function, "while.body");
        let after_block = self.context.append_basic_block(function, "while.after");

        // 将循环的上下文（继续点和退出点）压入栈中
        self.loop_context_stack.push((cond_block, after_block, None));

        // 1. 无条件跳转到条件检查块
        self.builder.build_unconditional_branch(cond_block)?;

        // 2. 编译条件检查块
        self.builder.position_at_end(cond_block);
        let condition = self.compile_expression(&while_stmt.condition)?;
        let bool_cond = condition.into_int_value();
        // 根据条件结果，决定是进入循环体还是跳出循环
        self.builder.build_conditional_branch(bool_cond, loop_block, after_block)?;

        // 3. 编译循环体块
        self.builder.position_at_end(loop_block);
        self.compile_block_statement(&while_stmt.body)?;
        // 循环体结束后，无条件跳回条件检查块
        self.builder.build_unconditional_branch(cond_block)?;

        // 4. 将 builder 定位到循环结束后的块，以继续生成后续代码
        self.builder.position_at_end(after_block);
        
        // 离开循环，弹出上下文
        self.loop_context_stack.pop();

        Ok(())
    }

    /// 编译 `break` 语句。
    fn compile_break_statement(&mut self, break_stmt: &BreakStatement) -> Result<(), CodegenError> {
        // FIXED: 在模式匹配时使用 `&`，可以将元组内的所有 Copy 类型的值拷贝出来，
        // 而不是持有对 self.loop_context_stack 的引用。这就立即结束了不可变借用。
        if let Some(&(_, exit_block, result_alloca)) = self.loop_context_stack.last() {
            // 到这里，对 self 的不可变借用已经结束，我们可以安全地可变借用 self。
            if let Some(expr) = &break_stmt.value {
                if let Some(alloca) = result_alloca {
                    // 现在这里调用 self.compile_expression 是安全的
                    let value = self.compile_expression(expr)?;
                    self.builder.build_store(alloca, value)?;
                } else {
                    return Err(CodegenError::Message(
                        "'break' with a value is not allowed in this loop.".to_string(),
                    ));
                }
            }
            self.builder.build_unconditional_branch(exit_block)?;
            Ok(())
        } else {
            Err(CodegenError::Message(
                "'break' used outside of a loop.".to_string(),
            ))
        }
    }

    /// 编译 `continue` 语句。
    fn compile_continue_statement(&mut self, _cont_stmt: &ContinueStatement) -> Result<(), CodegenError> {
        // 从循环上下文栈顶获取继续点
        let continue_block = self.loop_context_stack.last().map(|&(cont, _, _)| cont).ok_or(
            CodegenError::Message("'continue' used outside of a loop.".to_string())
        )?;
        self.builder.build_unconditional_branch(continue_block)?;
        Ok(())
    }

    // --- 表达式编译 (Expression Compilation) ---

    /// 编译一个表达式 AST 节点，并返回其对应的 LLVM 值 (`BasicValueEnum`)。
    ///
    /// 这是代码生成中最重要的递归函数。它作为“调度中心”，根据表达式的类型，
    /// 调用相应的、更具体的 `compile_...` 辅助函数。
    fn compile_expression(
        &mut self,
        expr: &Expression,
    ) -> Result<BasicValueEnum<'ctx>, CodegenError> {
        match expr {
            Expression::Literal(lit) => self.compile_literal(lit),
            Expression::Identifier(name) => self.compile_identifier(name),
            Expression::Prefix(prefix_expr) => self.compile_prefix_expression(prefix_expr),
            Expression::Infix(infix_expr) => self.compile_infix_expression(infix_expr),
            Expression::Assignment(assign_expr) => self.compile_assignment_expression(assign_expr),
            Expression::Call(call_expr) => self.compile_call_expression(call_expr),
            Expression::If(if_expr) => self.compile_if_expression(if_expr),
            Expression::Loop(loop_expr) => self.compile_loop_expression(loop_expr),
            Expression::Block(block_stmt) => self
                .compile_block_statement(block_stmt)?
                .ok_or_else(|| CodegenError::Message(
                    "A block used as an expression must return a value.".to_string()
                )),
        }
    }

    // --- 表达式编译辅助函数 (Expression Compilation Helpers) ---

    /// 编译字面量
    fn compile_literal(&self, lit: &Literal) -> Result<BasicValueEnum<'ctx>, CodegenError> {
        match lit {
            Literal::Integer(val) => Ok(self.context.i64_type().const_int(*val as u64, true).into()),
            Literal::Float(val) => Ok(self.context.f64_type().const_float(*val).into()),
            Literal::Boolean(val) => Ok(self.context.bool_type().const_int(*val as u64, false).into()),
            // 其他字面量...
            _ => Err(CodegenError::Message("This literal type is not yet supported in codegen.".to_string())),
        }
    }

    /// 编译标识符（变量读取）
    fn compile_identifier(&self, name: &str) -> Result<BasicValueEnum<'ctx>, CodegenError> {
        let (ptr, var_type) = self.lookup_variable(name).ok_or_else(|| CodegenError::SymbolNotFound(name.to_string()))?;
        // 从变量在栈上的地址（指针）加载其值
        Ok(self.builder.build_load(*var_type, *ptr, name)?)
    }

    /// 编译前缀表达式
    fn compile_prefix_expression(&mut self, prefix_expr: &crate::ast::PrefixExpression) -> Result<BasicValueEnum<'ctx>, CodegenError> {
        let value = self.compile_expression(&prefix_expr.right)?;
        match prefix_expr.op {
            crate::ast::PrefixOperator::Minus => {
                if value.is_int_value() {
                    Ok(self.builder.build_int_neg(value.into_int_value(), "neg")?.into())
                } else if value.is_float_value() {
                    Ok(self.builder.build_float_neg(value.into_float_value(), "fneg")?.into())
                } else {
                    Err(CodegenError::Message("Unary minus can only be applied to numbers.".to_string()))
                }
            }
            crate::ast::PrefixOperator::Not => {
                let bool_true = self.context.bool_type().const_int(1, false);
                Ok(self.builder.build_xor(value.into_int_value(), bool_true, "not")?.into())
            }
        }
    }
    
    /// 编译中缀表达式
    fn compile_infix_expression(&mut self, infix_expr: &crate::ast::InfixExpression) -> Result<BasicValueEnum<'ctx>, CodegenError> {
        // NEW: 引入 use 语句，简化后续代码
        
        let left = self.compile_expression(&infix_expr.left)?;
        let right = self.compile_expression(&infix_expr.right)?;

        if left.is_int_value() && right.is_int_value() {
            let l = left.into_int_value();
            let r = right.into_int_value();
            match infix_expr.op {
                // --- 算术运算 ---
                Operator::Plus => Ok(self.builder.build_int_add(l, r, "add")?.into()),
                Operator::Minus => Ok(self.builder.build_int_sub(l, r, "sub")?.into()),
                Operator::Multiply => Ok(self.builder.build_int_mul(l, r, "mul")?.into()),
                Operator::Divide => Ok(self.builder.build_int_signed_div(l, r, "div")?.into()),
                // --- 比较运算 ---
                Operator::Equal => Ok(self.builder.build_int_compare(IntPredicate::EQ, l, r, "eq")?.into()),
                Operator::NotEqual => Ok(self.builder.build_int_compare(IntPredicate::NE, l, r, "ne")?.into()),
                Operator::LessThan => Ok(self.builder.build_int_compare(IntPredicate::SLT, l, r, "lt")?.into()),
                Operator::LessEqual => Ok(self.builder.build_int_compare(IntPredicate::SLE, l, r, "le")?.into()),
                Operator::GreaterThan => Ok(self.builder.build_int_compare(IntPredicate::SGT, l, r, "gt")?.into()),
                Operator::GreaterEqual => Ok(self.builder.build_int_compare(IntPredicate::SGE, l, r, "ge")?.into()),
            }
        } else if left.is_float_value() && right.is_float_value() {
            let l = left.into_float_value();
            let r = right.into_float_value();
            match infix_expr.op {
                // --- 算术运算 ---
                Operator::Plus => Ok(self.builder.build_float_add(l, r, "fadd")?.into()),
                Operator::Minus => Ok(self.builder.build_float_sub(l, r, "fsub")?.into()),
                Operator::Multiply => Ok(self.builder.build_float_mul(l, r, "fmul")?.into()),
                Operator::Divide => Ok(self.builder.build_float_div(l, r, "fdiv")?.into()),
                // --- 比较运算 (O for Ordered, U for Unordered) ---
                Operator::Equal => Ok(self.builder.build_float_compare(FloatPredicate::OEQ, l, r, "feq")?.into()),
                Operator::NotEqual => Ok(self.builder.build_float_compare(FloatPredicate::ONE, l, r, "fne")?.into()),
                Operator::LessThan => Ok(self.builder.build_float_compare(FloatPredicate::OLT, l, r, "flt")?.into()),
                Operator::LessEqual => Ok(self.builder.build_float_compare(FloatPredicate::OLE, l, r, "fle")?.into()),
                Operator::GreaterThan => Ok(self.builder.build_float_compare(FloatPredicate::OGT, l, r, "fgt")?.into()),
                Operator::GreaterEqual => Ok(self.builder.build_float_compare(FloatPredicate::OGE, l, r, "fge")?.into()),
            }
        } else {
            Err(CodegenError::Message("Mismatched or unsupported types in binary operation.".to_string()))
        }
    }
    
    /// 编译赋值表达式
    fn compile_assignment_expression(&mut self, assign_expr: &crate::ast::AssignmentExpression) -> Result<BasicValueEnum<'ctx>, CodegenError> {
        let compiled_value = self.compile_expression(&assign_expr.value)?;

        // `compile_lvalue` 是一个新的辅助函数，它返回一个指针，而不是值
        let ptr = self.compile_lvalue_expression(&assign_expr.left)?;
        
        self.builder.build_store(ptr, compiled_value)?;
        // 赋值表达式的值就是被赋的值
        Ok(compiled_value)
    }

    /// 编译一个“左值”表达式，返回其内存地址（指针）
    fn compile_lvalue_expression(&mut self, expr: &Expression) -> Result<PointerValue<'ctx>, CodegenError> {
        match expr {
            Expression::Identifier(name) => {
                self.lookup_variable(name).map(|(ptr, _)| *ptr).ok_or_else(|| CodegenError::SymbolNotFound(name.clone()))
            }
            // TODO: 支持更复杂的左值，如 `a.b` 或 `*p`
            _ => Err(CodegenError::InvalidLValue),
        }
    }
    
    /// 编译函数调用
    fn compile_call_expression(&mut self, call_expr: &crate::ast::CallExpression) -> Result<BasicValueEnum<'ctx>, CodegenError> {
        // 我们假设 callee 是一个简单的标识符
        let callee_name = if let Expression::Identifier(name) = &*call_expr.function {
            name
        } else {
            return Err(CodegenError::Message("Complex function calls are not supported.".to_string()));
        };
        
        let function = self.module.get_function(callee_name).ok_or_else(|| CodegenError::SymbolNotFound(callee_name.clone()))?;

        // --- 将参数编译过程拆分为两步，解决类型推断问题 ---

        // 步骤 1: 编译所有参数表达式，将结果收集到一个 Result<Vec<...>, ...> 中。
        //         通过为 `compiled_values` 标注类型，我们告诉 `collect()` 在成功时需要一个 Vec。
        let compiled_values: Result<Vec<BasicValueEnum<'ctx>>, _> = call_expr
            .arguments
            .iter()
            .map(|arg| self.compile_expression(arg))
            .collect();

        // 步骤 2: 如果上一步成功（通过 `?`），则将 Vec<BasicValueEnum> 转换为 Vec<BasicMetadataValueEnum>。
        //         这是 `build_call` 所需的最终格式。
        let compiled_args: Vec<BasicMetadataValueEnum<'ctx>> = compiled_values?
            .into_iter()
            .map(|val| val.into())
            .collect();
        
        let call_site = self.builder.build_call(function, &compiled_args, "call_tmp")?;

        match call_site.try_as_basic_value().left() {
            Some(value) => Ok(value),
            None => Err(CodegenError::Message("Cannot use a void function as an expression.".to_string())),
        }
    }

    /// 编译 if-else 表达式
    fn compile_if_expression(&mut self, if_expr: &IfExpression) -> Result<BasicValueEnum<'ctx>, CodegenError> {
        let function = self.current_function.unwrap();
        
        let condition = self.compile_expression(&if_expr.condition)?.into_int_value();
        
        let then_block = self.context.append_basic_block(function, "then");
        let else_block = self.context.append_basic_block(function, "else");
        let merge_block = self.context.append_basic_block(function, "merge");
        
        self.builder.build_conditional_branch(condition, then_block, else_block)?;
        
        // --- 编译 then 分支 ---
        self.builder.position_at_end(then_block);
        let then_val = self.compile_block_statement(&if_expr.consequence)?.unwrap(); // 假设 if 作为表达式必须有返回值
        self.builder.build_unconditional_branch(merge_block)?;
        let then_end_block = self.builder.get_insert_block().unwrap();
        
        // --- 编译 else 分支 ---
        self.builder.position_at_end(else_block);
        let else_val = if let Some(alt) = &if_expr.alternative {
             self.compile_expression(alt)?
        } else {
            // 如果 if-else 表达式要返回值，else 分支必须存在
            return Err(CodegenError::Message("'if' expression must have an 'else' branch".to_string()));
        };
        self.builder.build_unconditional_branch(merge_block)?;
        let else_end_block = self.builder.get_insert_block().unwrap();

        // --- 编译 merge (PHI) 块 ---
        self.builder.position_at_end(merge_block);
        let phi = self.builder.build_phi(then_val.get_type(), "iftmp")?;
        phi.add_incoming(&[(&then_val, then_end_block), (&else_val, else_end_block)]);
        
        Ok(phi.as_basic_value())
    }

    /// 编译 `loop` 表达式。
    ///
    /// `loop { ... }` 在 LLVM 中被实现为一个无条件循环。
    /// 1. 创建 `loop_body` 和 `after_loop` 两个基本块。
    /// 2. 将 `(loop_body, after_loop)` 压入循环上下文栈，为 `break` 和 `continue` 提供跳转目标。
    /// 3. 生成一个无条件跳转到 `loop_body`。
    /// 4. 编译循环体内的代码。
    /// 5. 在循环体末尾，生成一个无条件跳转，指回 `loop_body` 的开头，形成无限循环。
    /// 6. 将 builder 定位到 `after_loop`，后续的代码将从这里开始。
    ///
    /// # 关于返回值
    /// `loop` 本身是一个表达式，其类型和值由 `break <value>` 语句决定。
    /// 一个完整的实现需要使用 PHI 节点来合并所有可能的 `break` 的值。
    /// 为简化起见，当前版本假定 `loop` 表达式不返回值 (类型为 Void)。
    fn compile_loop_expression(&mut self, loop_expr: &LoopExpression) -> Result<BasicValueEnum<'ctx>, CodegenError> {
        let function = self.current_function.ok_or_else(|| {
            CodegenError::Message("Cannot compile loop: not in a function context.".to_string())
        })?;

        // --- 核心改动：使用 Alloca 模式 ---
        // TODO: loop 表达式的返回类型应该由语义分析器推断出来。
        //       这里我们暂时硬编码为 i64 作为示例。
        let result_type = self.context.i64_type().as_basic_type_enum();
        let result_alloca = self.create_entry_block_alloca(result_type, "loop_result")?;

        let loop_bb = self.context.append_basic_block(function, "loop.body");
        let after_bb = self.context.append_basic_block(function, "loop.after");

        // 将循环上下文（包括结果指针）压入栈中
        self.loop_context_stack.push((loop_bb, after_bb, Some(result_alloca)));

        // 从当前块跳转到循环体
        self.builder.build_unconditional_branch(loop_bb)?;

        // 编译循环体
        self.builder.position_at_end(loop_bb);
        // 我们不关心循环体的返回值，因为它通过 `break` 传递
        self.compile_block_statement(&loop_expr.body)?;

        // 如果循环体执行完都没有 break 或 return，说明它会无限循环。
        // 我们在这里也需要一个跳转，指回循环开头。
        if loop_bb.get_terminator().is_none() {
            self.builder.build_unconditional_branch(loop_bb)?;
        }
        
        // 离开循环，弹出上下文
        self.loop_context_stack.pop();

        // --- 核心改动：加载最终结果 ---
        // 将 builder 定位到循环结束后的块
        self.builder.position_at_end(after_bb);
        // 从为 loop 结果预留的内存中加载值，这个值就是整个 loop 表达式的值。
        let loop_result = self.builder.build_load(result_type, result_alloca, "loop_val")?;
        
        Ok(loop_result)
    }
}