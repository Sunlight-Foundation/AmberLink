#[derive(Debug, Clone)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
    LessThan,
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
    ArrayAccess(String, Box<Expr>), // Name, Index
    Call(String, Vec<Expr>),
    MethodCall(Box<Expr>, String, Vec<Expr>), // Object, Method Name, Args
    NewInstance(String, Vec<Expr>), // Class Name, Args
    GetField(Box<Expr>, String), // Object Expr, Field Name
    Binary(Box<Expr>, Op, Box<Expr>),
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
    Function(String, Vec<(String, Type)>, Vec<Stmt>), // Name, Params, Body
    Class(String, Vec<(String, Type)>, Vec<Stmt>), // Name, Fields, Methods
    FieldSet(Box<Expr>, String, Expr), // Object, Field Name, Value
}
