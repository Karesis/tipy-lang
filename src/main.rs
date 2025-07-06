// In src/main.rs

mod token;
mod lexer;
mod ast;
mod parser;
mod codegen;
mod error;

use inkwell::context::Context;
use lexer::Lexer;
use parser::Parser;
use codegen::CodeGen;

fn main() {
    let input = r#"
    main() {
        print("Hello, Tipy World!")
    }
    "#;

    // --- 前端 ---
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program();
    println!("--- AST ---");
    println!("{:#?}\n", program);

    // --- 后端 ---
    let context = Context::create();
    let mut codegen = CodeGen::new(&context);
    
    match codegen.compile(&program) {
        Ok(()) => {
            // 编译成功，打印生成的 LLVM IR
            codegen.print_ir();
        },
        Err(e) => {
            eprintln!("Error during compilation: {}", e);
        }
    }
}