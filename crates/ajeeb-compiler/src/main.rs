mod ast;
mod cache;
mod codegen;
mod das_parser;
mod error;
mod eval;
mod interop;
mod lexer;
mod module;
mod parser;
mod semantic;
mod token;

use ast::Stmt;
use cache::ModuleCache;
use das_parser::DasConfig;
use eval::Evaluator;
use lexer::Lexer;
use module::ModuleLoader;
use parser::Parser;
use semantic::SemanticAnalyzer;
use std::env;
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::Command;
use token::Token;

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Arre Bhai! File ka naam toh do. Example: cargo run test.ajb");
        return Ok(());
    }

    let file_path = &args[1];
    let output_path = if args.len() >= 3 { &args[2] } else { "build/output.ll" };
    let use_llvm = args.iter().any(|a| a == "--llvm");

    // Auto-discover parth.das (check cwd, then parent)
    for dir in [Path::new("."), Path::new("..")] {
        let das_path = dir.join("parth.das");
        if let Ok(mut das_file) = File::open(&das_path) {
            let mut das_src = String::new();
            das_file.read_to_string(&mut das_src)?;
            let config = DasConfig::parse(&das_src);
            let name = config.get("package", "name").cloned().unwrap_or_default();
            let version = config.get("package", "version").cloned().unwrap_or_default();
            if !name.is_empty() && name != "project" {
                println!("📦 parth: '{}' v{}", name, version);
            }
            break;
        }
    }

    // 0. CACHE CHECK — skip lex/parse/module-loading if cached AST is still fresh
    let entry_path = Path::new(file_path);
    let mut module_cache = ModuleCache::new(PathBuf::from("build/cache"));
    module_cache.add_source(entry_path);

    let all_stmts = if let Some(cached_stmts) = module_cache.load() {
        println!("✓ Cache hit: {} statements loaded from cache", cached_stmts.len());
        cached_stmts
    } else {
        // Cache miss — run full pipeline
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

        // 2b. MODULE LOADING — resolve imports
        let mut loader = ModuleLoader::new();
        let entry_dir = entry_path.parent().unwrap_or(Path::new("."));
        loader.add_import_path(entry_dir.to_path_buf());

        if Path::new("std").exists() {
            loader.add_import_path(Path::new("std").to_path_buf());
        }
        if Path::new("../std").exists() {
            loader.add_import_path(Path::new("../std").to_path_buf());
        }
        for dir in [Path::new("."), Path::new("..")] {
            let das_path = dir.join("parth.das");
            if das_path.exists() {
                let std_path = dir.join("std");
                if std_path.exists() {
                    loader.add_import_path(std_path);
                }
                break;
            }
        }

        let module_name = entry_path.file_stem().and_then(|s| s.to_str()).unwrap_or("main");
        let entry_module = module::Module {
            name: module_name.to_string(),
            file_path: entry_path.to_path_buf(),
            imports: ast.iter().filter_map(|s| {
                if let Stmt::Import(i) = s { Some(i.clone()) } else { None }
            }).collect(),
            stmts: ast,
        };
        loader.modules.insert(module_name.to_string(), entry_module);

        if let Err(e) = loader.resolve_imports() {
            println!("❌ Module resolution error: {}", e);
            return Ok(());
        }

        let resolved_stmts = loader.collect_all_stmts();
        println!("✓ Module loader: {} modules, {} total statements", loader.modules.len(), resolved_stmts.len());

        // After successful module loading, cache the result with all source mtimes
        for module in loader.modules.values() {
            module_cache.add_source(&module.file_path);
        }
        module_cache.save(&resolved_stmts);

        resolved_stmts
    };

    // 3. SEMANTIC ANALYSIS
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(&all_stmts);
    if !analyzer.errors.is_empty() {
        for err in &analyzer.errors {
            println!("{}", err);
        }
        println!("\n😤 Semantic analysis failed! Code mein type ya scope ki problem hai.");
    }

    // 4. DIRECT EXECUTION (skip with --skip-run)
    let skip_run = args.iter().any(|a| a == "--skip-run");
    if !skip_run {
        println!("\n🚀 --- Ajeeb Direct Run Started ---");
        let mut evaluator = Evaluator::new();
        let mut program_args = vec![args[0].clone()];
        if args.len() >= 3 {
            program_args.extend_from_slice(&args[2..]);
        }
        evaluator.set_program_args(program_args);
        evaluator.evaluate_program(&all_stmts);
        println!("--- Ajeeb Execution Ended ---\n🎉 Execution Completed Successfully!");
    } else {
        println!("\n⏭️  Skipping direct execution (--skip-run)");
    }

    // 5. LLVM IR CODEGEN (Phase 2 native compilation)
    let mut llvm_ok = false;
    let mut gcc_ok = false;

    if use_llvm && Command::new("llc").arg("--version").status().is_ok() {
        let mut codegen = codegen::Codegen::new();
        match codegen.compile(&all_stmts) {
            Ok(_) => {
                std::fs::create_dir_all("build").ok();
                codegen.write_ir_to_file(output_path).ok();
                println!("\n🔨 Compiling {} → build/ajeeb_llvm (via llc + as + ld) ...", output_path);
                let status = Command::new("llc")
                    .args(["-O2", output_path, "-o", "build/output.s"])
                    .status()
                    .and_then(|s| if s.success() {
                        Command::new("as")
                            .args(["build/output.s", "-o", "build/output.o"])
                            .status()
                            .and_then(|s2| if s2.success() {
                                Command::new("gcc")
                                    .args(["build/output.o", "runtime/ajeeb_runtime.c", "-o", "build/ajeeb_llvm", "-lm", "-ldl", "-Wl,--allow-multiple-definition"])
                                    .status()
                            } else {
                                Ok(s2)
                            })
                    } else {
                        Ok(s)
                    });
                match status {
                    Ok(s) if s.success() => { println!("✅ LLVM Compilation OK → ./build/ajeeb_llvm"); llvm_ok = true; }
                    Ok(s) => println!("❌ LLVM Compilation failed (exit: {})", s),
                    Err(e) => println!("❌ Could not run clang: {}", e),
                }
            }
            Err(e) => println!("⚠️  LLVM codegen skipped: {}", e),
        }
    } else if use_llvm {
        println!("ℹ️  llc not found — skipping LLVM codegen");
        println!("   Install LLVM to enable native compilation");
    }

    // 6. LEGACY C CODEGEN: if build/output.c was generated, compile it with runtime
    if Path::new("build/output.c").exists() {
        println!("\n🔨 Compiling build/output.c → build/ajeeb_native (via gcc) ...");
        let status = Command::new("gcc")
            .args([
                "build/output.c",
                "runtime/ajeeb_runtime.c",
                "-o",
                "build/ajeeb_native",
                "-Wall",
                "-Wno-int-to-pointer-cast",
                "-Wno-pointer-to-int-cast",
                "-ldl",
            ])
            .status();
        match status {
            Ok(s) if s.success() => { println!("✅ GCC Compilation OK → ./build/ajeeb_native"); gcc_ok = true; }
            Ok(s) => println!("❌ GCC Compilation failed (exit: {})", s),
            Err(e) => println!("❌ Could not run gcc: {}", e),
        }
    }

    // 7. BUILD SUMMARY
    println!("\n═══════════════════════════════");
    println!("📦 Build Summary:");
    if llvm_ok {
        println!("  ⚡ Native (LLVM): build/ajeeb_llvm");
    }
    if gcc_ok {
        println!("  🔧 Native (GCC):  build/ajeeb_native");
    }
    println!("  🐢 Interpreter:   cargo run -p ajeeb-compiler --bin ajeeb_compiler");
    println!("═══════════════════════════════");

    Ok(())
}
