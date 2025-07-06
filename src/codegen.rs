use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::values::FunctionValue;

use crate::ast::{Program, Statement, Expression};
use crate::token::Literal;
use crate::error::CompileError;
use std::collections::HashMap;

pub struct CodeGen<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,

    functions: HashMap<String, FunctionValue<'ctx>>,
}

impl<'ctx> CodeGen<'ctx> {
    pub fn new(context: &'ctx Context) -> Self {
        let module = context.create_module("tipy_module");
        let builder = context.create_builder();

        CodeGen {
            context,
            module,
            builder,
            functions: HashMap::new(),
        }
    }

    pub fn compile(&mut self, program: &Program) -> Result<(), CompileError> {
        self.declare_externs();

        for statement in &program.statements {
            self.compile_statement(statement)?;
        }
        Ok(())
    }

    pub fn print_ir(&self) {
        println!("--- LLVM IR ---");
        self.module.print_to_stderr();
        println!("---------------");
    }

    pub fn save_ir_to_file(&self, path: &std::path::Path) -> Result<(), &'static str> {
        self.module
            .print_to_file(path)
            .map_err(|_| "Error writing IR to file")
    }

    fn declare_externs(&mut self) {
        // 对于 v0.0.1，我们只需要 C 标准库中的 `puts` 函数
        // `puts` 接收一个 `char*` 指针，返回一个 `i32`。
        // `print("...")` 在底层将被我们编译为 `puts("...")`

        // 1. 定义 `i32` 和 `i8*` 类型
        let i32_type = self.context.i32_type();
        let i8_ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());

        // 2. 创建 puts 的函数类型 `fn(i8*) -> i32`
        let puts_fn_type = i32_type.fn_type(&[i8_ptr_type.into()], false);

        // 3. 在模块中添加这个函数声明
        let puts_fn = self.module.add_function("puts", puts_fn_type, None);

        // 4. 将我们语言中的 `print` 映射到 `puts`
        self.functions.insert("print".to_string(), puts_fn);
    }

    fn compile_statement(&mut self, statement: &Statement) -> Result<(), CompileError> {
        match statement {
            Statement::Function { name, body } => {
                // TODO: 调用函数编译的逻辑
                self.compile_function(name, body)
            },
            Statement::Expression(expr) => {
                // TODO: 调用表达式编译的逻辑
                self.compile_expression(expr).map(|_| ()) // 丢弃表达式的值
            },
        }
    }

    fn compile_function(&mut self, name: &Expression, body: &[Statement]) -> Result<(), CompileError> {
        // 1. 从 AST 节点中提取函数名
        let fn_name = match name {
            Expression::Identifier(name_str) => name_str,
            _ => return Err(CompileError::Semantic("Expected function name to be an identifier.".to_string())),
        };
    
        // 2. 检查函数是否已被定义
        if self.functions.contains_key(fn_name) {
            return Err(CompileError::Semantic("Function cannot be redefined.".to_string()));
        }
    
        // 3. 为我们的 `main` 函数创建函数类型 `fn() -> ()`
        let function = if fn_name == "main" {
            // main 函数返回 i32
            let i32_type = self.context.i32_type();
            let fn_type = i32_type.fn_type(&[], false);
            self.module.add_function(fn_name, fn_type, None)
        } else {
            // 其他函数暂时返回 void
            let void_type = self.context.void_type();
            let fn_type = void_type.fn_type(&[], false);
            self.module.add_function(fn_name, fn_type, None)
        };
        
        // 5. 创建函数体的入口基本块 (BasicBlock)
        let entry_block = self.context.append_basic_block(function, "entry");
    
        // 6. 将 builder 定位到入口块的末尾，准备开始写入指令
        self.builder.position_at_end(entry_block);
    
        // 7. 将函数存入我们的符号表，以备后续调用
        self.functions.insert(fn_name.clone(), function);
    
        // 8. 编译函数体内的所有语句
        for stmt in body {
            self.compile_statement(stmt)?;
        }

        if fn_name == "main" {
            // main 函数返回 0
            let i32_type = self.context.i32_type();
            self.builder.build_return(Some(&i32_type.const_int(0, false)))?;
        } else {
            // 其他函数返回 void
            self.builder.build_return(None)?;
        }
    
        Ok(())
    }

    fn compile_expression(&mut self, expr: &Expression) -> Result<inkwell::values::BasicValueEnum<'ctx>, CompileError> {
        match expr {
            // 处理字符串字面量
            Expression::Literal(Literal::String(s)) => {
                // 在模块中创建一个全局字符串，并返回一个指向它的指针
                Ok(self.builder
                    .build_global_string_ptr(s, ".str")?
                    .as_pointer_value()
                    .into())
            },
    
            // 在 compile_expression 的 match 语句中

            Expression::Call { function, arguments } => {
                // 步骤 1：使用 map + collect，一次性处理完所有参数的编译。
                // 这就像是把所有零件的蓝图先画好，放进一个叫 `compiled_args` 的 Vec 里。
                let compiled_args: Vec<_> = arguments.iter()
                    .map(|arg| self.compile_expression(arg)) // 对每个参数应用编译函数
                    .collect::<Result<Vec<_>, _>>()?; // 将所有 Result 收集成一个 Result<Vec<...>>，然后用 ? 解包
            
                // 步骤 2：现在安全了，我们可以处理函数名了。
                let callee_name: &str = match &**function {
                    Expression::Identifier(name) => name,
                    _ => return Err(CompileError::Semantic("Function call target must be a simple identifier.".to_string())),
                };
                
                let function_value = self.functions.get(callee_name)
                    .ok_or_else(|| CompileError::Semantic(format!("Call to undefined function: `{}`", callee_name)))?;
            
                // 步骤 3：组装。
                let args_as_metadata: Vec<inkwell::values::BasicMetadataValueEnum> = compiled_args.iter()
                    .map(|val| (*val).into())
                    .collect();
            
                let call_site = self.builder.build_call(*function_value, &args_as_metadata, "tmp_call")?;
            
                Ok(call_site.try_as_basic_value().left().unwrap())
            },
            
            // 其他表达式类型暂不支持
            _ => Err(CompileError::Semantic("Unsupported expression type for compilation.".to_string())),
        }
    }
}