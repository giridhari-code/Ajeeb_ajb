mod ast;
mod das_parser;
mod error;
mod eval;
mod interop;
mod lexer;
mod parser;
mod semantic;
mod token;

use das_parser::DasConfig;
use eval::Evaluator;
use lexer::Lexer;
use parser::Parser;
use semantic::SemanticAnalyzer;
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

    // Auto-discover parth.das (check cwd, then parent)
    for dir in [Path::new("."), Path::new("..")] {
        let das_path = dir.join("parth.das");
        if let Ok(mut das_file) = File::open(&das_path) {
            let mut das_src = String::new();
            das_file.read_to_string(&mut das_src)?;
            let config = DasConfig::parse(&das_src);
            let name = config.get("package", "name").cloned().unwrap_or_default();
            let version = config.get("package", "version").cloned().unwrap_or_default();
            if !name.is_empty() {
                println!("📦 parth: '{}' v{}", name, version);
            }
            break;
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

    // 3. SEMANTIC ANALYSIS
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(&ast);
    if !analyzer.errors.is_empty() {
        for err in &analyzer.errors {
            println!("{}", err);
        }
        println!("\n😤 Semantic analysis failed! Code mein type ya scope ki problem hai.");
    }

    // 4. DIRECT EXECUTION
    println!("\n🚀 --- Ajeeb Direct Run Started ---");
    let mut evaluator = Evaluator::new();
    // Include program name at index 0 to match C runtime's argv convention
    let mut program_args = vec![args[0].clone()];
    program_args.extend_from_slice(&args[1..]);
    evaluator.set_program_args(program_args);
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
