mod lexer;
mod parser;
mod semant;
mod codegen;
mod ast;
use codegen::bytecode::OpCode;

use std::env;
use std::fs;
use lexer::Lexer;
use parser::Parser;
use semant::SymbolTable;
use codegen::emitter::Emitter;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: ambc <file.amb>");
        std::process::exit(1);
    }

    let filename = &args[1];
    let source = fs::read_to_string(filename).expect("Failed to read source file");
    
    // 1. Tokenize
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();

    // 2. Parse & Semantic Analysis
    let mut symbols = SymbolTable::new();
    symbols.init_native_registry();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse(&mut symbols);

    // 3. Emit
    let mut emitter = Emitter::new();
    for stmt in ast {
        emitter.emit_stmt(&stmt, &mut symbols);
    }
    emitter.emit_byte(OpCode::Halt.into());
    emitter.finalize(&symbols); // Patch function calls

    let output_path = filename.replace(".amb", ".amc");
    emitter.write_file(&output_path).expect("Failed to write file");
    println!("Amberlink: Compiled {} to {}", filename, output_path);
}
