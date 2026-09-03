// amber-core/src/error.rs
// Compile-time error reporting. Replaces panic!-based reporting so the
// compiler can accumulate multiple errors and never crash on bad input.

use std::fmt;

#[derive(Debug, Clone)]
pub struct CompileError {
    pub line: usize,
    pub column: usize,
    pub message: String,
}

impl CompileError {
    pub fn new(line: usize, column: usize, message: impl Into<String>) -> Self {
        Self { line, column, message: message.into() }
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error[{}:{}]: {}", self.line, self.column, self.message)
    }
}

#[derive(Debug, Default, Clone)]
pub struct ErrorList {
    pub errors: Vec<CompileError>,
}

impl ErrorList {
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    pub fn push(&mut self, err: CompileError) {
        self.errors.push(err);
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn len(&self) -> usize {
        self.errors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn extend(&mut self, other: ErrorList) {
        self.errors.extend(other.errors);
    }
}

impl fmt::Display for ErrorList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for e in &self.errors {
            writeln!(f, "{}", e)?;
        }
        Ok(())
    }
}
