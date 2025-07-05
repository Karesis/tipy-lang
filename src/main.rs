// src/main.rs

use std::fs;
use clap::Parser;
// ... 我们很快会在这里加上 logos, chumsky 等的 use 声明

/// Tipy 语言的简易编译器
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// 需要被编译的 .tp 文件路径
    #[arg(required = true)]
    file_path: String,
}

fn main() {
    let args = Args::parse();
    let source_code = match fs::read_to_string(&args.file_path) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("错误: 无法读取文件 '{}': {}", args.file_path, e);
            return;
        }
    };

    // --- 编译器流水线 ---
    // 1. 词法分析
    println!("(1) 正在进行词法分析...");
    // TODO: 使用 `logos` 将 `source_code` 转换为 Token 流

    // 2. 语法分析
    println!("(2) 正在进行语法分析...");
    // TODO: 使用 `chumsky` 解析 Token 流，生成 AST

    // 3. 语义分析与代码生成
    println!("(3) 正在生成 LLVM IR...");
    // TODO: 遍历 AST，使用 `inkwell` 生成 LLVM IR
    
    println!("\n编译成功! (模拟)");
}