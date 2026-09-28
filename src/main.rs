use std::env;
use std::fs;
use std::io;
use std::process;

use ssharp::error::SSharpError;
use ssharp::formatter::format_source;
use ssharp::lexer::Lexer;
use ssharp::parser::Parser;
use ssharp::interpreter::{Interpreter, run_repl};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let args: Vec<String> = env::args().collect();

    // Handle --version / -v
    if args.iter().any(|a| a == "--version" || a == "-v") {
        println!("S# (ssharp) v{}", VERSION);
        process::exit(0);
    }

    // Handle --help / -h
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_usage(&args[0]);
        process::exit(0);
    }

    // No file provided - start the interactive REPL
    if args.len() < 2 {
        let stdin = io::stdin();
        run_repl(stdin.lock(), io::stdout());
        process::exit(0);
    }

    // Formatter: `ssharp fmt [--write] file.ssharp`
    if args[1] == "fmt" {
        let (write, file_path) = match args.get(2).map(|s| s.as_str()) {
            Some("-w") | Some("--write") => match args.get(3) {
                Some(f) => (true, f),
                None => {
                    eprintln!("Usage: {} fmt [--write] <file.ssharp>", args[0]);
                    process::exit(1);
                }
            },
            Some(_) => (false, &args[2]),
            None => {
                eprintln!("Usage: {} fmt [--write] <file.ssharp>", args[0]);
                process::exit(1);
            }
        };
        let source = match fs::read_to_string(file_path) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("Error reading file '{}': {}", file_path, e);
                process::exit(1);
            }
        };
        match format_source(&source) {
            Ok(formatted) => {
                if write {
                    if let Err(e) = fs::write(file_path, formatted) {
                        eprintln!("Error writing file '{}': {}", file_path, e);
                        process::exit(1);
                    }
                } else {
                    print!("{}", formatted);
                }
            }
            Err(e) => {
                eprintln!("{}", e);
                process::exit(1);
            }
        }
        process::exit(0);
    }

    let file_path = &args[1];

    let source = match fs::read_to_string(file_path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", file_path, e);
            process::exit(1);
        }
    };

    if let Err(e) = run(&source, file_path) {
        eprintln!("{}", e);
        process::exit(1);
    }
}

fn print_usage(program_name: &str) {
    println!("Usage: {} [file.ssharp]", program_name);
    println!("       {} fmt [--write] [file.ssharp]", program_name);
    println!();
    println!("Run an S# script file, or start the interactive REPL when no file is given.");
    println!();
    println!("Examples:");
    println!("  {} my_program.ssharp", program_name);
    println!("  {} fmt my_program.ssharp           # print canonically formatted source", program_name);
    println!("  {} fmt --write my_program.ssharp   # reformat the file in place", program_name);
    println!("  {}                         # start the REPL", program_name);
    println!();
    println!("Options:");
    println!("  -h, --help     Print this help message and exit");
    println!("  -v, --version  Print version information and exit");
}

fn run(source: &str, file_path: &str) -> Result<(), SSharpError> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize()?;

    let mut parser = Parser::new(tokens);
    let program = parser.parse()?;

    let mut interpreter = Interpreter::new();
    let path = std::path::Path::new(file_path);
    let base_dir = path
        .parent()
        .map(|p| {
            if p.as_os_str().is_empty() {
                std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
            } else {
                p.to_path_buf()
            }
        })
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    interpreter.set_base_dir(base_dir);
    // The entry script counts as imported: a library importing it back
    // resolves to a no-op instead of re-running the whole program.
    if let Ok(canonical) = path.canonicalize() {
        interpreter.mark_imported(canonical);
    }
    interpreter.interpret(&program)?;

    Ok(())
}