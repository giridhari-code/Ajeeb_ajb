mod token;
mod lexer;
mod parser;
mod ast;
mod codegen;
mod error;

use std::env;
use std::fs::File;
use std::io::{self, Read, Write};
use lexer::Lexer;
use parser::Parser;
use codegen::CCodeGen;
use token::Token;

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Arre Bhai! File ka naam toh do. Example: cargo run test.ajb");
        return Ok(());
    }

    let file_path = &args[1];
    let mut file = File::open(file_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    // 1. LEX
    let mut lexer = Lexer::new(&contents);
    let mut tokens = Vec::new();
    loop {
        match lexer.next_token() {
            Ok(Token::Eof) => break,
            Ok(tok) => tokens.push(tok),
            Err(e) => {
                println!("{}\n😡 Lexing error! Tokenize karte waqt problem aayi.", e);
                return Ok(());
            }
        }
    }

    println!("✓ Lexer: {} tokens mil gaye", tokens.len());

    // 2. PARSE
    let mut parser = Parser::new(tokens);
    let ast = match parser.parse_program() {
        Ok(stmts) => stmts,
        Err(e) => {
            println!("{}\n😤 Parsing error! AST banane me problem aayi.", e);
            return Ok(());
        }
    };

    println!("✓ Parser: {} statements parse ho gaye", ast.len());

    // 3. C CODE GENERATION
    let mut cgen = CCodeGen::new();
    match cgen.generate_c_source(&ast) {
        Ok(c_code) => {
            let mut c_file = File::create("output.c")?;
            c_file.write_all(c_code.as_bytes())?;
            println!("🎉 Sukriya! 'output.c' file create ho chuki hai.");
            println!("📝 Compile karne ke liye: gcc output.c -o output && ./output");
        }
        Err(e) => {
            println!("{}\n🔥 Code generation error! C code nahi ban paayi.", e);
        }
    }

    Ok(())
}
