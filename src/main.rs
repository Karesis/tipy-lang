// src/main.rs

// 引入所有模块
mod token;
mod lexer;
mod ast;
mod parser;
mod types;
mod semantic;
mod analyzer;
mod error;
mod codegen;

use inkwell::context::Context;
use lexer::Lexer;
use parser::Parser;
use analyzer::SemanticAnalyzer;
use codegen::CodeGen;

use std::path::Path;

fn main() {
    // 我们的 Tipy 源代码
    let input = r#"
        add(a: i32, b: i32) -> i32 {
            ret a + b
        }

        main() -> i32 {
            a: i32 = 10;
            b: ~i32 = 20;
            b = add(a, b * 2); // 应计算为 add(10, 40) = 50
            ret b;
        }
    "#;

    println!("--- Compiling Tipy source ---");
    println!("{}\n", input);

    // --- 1. 词法分析 (Lexing) ---
    let lexer = Lexer::new(input);

    // --- 2. 语法分析 (Parsing) ---
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program();

    if !parser.errors.is_empty() {
        eprintln!("Encountered parsing errors:");
        for err in parser.errors {
            eprintln!("- {}", err);
        }
        return;
    }
    println!("--- AST ---");
    println!("{:#?}\n", program);

    // --- 3. 语义分析 (Semantic Analysis) ---
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(&program);

    if !analyzer.errors.is_empty() {
        eprintln!("Encountered semantic errors:");
        for err in analyzer.errors {
            eprintln!("- {}", err);
        }
        return;
    }
    println!("--- Semantic Analysis Successful ---\n");


    // --- 4. 代码生成 (Code Generation) ---
    let context = Context::create();
    let mut codegen = CodeGen::new(&context, "tipy_module");
    
    // 将 Program (AST) 和 Analyzer (类型信息) 一起传入
    match codegen.compile(&program, &analyzer) {
        Ok(()) => {
            println!("--- Compilation Successful ---");
            // 打印生成的 LLVM IR 到控制台，方便调试
            codegen.print_ir_to_stderr();

            // 将 IR 保存到文件
            let output_path = Path::new("output.ll");
            if let Err(e) = codegen.save_ir_to_file(output_path) {
                eprintln!("Error saving IR to file: {}", e);
            } else {
                println!("\nIR saved to output.ll");
                println!("Run the following commands to create an executable:");
                println!("  llc-18 -filetype=obj -relocation-model=pic -o output.o output.ll");
                println!("  clang output.o -o my_program");
                println!("  ./my_program");
                println!("  echo $?  # Should print 50 on Linux/macOS");
            }
        },
        Err(e) => {
            eprintln!("\nError during code generation: {}", e);
        }
    }
}