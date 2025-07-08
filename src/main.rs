// src/main.rs

// --- 模块声明 ---
// 声明编译器项目的所有模块
mod token;
mod lexer;
mod ast;
mod parser;
mod types;
mod scope;
mod analyzer;
mod codegen;
mod diagnostics;

// --- 模块引入 ---
use inkwell::context::Context;
use lexer::Lexer;
use parser::Parser;
use analyzer::SemanticAnalyzer;
use codegen::CodeGen;
use std::path::Path;

/// Tipy 编译器的主入口函数。
fn main() {
    // --- 源代码输入 ---
    // UPDATED: 一个更全面的测试用例，用于测试 v0.0.5 的所有核心功能，
    // 包括 if-else 表达式和能返回值的 loop 表达式。
    let input = r#"
// 一个使用 if-else 表达式的函数
max(a: i64, b: i64) -> i64 {
    if a > b {
        a // if 块的隐式返回
    } else {
        b // else 块的隐式返回
    }
}

// 一个演示 loop 表达式返回值的函数
count_to_ten_and_double() -> i64 {
    counter: ~i64 = 0;
    
    // loop 是一个表达式，它的值由第一个执行的 `break <value>` 决定
    result: i64 = loop {
        counter = counter + 1;
        if counter == 10 {
            break counter * 2; // 循环将在此处中断，并返回值 20
        }
    };

    result // 函数隐式返回 result (20)
}

// 主函数，程序的入口点
main() -> i64 {
    // 测试 if-else 表达式，max_val 应为 100
    max_val: i64 = max(100, 50);

    // 测试 loop 表达式，loop_val 应为 20
    loop_val: i64 = count_to_ten_and_double();
    
    // 最终结果应为 100 + 20 = 120
    ret max_val + loop_val;
}
    "#;

    println!("--- Compiling Tipy source ---");
    println!("{}\n", input);

    // --- 1. 词法分析 (Lexing) ---
    // 词法分析器将源代码字符串转换为 Token 流。
    // 我们的新 Lexer 在遇到词法错误时，会由 Parser 在 next_token() 中捕获。
    let lexer = Lexer::new(input);

    // --- 2. 语法分析 (Parsing) ---
    // 解析器消耗 Token 流，并构建抽象语法树 (AST)。
    // 我们的新 Parser 具备错误恢复能力，并会将所有词法和语法错误收集起来。
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program();

    // 检查在前端（词法和语法）阶段是否收集到了错误。
    if !parser.errors.is_empty() {
        eprintln!("Encountered Parsing or Lexing errors:");
        for err in parser.errors {
            // 我们统一的 CompilerError 现在可以被优雅地打印出来。
            eprintln!("- {}", err);
        }
        return;
    }
    println!("--- AST ---");
    println!("{:#?}\n", program);

    // --- 3. 语义分析 (Semantic Analysis) ---
    // 语义分析器遍历 AST，进行类型检查和作用域分析。
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
    // 代码生成器将验证通过的 AST 转换为 LLVM IR。
    let context = Context::create();
    let mut codegen = CodeGen::new(&context, "tipy_module");
    
    // 将 Program (AST) 和 Analyzer (用于查询类型信息) 一起传入
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
                // 注意：请确保你的系统上安装了与 inkwell 匹配的 llc 和 clang 版本
                // 例如，对于 inkwell 0.4.0，通常需要 LLVM 15, 16, 17 或 18
                println!("  llc-18 -filetype=obj -relocation-model=pic -o output.o output.ll");
                println!("  clang-18 output.o -o my_program");
                println!("  ./my_program");
                // UPDATED: 期望的返回码现在是 120
                println!("  echo $?  # Should print 120 on Linux/macOS");
            }
        },
        Err(e) => {
            // 我们的新 CodegenError 现在可以被优雅地打印出来。
            eprintln!("\nError during code generation: {}", e);
        }
    }
}
