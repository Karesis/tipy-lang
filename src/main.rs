mod token;
mod lexer;
mod ast;
mod parser;

use lexer::Lexer;
use parser::Parser;

fn main() {
    let input = r#"
    main() {
        print("Hello, Tipy World!")
    }
    "#;

    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);

    let program = parser.parse_program();

    // 使用 {:?} 打印基础结构
    // 使用 {:#?} 可以进行“美化”打印，对树状结构更友好
    println!("Generated AST:\n{:#?}", program);
}