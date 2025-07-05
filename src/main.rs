mod lexer;
mod ast;
mod parser;
mod codegen;

use std::fs;
use clap::Parser as ClapParser;
use logos::Logos;
// 【已修正】引入 Stream
use chumsky::{Parser, Stream};
use inkwell::context::Context;

use crate::lexer::Token;
use crate::parser::program_parser;
use crate::codegen::CodeGen;

/// Tipy 语言的简易编译器
#[derive(ClapParser, Debug)]
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

    let lexer = Token::lexer(&source_code);
    let tokens: Vec<_> = lexer.spanned()
        .filter_map(|(token, span)| token.ok().map(|t| (t, span)))
        .collect();

    // 【核心修正】显式创建 Stream 对象
    // 我们需要告诉 Stream 输入流的结尾在哪里，这里就是 tokens 的长度
    let token_stream = Stream::from_iter(tokens.len()..tokens.len() + 1, tokens.into_iter());
    let (ast, parse_errors) = program_parser().parse_recovery(token_stream);
    
    if !parse_errors.is_empty() {
        println!("语法分析失败，产生 {} 个错误:", parse_errors.len());
        for error in parse_errors {
            println!("- {:?}", error);
        }
        return;
    }

    let program_ast = if let Some(ast) = ast {
        ast
    } else {
        println!("严重错误: AST 未能生成。");
        return;
    };
    println!("AST 生成成功!");
    
    println!("\n(3) 正在生成 LLVM IR...");
    let context = Context::create();
    let mut codegen = CodeGen::new(&context, "tipy_module");
    
    match codegen.compile(program_ast) {
        Ok(llvm_ir) => {
            println!("LLVM IR 生成成功:\n");
            println!("{}", llvm_ir);
            
            fs::write("output.ll", llvm_ir).expect("无法写入 output.ll");
            println!("\nLLVM IR 已写入 output.ll 文件。");
        }
        Err(e) => {
            println!("代码生成失败: {}", e);
        }
    }
}