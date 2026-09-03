mod lexer;
mod parser;
mod semant;
mod codegen;
mod ast;
mod error;
mod optimizer;
use error::{CompileError, ErrorList};
use codegen::bytecode::OpCode;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use lexer::Lexer;
use parser::Parser;
use semant::SymbolTable;
use codegen::emitter::Emitter;
use ast::Stmt;

// Collects the top-level statements of a module and, transitively, the
// statements of every module it imports. Imports are resolved depth-first, so
// a module's dependencies are always collected before the module itself.
fn collect_module(
    path: &Path,
    symbols: &mut SymbolTable,
    ordered_paths: &mut Vec<PathBuf>,
    out: &mut Vec<Stmt>,
    errors: &mut ErrorList,
) {
    let canonical = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    if ordered_paths.contains(&canonical) {
        return; // Already loaded (or being loaded: guards against import cycles).
    }
    ordered_paths.push(canonical);

    let source = match fs::read_to_string(path) {
        Ok(src) => src,
        Err(e) => {
            errors.push(CompileError::new(
                0, 0, format!("Could not read module '{}': {}", path.display(), e),
            ));
            return;
        }
    };

    // Tokenize.
    let mut lex_errors = ErrorList::new();
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize(&mut lex_errors);
    errors.extend(lex_errors);

    // Parse against the single shared symbol table so that definitions from
    // all modules are visible to one another, exactly as if they were one file.
    let mut parser = Parser::new(tokens);
    let module_ast = match parser.parse(symbols) {
        Ok(ast) => ast,
        Err(parse_errs) => {
            errors.extend(parse_errs);
            return;
        }
    };

    // First, load dependencies so they are collected before this module.
    let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
    for stmt in &module_ast {
        if let Stmt::Import(dep) = stmt {
            resolve_and_load(dep, base_dir, symbols, ordered_paths, out, errors);
        }
    }

    // Then append this module's own non-import statements.
    for stmt in module_ast {
        if !matches!(stmt, Stmt::Import(_)) {
            out.push(stmt);
        }
    }
}

// Resolves an import path relative to the importing file's directory, then
// falls back to the standard library directory, then the current directory.
fn resolve_and_load(
    import: &str,
    base_dir: &Path,
    symbols: &mut SymbolTable,
    ordered_paths: &mut Vec<PathBuf>,
    out: &mut Vec<Stmt>,
    errors: &mut ErrorList,
) {
    let mut candidates: Vec<PathBuf> = Vec::new();
    candidates.push(base_dir.join(import));
    candidates.push(PathBuf::from("stdlib").join(import));
    candidates.push(PathBuf::from(import));

    let mut found: Option<PathBuf> = None;
    for cand in candidates {
        if cand.is_file() {
            found = Some(cand);
            break;
        }
    }

    match found {
        Some(real) => collect_module(&real, symbols, ordered_paths, out, errors),
        None => errors.push(CompileError::new(
            0, 0, format!("Could not resolve import '{}', searched relative and stdlib/.", import),
        )),
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: ambc <file.amb>");
        process::exit(1);
    }

    let filename = &args[1];

    // Parse optional flags after the source file:
    //   ambc <file.amb> [--archive out.ama] [--resource name=path]... [--emit-ir] [--no-opt]
    let mut archive_out: Option<String> = None;
    let mut resources: Vec<(String, Vec<u8>)> = Vec::new();
    let mut emit_ir = false;
    let mut no_opt = false;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--archive" => {
                if i + 1 >= args.len() {
                    eprintln!("Error: --archive requires an output path");
                    process::exit(1);
                }
                archive_out = Some(args[i + 1].clone());
                i += 2;
            }
            "--resource" => {
                if i + 1 >= args.len() {
                    eprintln!("Error: --resource requires name=path");
                    process::exit(1);
                }
                let spec = args[i + 1].clone();
                let (name, path) = match spec.split_once('=') {
                    Some((n, p)) => (n.to_string(), p.to_string()),
                    None => {
                        eprintln!("Error: --resource must be name=path, got '{}'", spec);
                        process::exit(1);
                    }
                };
                match fs::read(&path) {
                    Ok(data) => resources.push((name, data)),
                    Err(e) => {
                        eprintln!("Error: could not read resource '{}': {}", path, e);
                        process::exit(1);
                    }
                }
                i += 2;
            }
            "--emit-ir" => {
                emit_ir = true;
                i += 1;
            }
            "--no-opt" => {
                no_opt = true;
                i += 1;
            }
            other => {
                eprintln!("Error: unexpected argument '{}'", other);
                process::exit(1);
            }
        }
    }

    let main_path = Path::new(filename);

    // Parse and load the main module plus all transitive imports.
    let mut all_errors = ErrorList::new();
    let mut symbols = SymbolTable::new();
    symbols.init_native_registry();
    let mut ordered_paths: Vec<PathBuf> = Vec::new();
    let mut ast: Vec<Stmt> = Vec::new();
    collect_module(main_path, &mut symbols, &mut ordered_paths, &mut ast, &mut all_errors);

    if all_errors.has_errors() {
        eprint!("{}", all_errors);
        process::exit(1);
    }

    // Local optimization pass over the merged AST (skipped with --no-opt).
    let ast = if no_opt { ast } else { optimizer::fold_program(ast) };

    // Emit the merged AST (dependencies first, then the main module).
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
    println!("Amberlink: Compiled {} ({} modules) to {}", filename, ordered_paths.len(), output_path);

    // If requested, dump the backend IR and verify the decode/re-encode round-trip.
    if emit_ir {
        match codegen::ir::decode(&emitter.code) {
            Ok(prog) => {
                let back = codegen::ir::encode(&prog);
                if back != emitter.code {
                    eprintln!("Error: IR round-trip mismatch (decoder drift)");
                    process::exit(1);
                }
                print!("{}", codegen::ir::format_program(&prog, &emitter.constants));
            }
            Err(e) => {
                eprintln!("Error: could not decode IR: {}", e);
                process::exit(1);
            }
        }
    }

    // If requested, wrap the compiled program plus bundled resources in a .ama archive.
    if let Some(archive_path) = archive_out {
        let resource_count = resources.len();
        let bytecode = emitter.to_bytes();
        let ama = codegen::archive::build_archive(bytecode, resources);
        fs::write(&archive_path, ama).expect("Failed to write archive");
        println!("Amberlink: Wrote archive {} ({} resources)", archive_path, resource_count);
    }
}
