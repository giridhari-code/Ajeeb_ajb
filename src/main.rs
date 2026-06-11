mod token;
mod lexer;
mod parser;
mod ast;
mod error;
mod eval;
mod das_parser;
mod interop;

use std::env;
use std::fs::File;
use std::io::{self, Read};
use lexer::Lexer;
use parser::Parser;
use token::Token;
use eval::Evaluator;
use das_parser::DasConfig;
use interop::LanguageBridge;

fn print_logo() {
    println!(r#"
  ┌────────────────────────────────────────────────────────┐
  │   █████╗      ██╗███████╗███████╗██████╗  ██╗      ██╗ │
  │  ██╔══██╗     ██║██╔════╝██╔════╝██╔══██╗ ██║      ██║ │
  │  ███████║     ██║█████╗  █████╗  ██████╔╝ ███████████║ │
  │  ██╔══██║██   ██║██╔══╝  ██╔══╝  ██╔══██╗ ╚══════██╔═╝ │
  │  ██║  ██║╚█████╔╝███████╗███████╗██████╔╝        ██║   │
  │  ╚═╝  ╚═╝ ╚════╝ ╚══════╝╚══════╝╚═════╝         ╚═╝   │
  │              v{} · Ajeeb Dynamic Language               │
  └────────────────────────────────────────────────────────┘
    "#, env!("CARGO_PKG_VERSION"));
}

fn main() -> io::Result<()> {
    print_logo();

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
            println!("📦 Loaded .das config: '{}'", config.get("module", "name").unwrap_or(&"unnamed".into()));

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
        if let Ok(mut das_file) = File::open("ajeeb.das") {
            let mut das_src = String::new();
            das_file.read_to_string(&mut das_src)?;
            let config = DasConfig::parse(&das_src);
            println!("📦 Auto-loaded ajeeb.das: '{}'", config.get("module", "name").unwrap_or(&"unnamed".into()));
        }
    }

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

    // 3. DIRECT EXECUTION
    println!("\n🚀 --- Ajeeb Direct Run Started ---");
    let mut evaluator = Evaluator::new();
    evaluator.evaluate_program(&ast);
    println!("--- Ajeeb Execution Ended ---\n🎉 Execution Completed Successfully!");

    Ok(())
}
