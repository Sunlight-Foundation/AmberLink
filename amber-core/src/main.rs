mod lexer;
mod parser;
mod semant;
mod codegen;
mod ast;
mod error;
use error::ErrorList;
use codegen::bytecode::OpCode;

use std::env;
use std::fs;
use std::process;
use lexer::Lexer;
use parser::Parser;
use semant::SymbolTable;
use codegen::emitter::Emitter;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: ambc <file.amb>");
        process::exit(1);
    }

    let filename = &args[1];
    let source = fs::read_to_string(filename).unwrap_or_else(|e| {
        eprintln!("Error: could not read '{}': {}", filename, e);
        process::exit(1);
    });

    // 1. Tokenize
    let mut lex_errors = ErrorList::new();
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize(&mut lex_errors);

    // 2. Parse & Semantic Analysis
    let mut symbols = SymbolTable::new();
    symbols.init_native_registry();
    let mut parser = Parser::new(tokens);
    let ast = match parser.parse(&mut symbols) {
        Ok(ast) => ast,
        Err(errors) => {
            // Combine lexer + parser errors and report them all.
            let mut all = lex_errors;
            all.extend(errors);
            eprint!("{}", all);
            process::exit(1);
        }
    };

    // If the lexer produced errors (e.g. unknown characters), fail before emission.
    if lex_errors.has_errors() {
        eprint!("{}", lex_errors);
        process::exit(1);
    }

    // 3. Emit
    let mut emit_errors = ErrorList::new();
    let mut emitter = Emitter::new();
    for stmt in ast {
        emitter.emit_stmt(&stmt, &mut symbols, &mut emit_errors);
    }
    emitter.emit_byte(OpCode::Halt.into());
    emitter.finalize(&symbols, &mut emit_errors); // Patch function calls

    if emit_errors.has_errors() {
        eprint!("{}", emit_errors);
        process::exit(1);
    }

    let output_path = filename.replace(".amb", ".amc");
    emitter.write_file(&output_path).expect("Failed to write file");
    println!("Amberlink: Compiled {} to {}", filename, output_path);
}
