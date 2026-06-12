mod ast;
mod das_parser;
mod error;
mod eval;
mod interop;
mod lexer;
mod parser;
mod token;

use das_parser::DasConfig;
use eval::Evaluator;
use interop::LanguageBridge;
use lexer::Lexer;
use parser::Parser;
use std::env;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use std::process::Command;
use token::Token;

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Arre Bhai! File ka naam toh do. Example: cargo run test.ajb");
        return Ok(());
    }

    let file_path = &args[1];

    // .das configuration path: if a second arg is given, load it
    if args.len() >= 3 {
        let das_path = &args[2];
        if let Ok(mut das_file) = File::open(das_path) {
            let mut das_src = String::new();
            das_file.read_to_string(&mut das_src)?;
            let config = DasConfig::parse(&das_src);
            println!(
                "📦 Loaded .das config: '{}'",
                config.get("package", "name").unwrap_or(&"unnamed".into())
            );

            let mut bridge = LanguageBridge::new();
            if config.is_enabled("compatibility", "python_ai_core") {
                bridge.load_compatibility_block("Python", "AI_Core");
            }
            if config.is_enabled("compatibility", "cpp_physics_engine") {
                bridge.load_compatibility_block("C++", "Physics_Engine");
            }
            println!("🔌 Bridge summary:");
            bridge.summary();
        } else {
            println!("⚠️  .das file not found: {}", das_path);
        }
    } else {
        // Look for ajeeb.das automatically in cwd
        if let Ok(mut das_file) = File::open("parth.das") {
            let mut das_src = String::new();
            das_file.read_to_string(&mut das_src)?;
            let config = DasConfig::parse(&das_src);
            println!(
                "📦 parth.das loaded: '{}'",
                config.get("package", "name").unwrap_or(&"unnamed".into())
            );
        }
    }

    let mut file = File::open(file_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    // 1. LEX
    let mut lexer = Lexer::new(&contents);
    let mut tokens = Vec::new();
    let mut token_lines = Vec::new();
    let mut token_cols = Vec::new();
    loop {
        match lexer.next_token_spanned() {
            Ok((Token::Eof, _, _)) => break,
            Ok((tok, line, col)) => {
                tokens.push(tok);
                token_lines.push(line);
                token_cols.push(col);
            }
            Err(e) => {
                println!("{}\n😡 Lexing error! Tokenize karte waqt problem aayi.", e);
                return Ok(());
            }
        }
    }

    println!("✓ Lexer: {} tokens mil gaye", tokens.len());

    // 2. PARSE
    let mut parser = Parser::with_positions(tokens, token_lines, token_cols);
    let ast = match parser.parse_program() {
        Ok(stmts) => stmts,
        Err(e) => {
            println!("{}\n😤 Parsing error! AST banane me problem aayi.", e);
            return Ok(());
        }
    };

    println!("✓ Parser: {} statements parse ho gaye", ast.len());

    // 3. DIRECT EXECUTION
    println!("\n🚀 --- Ajeeb Direct Run Started ---");
    let mut evaluator = Evaluator::new();
    evaluator.set_program_args(args[1..].to_vec());
    evaluator.evaluate_program(&ast);
    println!("--- Ajeeb Execution Ended ---\n🎉 Execution Completed Successfully!");

    // 4. AUTO-COMPILE: if build/output.c was generated, compile it with runtime
    if Path::new("build/output.c").exists() {
        println!("\n🔨 Compiling build/output.c → build/ajeeb_native ...");
        let status = Command::new("gcc")
            .args([
                "build/output.c",
                "runtime/ajeeb_runtime.c",
                "-o",
                "build/ajeeb_native",
                "-Wall",
                "-Wno-int-to-pointer-cast",
                "-Wno-pointer-to-int-cast",
            ])
            .status();
        match status {
            Ok(s) if s.success() => println!("✅ Compilation OK → ./ajeeb_native"),
            Ok(s) => println!("❌ Compilation failed (exit: {})", s),
            Err(e) => println!("❌ Could not run gcc: {}", e),
        }
    }

    Ok(())
}
