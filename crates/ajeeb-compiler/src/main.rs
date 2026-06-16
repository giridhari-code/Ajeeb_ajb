mod ast;
mod cache;
mod llvm;
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

fn detect_backend() -> &'static str {
    if Command::new("llc")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return "llvm";
    }
    if Command::new("gcc")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return "gcc";
    }
    "interpreter"
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Arre Bhai! File ka naam toh do. Example: cargo run test.ajb");
        return Ok(());
    }

    // --- Parse arguments ---
    let file_path = &args[1];
    let positional: Vec<&String> = args[1..]
        .iter()
        .filter(|a| !a.starts_with("--"))
        .collect();
    let output_path = if positional.len() >= 2 {
        positional[1].as_str()
    } else {
        "build/output.ll"
    };

    let force_llvm = args.iter().any(|a| a == "--llvm");
    let force_gcc = args.iter().any(|a| a == "--gcc");
    let skip_run = args.iter().any(|a| a == "--skip-run");
    let skip_compile = args.iter().any(|a| a == "--skip-compile");
    let force_run = args.iter().any(|a| a == "--run");

    // --- Detect backend ---
    let backend = if force_llvm {
        "llvm"
    } else if force_gcc {
        "gcc"
    } else {
        detect_backend()
    };

    // --- Print package info ---
    for dir in [Path::new("."), Path::new("..")] {
        let das_path = dir.join("parth.das");
        if let Ok(mut das_file) = File::open(&das_path) {
            let mut das_src = String::new();
            das_file.read_to_string(&mut das_src)?;
            let config = DasConfig::parse(&das_src);
            let name = config
                .get("package", "name")
                .cloned()
                .unwrap_or_default();
            let version = config
                .get("package", "version")
                .cloned()
                .unwrap_or_default();
            if !name.is_empty() && name != "project" {
                println!("📦 parth: '{}' v{}", name, version);
            }
            break;
        }
    }

    match backend {
        "llvm" => println!("⚡ Backend: LLVM (llc + as + ld)"),
        "gcc" => println!("🔧 Backend: GCC (C codegen)"),
        _ => println!("🐢 Backend: Interpreter only"),
    }

    // --- Binary name from input file ---
    let bin_name = Path::new(file_path)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let bin_path = format!("build/{}", bin_name);

    // 0. CACHE CHECK
    let entry_path = Path::new(file_path);
    let mut module_cache = ModuleCache::new(PathBuf::from("build/cache"));
    module_cache.add_source(entry_path);

    let all_stmts = if let Some(cached_stmts) = module_cache.load() {
        println!("✓ Cache hit: {} statements loaded from cache", cached_stmts.len());
        cached_stmts
    } else {
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

        println!("✓ Lexer: {} tokens", tokens.len());

        // 2. PARSE
        let mut parser = Parser::with_positions(tokens, token_lines, token_cols);
        let ast = match parser.parse_program() {
            Ok(stmts) => stmts,
            Err(e) => {
                println!("{}\n😤 Parsing error! AST banane me problem aayi.", e);
                return Ok(());
            }
        };

        println!("✓ Parser: {} statements", ast.len());

        // 2b. MODULE LOADING
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

        let module_name = entry_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("main");
        let entry_module = module::Module {
            name: module_name.to_string(),
            file_path: entry_path.to_path_buf(),
            imports: ast
                .iter()
                .filter_map(|s| {
                    if let Stmt::Import(i) = s {
                        Some(i.clone())
                    } else {
                        None
                    }
                })
                .collect(),
            stmts: ast,
        };
        loader
            .modules
            .insert(module_name.to_string(), entry_module);

        if let Err(e) = loader.resolve_imports() {
            println!("❌ Module resolution error: {}", e);
            return Ok(());
        }

        let resolved_stmts = loader.collect_all_stmts();
        println!(
            "✓ Modules: {} loaded, {} statements",
            loader.modules.len(),
            resolved_stmts.len()
        );

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
        return Ok(());
    }
    println!("✓ Semantic: OK");

    // 4. INTERPRETER — only run if backend is interpreter, or --run is passed
    let run_interpreter = backend == "interpreter" || force_run;
    if run_interpreter && !skip_run {
        println!("\n🚀 --- Ajeeb Direct Run Started ---");
        let mut evaluator = Evaluator::new();
        let mut program_args = vec![args[0].clone()];
        for arg in &args[2..] {
            if !arg.starts_with("--") {
                program_args.push(arg.clone());
            }
        }
        evaluator.set_program_args(program_args);
        evaluator.evaluate_program(&all_stmts);
        println!("--- Ajeeb Execution Ended ---\n🎉 Execution Completed Successfully!");
    }

    // 5. CODEGEN
    if skip_compile {
        println!("\n⏭️  Skipping codegen (--skip-compile)");
        return Ok(());
    }

    std::fs::create_dir_all("build").ok();
    let mut compiled_ok = false;

    if backend == "llvm" {
        // --- LLVM PIPELINE ---
        let mut codegen = llvm::Codegen::new();
        match codegen.compile(&all_stmts) {
            Ok(_) => {
                // Determine where to write .ll
                let ll_path = if output_path.ends_with(".ll") {
                    output_path.to_string()
                } else {
                    "build/output.ll".to_string()
                };
                codegen.write_ir_to_file(&ll_path).ok();

                println!("\n🔨 Compiling {} → {}", file_path, bin_path);

                // Step 1: llc — LLVM IR → Assembly
                let llc_ok = Command::new("llc")
                    .args(["-O2", &ll_path, "-o", "build/output.s"])
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false);

                if !llc_ok {
                    println!("❌ llc failed");
                    return Ok(());
                }

                // Step 2: as — Assembly → Object
                let as_ok = Command::new("as")
                    .args(["build/output.s", "-o", "build/output.o"])
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false);

                if !as_ok {
                    println!("❌ as failed");
                    return Ok(());
                }

                // Step 3: compile runtime.c to object (if not cached)
                if !Path::new("build/runtime.o").exists() {
                    let runtime_ok = Command::new("gcc")
                        .args([
                            "-c",
                            "runtime/ajeeb_runtime.c",
                            "-o",
                            "build/runtime.o",
                            "-Wno-int-to-pointer-cast",
                        ])
                        .status()
                        .map(|s| s.success())
                        .unwrap_or(false);

                    if !runtime_ok {
                        println!("❌ Runtime compilation failed");
                        return Ok(());
                    }
                }

                // Step 4: link objects → binary (cc handles CRT startup + libc)
                let link_status = Command::new("cc")
                    .args([
                        "build/output.o",
                        "build/runtime.o",
                        "-o",
                        &bin_path,
                        "-lm",
                        "-ldl",
                    ])
                    .status();

                match link_status {
                    Ok(s) if s.success() => {
                        println!("✅ Ready: ./{}", bin_path);
                        compiled_ok = true;
                    }
                    Ok(s) => println!("❌ ld failed (exit: {})", s),
                    Err(e) => println!("❌ ld error: {}", e),
                }
            }
            Err(e) => println!("⚠️  LLVM codegen skipped: {}", e),
        }
    } else if backend == "gcc" {
        // --- GCC FALLBACK: C codegen pipeline ---
        // Use the C codegen path if available
        let output_c = "build/output.c";
        if Path::new(output_c).exists() {
            println!("\n🔨 Compiling {} → {}", file_path, bin_path);
            let gcc_status = Command::new("gcc")
                .args([
                    output_c,
                    "runtime/ajeeb_runtime.c",
                    "-o",
                    &bin_path,
                    "-Wall",
                    "-Wno-int-to-pointer-cast",
                    "-Wno-pointer-to-int-cast",
                    "-ldl",
                ])
                .status();
            match gcc_status {
                Ok(s) if s.success() => {
                    println!("✅ Ready: ./{}", bin_path);
                    compiled_ok = true;
                }
                Ok(s) => println!("❌ GCC failed (exit: {})", s),
                Err(e) => println!("❌ gcc error: {}", e),
            }
        } else {
            println!("ℹ️  No C output found. Use --llvm or build with self-hosted compiler.");
        }
    }

    // 6. BUILD SUMMARY
    if compiled_ok {
        println!("\n═══════════════════════════════");
        println!("📦 Build: ./{}", bin_path);
        println!("═══════════════════════════════");
    }

    Ok(())
}
