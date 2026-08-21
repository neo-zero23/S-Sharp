use std::env;
use std::fs;
use std::process;

use ssharp::error::SSharpError;
use ssharp::lexer::Lexer;
use ssharp::parser::Parser;
use ssharp::interpreter::Interpreter;

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

    // No arguments provided - show usage and exit with error code
    if args.len() < 2 {
        print_usage(&args[0]);
        process::exit(1);
    }

    let file_path = &args[1];

    let source = match fs::read_to_string(file_path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", file_path, e);
            process::exit(1);
        }
    };

    if let Err(e) = run(&source) {
        eprintln!("{}", e);
        process::exit(1);
    }
}

fn print_usage(program_name: &str) {
    println!("Usage: {} <file.ssharp>", program_name);
    println!();
    println!("Run an S# script file.");
    println!();
    println!("Example:");
    println!("  {} my_program.ssharp", program_name);
    println!();
    println!("Options:");
    println!("  -h, --help     Print this help message and exit");
    println!("  -v, --version  Print version information and exit");
}

fn run(source: &str) -> Result<(), SSharpError> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize()?;

    let mut parser = Parser::new(tokens);
    let program = parser.parse()?;

    let mut interpreter = Interpreter::new();
    interpreter.interpret(&program)?;

    Ok(())
}