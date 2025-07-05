use std::collections::HashMap;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::values::{FunctionValue, BasicValueEnum};
use inkwell::AddressSpace;
use crate::ast;

pub struct CodeGen<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    functions: HashMap<String, FunctionValue<'ctx>>,
}

impl<'ctx> CodeGen<'ctx> {
    pub fn new(context: &'ctx Context, module_name: &str) -> Self {
        let module = context.create_module(module_name);
        let builder = context.create_builder();
        CodeGen {
            context,
            module,
            builder,
            functions: HashMap::new(),
        }
    }

    pub fn compile(&mut self, program: ast::Program) -> Result<String, String> {
        self.declare_printf();
        for func in program {
            self.compile_function(func)?;
        }
        Ok(self.module.print_to_string().to_string())
    }

    fn declare_printf(&mut self) {
        let i32_type = self.context.i32_type();
        let i8_ptr_type = self.context.ptr_type(AddressSpace::default());
        let printf_type = i32_type.fn_type(&[i8_ptr_type.into()], true);
        let function = self.module.add_function("printf", printf_type, None);
        self.functions.insert("printf".to_string(), function);
    }

    fn compile_function(&mut self, func: ast::Function) -> Result<(), String> {
        let function = {
            if func.name != "main" {
                return Err(format!("未知的函数: {}", func.name));
            }

            let i32_type = self.context.i32_type();
            let fn_type = i32_type.fn_type(&[], false);
            self.module.add_function(&func.name, fn_type, None)
        };

        let basic_block = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(basic_block);

        for expr in func.body {
            self.compile_expression(expr)?;
        }

        self.builder.build_return(Some(&self.context.i32_type().const_int(0, false))).unwrap();
        self.functions.insert(func.name.clone(), function);
        Ok(())
    }

    fn compile_expression(&mut self, expr: ast::Expression) -> Result<BasicValueEnum<'ctx>, String> {
        match expr {
            // 【已修正】用 callee@ _ 来忽略未使用的变量
            ast::Expression::Call { callee, args } => {
                if callee != "print" {
                    return Err(format!("不支持的函数调用: {}", callee));
                }
                
                let printf = *self.functions.get("printf").ok_or("printf 函数未声明")?;

                let mut compiled_args = Vec::new();
                for arg in args {
                    let val = self.compile_expression(arg)?;
                    compiled_args.push(val.into());
                }

                let call = self.builder.build_call(printf, &compiled_args, "printf_call").unwrap();
                Ok(call.try_as_basic_value().left().unwrap())
            }
            ast::Expression::StringLiteral(s) => {
                let ptr = self.builder.build_global_string_ptr(&s, ".str").unwrap();
                Ok(ptr.as_pointer_value().into())
            }
        }
    }
}