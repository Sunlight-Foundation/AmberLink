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

/// Compiles a single `.amb` file (plus its transitive imports) into `.amc`
/// bytecode bytes. Returns the bytecode and the list of loaded module paths.
fn compile_file(
    path: &Path,
    no_opt: bool,
    emit_link_info: bool,
) -> Result<(Vec<u8>, Vec<PathBuf>), ErrorList> {
    let mut all_errors = ErrorList::new();
    let mut symbols = SymbolTable::new();
    symbols.init_native_registry();
    let mut ordered_paths: Vec<PathBuf> = Vec::new();
    let mut ast: Vec<Stmt> = Vec::new();
    collect_module(path, &mut symbols, &mut ordered_paths, &mut ast, &mut all_errors);

    if all_errors.has_errors() {
        return Err(all_errors);
    }

    // Local optimization pass over the merged AST (skipped with --no-opt).
    let ast = if no_opt { ast } else { optimizer::fold_program(ast) };

    // Emit the merged AST.
    let mut emit_errors = ErrorList::new();
    let mut emitter = Emitter::new();
    emitter.emit_link_info = emit_link_info;
    for stmt in ast {
        emitter.emit_stmt(&stmt, &mut symbols, &mut emit_errors);
    }
    emitter.emit_byte(OpCode::Halt.into());
    emitter.finalize(&symbols, &mut emit_errors); // Patch function calls

    if emit_errors.has_errors() {
        return Err(emit_errors);
    }

    Ok((emitter.to_bytes(), ordered_paths))
}

/// Runtime support linked into every native build (see runtime/amb_rt.c).
const AMB_RUNTIME_C: &str = include_str!("../runtime/amb_rt.c");

/// Lowers a compiled `.amc` image to LLVM IR text, exiting with diagnostics on failure.
fn lower_to_llvm(amc: &[u8]) -> String {
    let (consts, code) = match codegen::ir::split_amc(amc) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error: could not read compiled image: {}", e);
            process::exit(1);
        }
    };
    let prog = match codegen::ir::decode(&code) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: could not decode IR for LLVM emission: {}", e);
            process::exit(1);
        }
    };
    match codegen::llvm_backend::emit_llvm(&prog, &consts) {
        Ok(ll) => ll,
        Err(errs) => {
            for e in errs {
                eprintln!("LLVM backend error: {}", e);
            }
            process::exit(1);
        }
    }
}

/// Builds a native executable from LLVM IR by invoking `clang` (must be on PATH).
fn build_native(ll: &str, exe: &str) {
    let dir = env::temp_dir().join(format!("amberlink-native-{}", process::id()));
    fs::create_dir_all(&dir).expect("Failed to create temp dir");
    let ll_path = dir.join("program.ll");
    let rt_path = dir.join("amb_rt.c");
    fs::write(&ll_path, ll).expect("Failed to write LLVM IR");
    fs::write(&rt_path, AMB_RUNTIME_C).expect("Failed to write runtime");
    let status = process::Command::new("clang")
        .arg("-O2")
        .arg("-Wno-override-module")
        .arg(&ll_path)
        .arg(&rt_path)
        .arg("-o")
        .arg(exe)
        .status();
    let _ = fs::remove_dir_all(&dir);
    match status {
        Ok(s) if s.success() => println!("Amberlink: Built native executable {}", exe),
        Ok(s) => {
            eprintln!("Error: clang exited with {}", s);
            process::exit(1);
        }
        Err(e) => {
            eprintln!("Error: could not run clang ({}). Install LLVM/clang and put it on PATH.", e);
            process::exit(1);
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: ambc <file.amb> [--module mod.amb]... [--archive out.ama] [--resource name=path]... [--emit-ir] [--emit-llvm] [--native out] [--no-opt]");
        process::exit(1);
    }

    let filename = &args[1];

    // Parse optional flags after the source file:
    //   ambc <file.amb> [--module mod.amb]... [--archive out.ama]
    //       [--resource name=path]... [--emit-ir] [--emit-llvm] [--no-opt]
    let mut archive_out: Option<String> = None;
    let mut resources: Vec<(String, Vec<u8>)> = Vec::new();
    let mut extra_modules: Vec<String> = Vec::new();
    let mut emit_ir = false;
    let mut emit_llvm = false;
    let mut native_out: Option<String> = None;
    let mut no_opt = false;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--module" => {
                if i + 1 >= args.len() {
                    eprintln!("Error: --module requires a file path");
                    process::exit(1);
                }
                extra_modules.push(args[i + 1].clone());
                i += 2;
            }
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
            "--emit-llvm" => {
                emit_llvm = true;
                i += 1;
            }
            "--native" => {
                if i + 1 >= args.len() {
                    eprintln!("Error: --native requires an output path");
                    process::exit(1);
                }
                native_out = Some(args[i + 1].clone());
                i += 2;
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

    // Compile the main module.
    let (main_bytecode, main_modules) = match compile_file(main_path, no_opt, !extra_modules.is_empty()) {
        Ok(result) => result,
        Err(errs) => {
            eprint!("{}", errs);
            process::exit(1);
        }
    };

    let output_path = filename.replace(".amb", ".amc");
    fs::write(&output_path, &main_bytecode).expect("Failed to write file");
    println!(
        "Amberlink: Compiled {} ({} modules) to {}",
        filename,
        main_modules.len(),
        output_path
    );

    // Compile extra --module files into separate .amc entries.
    let mut module_entries: Vec<(String, Vec<u8>)> =
        vec![(codegen::archive::MAIN_ENTRY.to_string(), main_bytecode)];
    for mod_path_str in &extra_modules {
        let mod_path = Path::new(mod_path_str);
        let (mod_bytecode, _) = match compile_file(mod_path, no_opt, true) {
            Ok(result) => result,
            Err(errs) => {
                eprint!("{}", errs);
                process::exit(1);
            }
        };
        let entry_name = mod_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("module")
            .to_string()
            .replace(".amb", ".amc");
        let entry_len = mod_bytecode.len();
        module_entries.push((entry_name, mod_bytecode));
        println!(
            "Amberlink: Compiled module {} ({} bytes) for archive",
            mod_path_str,
            entry_len
        );
    }

    // If requested, dump the backend IR and verify the decode/re-encode round-trip.
    if emit_ir {
        let (consts, code) = match codegen::ir::split_amc(&module_entries[0].1) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Error: could not read compiled image: {}", e);
                process::exit(1);
            }
        };
        match codegen::ir::decode(&code) {
            Ok(prog) => {
                let back = codegen::ir::encode(&prog);
                if back != code {
                    eprintln!("Error: IR round-trip mismatch (decoder drift)");
                    process::exit(1);
                }
                print!("{}", codegen::ir::format_program(&prog, &consts));
            }
            Err(e) => {
                eprintln!("Error: could not decode IR: {}", e);
                process::exit(1);
            }
        }
    }

    // If requested, emit LLVM IR for the compiled program (textual IR; no LLVM install needed).
    if emit_llvm || native_out.is_some() {
        let ll = lower_to_llvm(&module_entries[0].1);
        if emit_llvm {
            print!("{}", ll);
        }
        if let Some(exe) = &native_out {
            build_native(&ll, exe);
        }
    }

    // If requested, wrap the compiled modules plus bundled resources in a .ama archive.
    if let Some(archive_path) = archive_out {
        let module_count = module_entries.len();
        let resource_count = resources.len();
        let ama = codegen::archive::build_archive(module_entries, resources);
        fs::write(&archive_path, &ama).expect("Failed to write archive");
        println!(
            "Amberlink: Wrote archive {} ({} modules, {} resources)",
            archive_path,
            module_count,
            resource_count
        );
    }
}
