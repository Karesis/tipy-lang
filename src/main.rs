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

use std::path::Path;

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
            println!("Compilation successful. IR output:");
            codegen.print_ir(); // 我们仍然打印出来方便调试

            // 将 IR 保存到文件
            let output_path = Path::new("output.ll");
            if let Err(e) = codegen.save_ir_to_file(output_path) {
                eprintln!("Error saving IR to file: {}", e);
            } else {
                println!("IR saved to output.ll");
                println!("Run the following commands to create an executable:");
                println!("  llc-18 -filetype=obj -relocation-model=pic -o output.o output.ll");
                println!("  clang output.o -o my_program");
                println!("  ./my_program");
            }
        },

        Err(e) => {
            eprintln!("Error during compilation: {}", e);
        }
    }
}