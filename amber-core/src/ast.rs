#[derive(Debug, Clone, PartialEq)]
pub enum Visibility {
    Public, Private, Protected
}

#[derive(Debug, Clone)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
    LessThan,
    GreaterThan,
    Equals,
    NotEquals,
    LessEquals,
    GreaterEquals,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Float,
    Bool,
    Char,
    String,
    Void,
    Class(String),
    List, // New Type
    Unknown, // Unresolved type (error recovery / inference fallback)
}

#[derive(Debug, Clone)]
pub enum Expr {
    Integer(i32),
    Float(f64),
    Char(char),
    Boolean(bool),
    StringLiteral(String),
    Variable(String),
    NewArray(Box<Expr>), // Size
    NewList, // New List
    ArrayAccess(String, Box<Expr>), // Name, Index
    Call(String, Vec<Expr>),
    Spawn(String, Vec<Expr>), // Function name, Args -> thread handle (int)
    MethodCall(Box<Expr>, String, Vec<Expr>), // Object, Method Name, Args
    NewInstance(String, Vec<Expr>), // Class Name, Args
    GetField(Box<Expr>, String), // Object Expr, Field Name
    Binary(Box<Expr>, Op, Box<Expr>),
    // List Operations
    ListGet(Box<Expr>, Box<Expr>), // List Expr, Index
    ListSize(Box<Expr>), // List Expr
    // Sentinel for a parse error; lets parsing continue past a bad expression.
    Error,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    VarDecl(String, Option<Type>, Expr),
    Assign(String, Expr),
    Return(Expr),
    ArraySet(String, Expr, Expr), // Name, Index, Value
    Print(Expr),
    Block(Vec<Stmt>),
    If(Expr, Box<Stmt>, Option<Box<Stmt>>), // Condition, Then, Else
    While(Expr, Box<Stmt>),                 // Condition, Body
    Expression(Expr),
    Function(String, Vec<(String, Type)>, Vec<Stmt>, Visibility, bool), // Name, Params, Body, Visibility, is_static
    Class(String, Option<String>, Vec<(String, Type, Visibility, bool)>, Vec<Stmt>, Vec<String>), // Name, Parent, Fields(name,type,vis,is_static), Methods, Implements
    FieldSet(Box<Expr>, String, Expr), // Object, Field Name, Value
    Interface(String, Vec<(String, Type, Vec<(String, Type)>)>), // Name, Method signatures (name, return_type, params)
    // Import a module file. Path is resolved by the compiler.
    Import(String),
    // List Operations
    ListAdd(Box<Expr>, Expr), // List Expr, Value
    ListSet(Box<Expr>, Box<Expr>, Expr), // List Expr, Index, Value
    // Sentinel for a parse error; lets parsing continue past a bad statement.
    Error,
}
