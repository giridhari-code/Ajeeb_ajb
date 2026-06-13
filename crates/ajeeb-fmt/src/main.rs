use std::process;
use ajeeb_fmt::config::FormatConfig;
use ajeeb_fmt::formatter::format_source;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut config = FormatConfig::default();
    let mut files = Vec::new();
    let mut i = 1;

    while i < args.len() {
        match args[i].as_str() {
            "--check" => { config.check_mode = true; config.write_mode = false; }
            "--write" => { config.write_mode = true; config.check_mode = false; }
            "--stdout" => { config.stdout_mode = true; config.write_mode = false; }
            "--indent" => {
                i += 1;
                if i < args.len() {
                    config.indent_size = args[i].parse().unwrap_or(4);
                }
            }
            "--width" => {
                i += 1;
                if i < args.len() {
                    config.max_line_width = args[i].parse().unwrap_or(100);
                }
            }
            "--tab" => { config.use_tabs = true; }
            "--help" | "-h" => {
                print_help();
                return;
            }
            f if f.starts_with('-') => {
                eprintln!("Unknown option: {}", f);
                process::exit(1);
            }
            f => { files.push(f.to_string()); }
        }
        i += 1;
    }

    if files.is_empty() {
        // Read from stdin
        let mut source = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut source).unwrap();
        match format_source(&config, &source) {
            Ok(formatted) => { print!("{}", formatted); }
            Err(e) => { eprintln!("Error: {}", e); process::exit(1); }
        }
        return;
    }

    config.files = files;
    let mut had_errors = false;
    let mut had_unformatted = false;

    for file in &config.files {
        let source = match std::fs::read_to_string(file) {
            Ok(s) => s,
            Err(e) => { eprintln!("Error reading {}: {}", file, e); had_errors = true; continue; }
        };
        match format_source(&config, &source) {
            Ok(formatted) => {
                if config.check_mode {
                    if formatted != source {
                        eprintln!("{}: would reformat", file);
                        had_unformatted = true;
                    }
                } else if config.stdout_mode {
                    print!("{}", formatted);
                } else if config.write_mode {
                    if formatted != source {
                        match std::fs::write(file, &formatted) {
                            Ok(_) => eprintln!("Formatted: {}", file),
                            Err(e) => { eprintln!("Error writing {}: {}", file, e); had_errors = true; }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Error formatting {}: {}", file, e);
                had_errors = true;
            }
        }
    }

    if had_errors { process::exit(1); }
    if had_unformatted { process::exit(1); }
}

fn print_help() {
    println!("ajeeb fmt — Ajeeb code formatter");
    println!();
    println!("USAGE:");
    println!("  ajeeb fmt [OPTIONS] [FILES...]");
    println!();
    println!("OPTIONS:");
    println!("  --check       Check formatting without modifying files");
    println!("  --write       Write formatted output in-place (default)");
    println!("  --stdout      Write formatted output to stdout");
    println!("  --indent N    Indentation width (default: 4)");
    println!("  --width N     Max line width (default: 100)");
    println!("  --tab         Use tabs for indentation");
    println!("  --help        Print this help");
}
