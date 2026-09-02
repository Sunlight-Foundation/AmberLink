// amber-core/src/parser.rs
use crate::lexer::{Token, SpannedToken};
use crate::semant::SymbolTable;
use crate::ast::{Stmt, Expr, Op, Type};
use crate::error::{ErrorList, CompileError};

pub struct Parser {
    tokens: Vec<SpannedToken>,
    pos: usize,
    errors: ErrorList,
}

impl Parser {
    pub fn new(tokens: Vec<SpannedToken>) -> Self {
        Self { tokens, pos: 0, errors: ErrorList::new() }
    }

    pub fn errors(&self) -> &ErrorList { &self.errors }
    pub fn into_errors(self) -> ErrorList { self.errors }

    // Record an error at the current token's position. Returns an Expr sentinel
    // so the caller can keep parsing (recovery) instead of aborting.
    fn error_expr(&mut self, msg: impl Into<String>) -> Expr {
        let (line, column) = if self.pos < self.tokens.len() {
            (self.tokens[self.pos].line, self.tokens[self.pos].column)
        } else {
            (0, 0)
        };
        self.errors.push(CompileError::new(line, column, msg));
        Expr::Error
    }

    // Same, but returns a Stmt sentinel.
    fn error_stmt(&mut self, msg: impl Into<String>) -> Stmt {
        let (line, column) = if self.pos < self.tokens.len() {
            (self.tokens[self.pos].line, self.tokens[self.pos].column)
        } else {
            (0, 0)
        };
        self.errors.push(CompileError::new(line, column, msg));
        Stmt::Error
    }

    // Panic-mode recovery: skip tokens until the next statement boundary
    // (newline, semicolon, '}', or EOF). Consumes one terminator if found.
    fn synchronize(&mut self) {
        while !self.is_at_end() {
            match self.peek() {
                Token::Newline => { self.advance(); break; }
                Token::Semicolon => { self.advance(); break; }
                Token::RBrace => { break; }
                _ => { self.advance(); }
            }
        }
    }

    // Tracks error count across a boundary; advances to next statement if it grew.
    pub fn parse(&mut self, symbols: &mut SymbolTable) -> Result<Vec<Stmt>, ErrorList> {
        let mut statements = Vec::new();
        while !self.is_at_end() {
            match self.peek() {
                Token::Newline => { self.advance(); }
                Token::Semicolon => { self.advance(); }
                _ => {
                    let before = self.errors.len();
                    let stmt = self.parse_statement(symbols);
                    if self.errors.len() > before {
                        // Statement had errors; recover at the next boundary.
                        self.synchronize();
                    } else {
                        statements.push(stmt);
                    }
                }
            }
        }
        if self.errors.has_errors() {
            Err(self.errors.clone())
        } else {
            Ok(statements)
        }
    }

    fn consume_semicolon(&mut self) {
        if self.peek() == Token::Semicolon {
            self.advance();
        }
    }

    fn parse_statement(&mut self, symbols: &mut SymbolTable) -> Stmt {
        match self.peek() {
            Token::Var => self.parse_declaration(None),
            Token::Int | Token::Void | Token::String | Token::Bool | Token::Float | Token::Char | Token::List => {
                if matches!(self.peek_n(1), Token::Identifier(_)) && self.peek_n(2) == Token::LParen {
                    self.parse_function(symbols)
                } else {
                    let explicit_type = self.parse_type();
                    self.parse_declaration(Some(explicit_type))
                }
            }
            Token::If => self.parse_if(symbols),
            Token::While => self.parse_while(symbols),
            Token::For => self.parse_for(symbols),
            Token::LBrace => self.parse_block(symbols),
            Token::Class => self.parse_class_decl(symbols),
            Token::Interface => self.parse_interface_decl(),
            Token::Return => self.parse_return(),
            Token::Print => self.parse_print(),
            Token::Identifier(_) => {
                let expr = self.parse_expr();

                if self.peek() == Token::Equals {
                    self.advance();
                    let value = self.parse_expr();
                    self.consume_semicolon();
                    match expr {
                        Expr::Variable(name) => Stmt::Assign(name, value),
                        Expr::ArrayAccess(name, index) => Stmt::ArraySet(name, *index, value),
                        Expr::GetField(obj, field) => Stmt::FieldSet(obj, field, value),
                        Expr::Error => Stmt::Error,
                        _ => self.error_stmt("Invalid assignment target. Only variables, array elements, and fields can be assigned."),
                    }
                } else {
                    self.consume_semicolon();
                    Stmt::Expression(expr)
                }
            }
            _ => {
                let expr = self.parse_expr();
                self.consume_semicolon();
                Stmt::Expression(expr)
            }
        }
    }

    fn parse_type(&mut self) -> Type {
        let tok = self.advance();
        match tok {
            Token::Int => Type::Int,
            Token::Float => Type::Float,
            Token::Bool => Type::Bool,
            Token::Char => Type::Char,
            Token::String => Type::String,
            Token::Void => Type::Void,
            Token::List => Type::List,
            Token::Identifier(name) => Type::Class(name),
            t => {
                self.errors.push(CompileError::new(
                    self.tokens[self.pos.min(self.tokens.len() - 1)].line,
                    self.tokens[self.pos.min(self.tokens.len() - 1)].column,
                    format!("Expected type, found {:?}", t),
                ));
                Type::Unknown
            }
        }
    }

    // --- Expression Parsing (Recursive Descent) ---

    fn parse_expr(&mut self) -> Expr {
        self.parse_comparison()
    }

    fn parse_comparison(&mut self) -> Expr {
        let mut expr = self.parse_term();
        while matches!(self.peek(), Token::LessThan | Token::GreaterThan | Token::DoubleEquals | Token::NotEquals | Token::LessEquals | Token::GreaterEquals) {
            let op = match self.advance() {
                Token::LessThan => Op::LessThan,
                Token::GreaterThan => Op::GreaterThan,
                Token::DoubleEquals => Op::Equals,
                Token::NotEquals => Op::NotEquals,
                Token::LessEquals => Op::LessEquals,
                Token::GreaterEquals => Op::GreaterEquals,
                _ => unreachable!(),
            };
            let right = self.parse_term();
            expr = Expr::Binary(Box::new(expr), op, Box::new(right));
        }
        expr
    }

    fn parse_term(&mut self) -> Expr {
        let mut expr = self.parse_factor();
        while matches!(self.peek(), Token::Plus | Token::Minus) {
            let op = match self.advance() {
                Token::Plus => Op::Add,
                Token::Minus => Op::Sub,
                _ => unreachable!(),
            };
            let right = self.parse_factor();
            expr = Expr::Binary(Box::new(expr), op, Box::new(right));
        }
        expr
    }

    fn parse_factor(&mut self) -> Expr {
        let mut expr = self.parse_primary();
        while matches!(self.peek(), Token::Star | Token::Slash) {
            let op = match self.advance() {
                Token::Star => Op::Mul,
                Token::Slash => Op::Div,
                _ => unreachable!(),
            };
            let right = self.parse_primary();
            expr = Expr::Binary(Box::new(expr), op, Box::new(right));
        }
        expr
    }

    fn parse_primary(&mut self) -> Expr {
        let tok = self.advance();
        match tok {
            Token::Number(val) => Expr::Integer(val as i32),
            Token::FloatLit(val) => {
                match val.parse() {
                    Ok(f) => Expr::Float(f),
                    Err(_) => self.error_expr(format!("Invalid float literal '{}'.", val)),
                }
            }
            Token::CharLit(val) => Expr::Char(val),
            Token::True => Expr::Boolean(true),
            Token::False => Expr::Boolean(false),
            Token::LParen => {
                let expr = self.parse_expr();
                if self.peek() != Token::RParen {
                    return self.error_expr("Expected ')' after expression");
                }
                self.advance();
                expr
            },
            Token::New => {
                let type_token = self.advance();
                match type_token {
                    Token::Int | Token::String | Token::Bool | Token::Float | Token::Char => {
                        if self.peek() != Token::LBracket { return self.error_expr("Expected '[' after type"); }
                        self.advance();
                        let size = self.parse_expr();
                        if self.peek() != Token::RBracket { return self.error_expr("Expected ']' after size"); }
                        self.advance();
                        Expr::NewArray(Box::new(size))
                    },
                    Token::List => {
                        if self.peek() != Token::LParen { return self.error_expr("Expected '(' after List"); }
                        self.advance();
                        if self.peek() != Token::RParen { return self.error_expr("Expected ')' after List"); }
                        self.advance();
                        Expr::NewList
                    },
                    Token::Identifier(name) => {
                        if self.peek() != Token::LParen { return self.error_expr("Expected '(' after class name"); }
                        self.advance();

                        let mut args = Vec::new();
                        if self.peek() != Token::RParen {
                            loop {
                                args.push(self.parse_expr());
                                if self.peek() == Token::Comma { self.advance(); } else { break; }
                            }
                        }

                        if self.peek() != Token::RParen { return self.error_expr("Expected ')' after arguments"); }
                        self.advance();
                        Expr::NewInstance(name, args)
                    },
                    _ => self.error_expr("Expected type or class name after 'new'"),
                }
            }
            Token::StringLit(s) => Expr::StringLiteral(s),
            Token::Identifier(name) => {
                if self.peek() == Token::LParen {
                    self.advance();
                    let mut args = Vec::new();
                    if self.peek() != Token::RParen {
                        loop {
                            args.push(self.parse_expr());
                            if self.peek() == Token::Comma { self.advance(); } else { break; }
                        }
                    }
                    if self.peek() != Token::RParen {
                        return self.error_expr("Expected ')' after arguments");
                    }
                    self.advance();
                    Expr::Call(name, args)
                } else if self.peek() == Token::LBracket {
                    self.advance();
                    let index = self.parse_expr();
                    if self.peek() != Token::RBracket { return self.error_expr("Expected ']'"); }
                    self.advance();
                    Expr::ArrayAccess(name, Box::new(index))
                } else if self.peek() == Token::Dot {
                    self.advance();
                    let member = match self.advance() { Token::Identifier(f) => f, _ => return self.error_expr("Expected member name") };

                    if self.peek() == Token::LParen {
                        self.advance();
                        let mut args = Vec::new();
                        if self.peek() != Token::RParen {
                            loop {
                                args.push(self.parse_expr());
                                if self.peek() == Token::Comma { self.advance(); } else { break; }
                            }
                        }
                        if self.peek() != Token::RParen { return self.error_expr("Expected ')' after arguments"); }
                        self.advance();

                        if member == "get" && args.len() == 1 {
                            Expr::ListGet(Box::new(Expr::Variable(name)), Box::new(args[0].clone()))
                        } else if member == "size" && args.is_empty() {
                            Expr::ListSize(Box::new(Expr::Variable(name)))
                        } else {
                            Expr::MethodCall(Box::new(Expr::Variable(name)), member, args)
                        }
                    } else {
                        Expr::GetField(Box::new(Expr::Variable(name)), member)
                    }
                } else {
                    Expr::Variable(name)
                }
            }
            tok => self.error_expr(format!("Unexpected token: {:?}. Expected a number, identifier, or expression.", tok)),
        }
    }

    fn parse_function(&mut self, symbols: &mut SymbolTable) -> Stmt {
        let return_type = self.parse_type();

        let name = match self.advance() {
            Token::Identifier(n) => n,
            t => return self.error_stmt(format!("Expected function name, found {:?}", t)),
        };

        if self.peek() != Token::LParen { return self.error_stmt("Expected '(' after function name"); }
        self.advance();
        let mut params: Vec<(String, Type)> = Vec::new();
        if self.peek() != Token::RParen {
            loop {
                let param_type = self.parse_type();
                match self.advance() {
                    Token::Identifier(param_name) => params.push((param_name, param_type)),
                    _ => return self.error_stmt("Expected parameter name"),
                }
                if self.peek() == Token::Comma { self.advance(); } else { break; }
            }
        }
        if self.peek() != Token::RParen { return self.error_stmt("Expected ')' after parameters"); }
        self.advance();

        symbols.functions.insert(name.clone(), crate::semant::FunctionInfo {
            name: name.clone(),
            address: 0,
            return_type: return_type.clone(),
        });

        let old_locals = symbols.locals.clone();
        let old_local_index = symbols.next_local_index;
        symbols.locals.clear();
        symbols.next_local_index = 0;

        for (param_name, _) in &params {
            symbols.locals.insert(param_name.clone(), symbols.next_local_index);
            symbols.next_local_index += 1;
        }

        let body_stmt = self.parse_block(symbols);
        let body = match body_stmt { Stmt::Block(stmts) => stmts, _ => vec![] };

        symbols.locals = old_locals;
        symbols.next_local_index = old_local_index;

        Stmt::Function(name, params, body, crate::ast::Visibility::Public, false)
    }

    fn parse_class_decl(&mut self, symbols: &mut SymbolTable) -> Stmt {
        self.advance(); // consume 'class'
        let name = match self.advance() {
            Token::Identifier(n) => n,
            _ => return self.error_stmt("Expected class name"),
        };

        let parent = if self.peek() == Token::Extends {
            self.advance();
            match self.advance() {
                Token::Identifier(p) => Some(p),
                _ => return self.error_stmt("Expected parent class name after 'extends'"),
            }
        } else {
            None
        };

        let mut implements = Vec::new();
        if self.peek() == Token::Implements {
            self.advance();
            loop {
                match self.advance() {
                    Token::Identifier(iface) => implements.push(iface),
                    _ => return self.error_stmt("Expected interface name after 'implements'"),
                }
                if self.peek() == Token::Comma { self.advance(); } else { break; }
            }
        }

        if self.peek() != Token::LBrace { return self.error_stmt("Expected '{' after class name"); }
        self.advance();

        let mut fields = Vec::new();
        let mut methods = Vec::new();

        while !self.is_at_end() && self.peek() != Token::RBrace {
            if self.peek() == Token::Newline { self.advance(); continue; }

            let mut vis = crate::ast::Visibility::Public;
            if matches!(self.peek(), Token::Public | Token::Private | Token::Protected) {
                vis = match self.advance() {
                    Token::Public => crate::ast::Visibility::Public,
                    Token::Private => crate::ast::Visibility::Private,
                    Token::Protected => crate::ast::Visibility::Protected,
                    _ => unreachable!(),
                };
            }

            let is_static = if self.peek() == Token::Static {
                self.advance();
                true
            } else {
                false
            };

            if self.peek() == Token::Init {
                methods.push(self.parse_constructor(symbols, &name, vis));
            } else if matches!(self.peek_n(1), Token::Identifier(_)) && self.peek_n(2) == Token::LParen {
                methods.push(self.parse_method(symbols, &name, vis, is_static));
            } else {
                let field_type = self.parse_type();
                let field_name = match self.advance() { Token::Identifier(n) => n, _ => return self.error_stmt("Expected field name") };
                self.consume_semicolon();
                fields.push((field_name, field_type, vis, is_static));
            }
        }
        if self.peek() != Token::RBrace { return self.error_stmt("Expected '}' after class body"); }
        self.advance();

        Stmt::Class(name, parent, fields, methods, implements)
    }

    fn parse_method(&mut self, symbols: &mut SymbolTable, class_name: &str, vis: crate::ast::Visibility, is_static: bool) -> Stmt {
        let return_type = self.parse_type();

        let method_name = match self.advance() { Token::Identifier(n) => n, _ => return self.error_stmt("Expected method name") };

        if self.peek() != Token::LParen { return self.error_stmt("Expected '(' after method name"); }
        self.advance();
        let mut params: Vec<(String, Type)> = Vec::new();

        if self.peek() != Token::RParen {
            loop {
                let param_type = self.parse_type();
                match self.advance() {
                    Token::Identifier(param_name) => params.push((param_name, param_type)),
                    _ => return self.error_stmt("Expected parameter name"),
                }
                if self.peek() == Token::Comma { self.advance(); } else { break; }
            }
        }
        if self.peek() != Token::RParen { return self.error_stmt("Expected ')' after parameters"); }
        self.advance();

        let type_suffix: String = params.iter()
            .map(|(_, t)| format!("{}", match t {
                Type::Int => "int",
                Type::Float => "float",
                Type::Bool => "bool",
                Type::Char => "char",
                Type::String => "String",
                Type::Void => "void",
                Type::List => "List",
                Type::Class(name) => name.as_str(),
                Type::Unknown => "unknown",
            }))
            .collect::<Vec<_>>()
            .join("_");
        let full_name = if type_suffix.is_empty() {
            format!("{}_{}", class_name, method_name)
        } else {
            format!("{}_{}_{}", class_name, method_name, type_suffix)
        };

        symbols.functions.insert(full_name.clone(), crate::semant::FunctionInfo {
            name: full_name.clone(),
            address: 0,
            return_type,
        });

        let old_locals = symbols.locals.clone();
        let old_local_index = symbols.next_local_index;
        symbols.locals.clear();
        symbols.next_local_index = 0;

        if !is_static {
            symbols.locals.insert("this".to_string(), symbols.next_local_index);
            symbols.next_local_index += 1;
        }

        for (param_name, _) in &params {
            symbols.locals.insert(param_name.clone(), symbols.next_local_index);
            symbols.next_local_index += 1;
        }

        let body_stmt = self.parse_block(symbols);
        let body = match body_stmt { Stmt::Block(stmts) => stmts, _ => vec![] };

        symbols.locals = old_locals;
        symbols.next_local_index = old_local_index;

        if !is_static {
            params.insert(0, ("this".to_string(), Type::Class(class_name.to_string())));
        }

        Stmt::Function(full_name, params, body, vis, is_static)
    }

    fn parse_constructor(&mut self, symbols: &mut SymbolTable, class_name: &str, vis: crate::ast::Visibility) -> Stmt {
        self.advance(); // consume 'init'

        let full_name = format!("{}_init", class_name);

        if self.peek() != Token::LParen { return self.error_stmt("Expected '(' after init"); }
        self.advance();
        let mut params: Vec<(String, Type)> = Vec::new();

        if self.peek() != Token::RParen {
            loop {
                let param_type = self.parse_type();
                match self.advance() {
                    Token::Identifier(param_name) => params.push((param_name, param_type)),
                    _ => return self.error_stmt("Expected parameter name"),
                }
                if self.peek() == Token::Comma { self.advance(); } else { break; }
            }
        }
        if self.peek() != Token::RParen { return self.error_stmt("Expected ')' after parameters"); }
        self.advance();

        symbols.functions.insert(full_name.clone(), crate::semant::FunctionInfo {
            name: full_name.clone(),
            address: 0,
            return_type: Type::Void,
        });

        let old_locals = symbols.locals.clone();
        let old_local_index = symbols.next_local_index;
        symbols.locals.clear();
        symbols.next_local_index = 0;

        symbols.locals.insert("this".to_string(), symbols.next_local_index);
        symbols.next_local_index += 1;

        for (param_name, _) in &params {
            symbols.locals.insert(param_name.clone(), symbols.next_local_index);
            symbols.next_local_index += 1;
        }

        let body_stmt = self.parse_block(symbols);
        let mut body = match body_stmt { Stmt::Block(stmts) => stmts, _ => vec![] };

        body.push(Stmt::Return(Expr::Variable("this".to_string())));

        symbols.locals = old_locals;
        symbols.next_local_index = old_local_index;

        params.insert(0, ("this".to_string(), Type::Class(class_name.to_string())));

        Stmt::Function(full_name, params, body, vis, false)
    }

    fn parse_interface_decl(&mut self) -> Stmt {
        self.advance(); // consume 'interface'
        let name = match self.advance() {
            Token::Identifier(n) => n,
            _ => return self.error_stmt("Expected interface name"),
        };

        if self.peek() != Token::LBrace { return self.error_stmt("Expected '{' after interface name"); }
        self.advance();

        let mut signatures = Vec::new();
        while !self.is_at_end() && self.peek() != Token::RBrace {
            if self.peek() == Token::Newline { self.advance(); continue; }

            let return_type = self.parse_type();
            let method_name = match self.advance() {
                Token::Identifier(n) => n,
                _ => return self.error_stmt("Expected method name in interface"),
            };

            if self.peek() != Token::LParen { return self.error_stmt("Expected '(' after method name in interface"); }
            self.advance();
            let mut params: Vec<(String, Type)> = Vec::new();
            if self.peek() != Token::RParen {
                loop {
                    let param_type = self.parse_type();
                    match self.advance() {
                        Token::Identifier(param_name) => params.push((param_name, param_type)),
                        _ => return self.error_stmt("Expected parameter name in interface method"),
                    }
                    if self.peek() == Token::Comma { self.advance(); } else { break; }
                }
            }
            if self.peek() != Token::RParen { return self.error_stmt("Expected ')' after interface method params"); }
            self.advance();
            self.consume_semicolon();

            signatures.push((method_name, return_type, params));
        }
        if self.peek() != Token::RBrace { return self.error_stmt("Expected '}' after interface body"); }
        self.advance();

        Stmt::Interface(name, signatures)
    }

    fn parse_declaration(&mut self, explicit_type: Option<Type>) -> Stmt {
        if explicit_type.is_none() {
            self.advance(); // consume 'var'
        }

        let name = match self.advance() {
            Token::Identifier(n) => n,
            _ => return self.error_stmt("Expected variable name"),
        };

        if self.peek() != Token::Equals { return self.error_stmt("Expected '=' after variable name"); }
        self.advance();

        let initializer = self.parse_expr();
        self.consume_semicolon();
        Stmt::VarDecl(name, explicit_type, initializer)
    }

    fn parse_block(&mut self, symbols: &mut SymbolTable) -> Stmt {
        self.advance(); // skip '{'
        let mut statements = Vec::new();

        while !self.is_at_end() && self.peek() != Token::RBrace {
            if self.peek() == Token::Newline { self.advance(); continue; }
            let before = self.errors.len();
            let stmt = self.parse_statement(symbols);
            if self.errors.len() > before {
                self.synchronize();
            } else {
                statements.push(stmt);
            }
        }

        if self.peek() == Token::RBrace {
            self.advance();
        } else {
            return self.error_stmt("Expected '}' after block");
        }

        Stmt::Block(statements)
    }

    fn parse_if(&mut self, symbols: &mut SymbolTable) -> Stmt {
        self.advance(); // skip 'if'
        let condition = self.parse_expr();
        let then_branch = Box::new(self.parse_statement(symbols));
        let mut else_branch = None;

        while self.peek() == Token::Newline {
            self.advance();
        }

        if self.peek() == Token::Else {
            self.advance();
            else_branch = Some(Box::new(self.parse_statement(symbols)));
        }

        Stmt::If(condition, then_branch, else_branch)
    }

    fn parse_while(&mut self, symbols: &mut SymbolTable) -> Stmt {
        self.advance(); // skip 'while'
        let condition = self.parse_expr();
        let body = Box::new(self.parse_statement(symbols));
        Stmt::While(condition, body)
    }

    fn parse_for(&mut self, symbols: &mut SymbolTable) -> Stmt {
        self.advance(); // skip 'for'
        if self.peek() != Token::LParen { return self.error_stmt("Expected '(' after 'for'"); }
        self.advance();

        let initializer = if self.peek() == Token::Semicolon {
            self.advance();
            None
        } else if matches!(self.peek(), Token::Var | Token::Int | Token::String | Token::Bool | Token::Float | Token::Char) {
            if self.peek() == Token::Var {
                Some(Box::new(self.parse_declaration(None)))
            } else {
                let explicit_type = self.parse_type();
                Some(Box::new(self.parse_declaration(Some(explicit_type))))
            }
        } else {
            Some(Box::new(Stmt::Expression(self.parse_expr())))
        };

        if initializer.is_some() && self.peek() == Token::Semicolon {
             self.advance();
        }

        let condition = if self.peek() == Token::Semicolon {
            Expr::Boolean(true)
        } else {
            self.parse_expr()
        };
        if self.peek() != Token::Semicolon { return self.error_stmt("Expected ';' after loop condition"); }
        self.advance();

        let increment = if self.peek() == Token::RParen {
            None
        } else {
             let expr = self.parse_expr();
             if self.peek() == Token::Equals {
                self.advance();
                let value = self.parse_expr();
                match expr {
                    Expr::Variable(name) => Some(Box::new(Stmt::Assign(name, value))),
                    Expr::ArrayAccess(name, index) => Some(Box::new(Stmt::ArraySet(name, *index, value))),
                    Expr::GetField(obj, field) => Some(Box::new(Stmt::FieldSet(obj, field, value))),
                    _ => return self.error_stmt("Invalid assignment target in for loop increment."),
                }
            } else {
                Some(Box::new(Stmt::Expression(expr)))
            }
        };

        if self.peek() != Token::RParen { return self.error_stmt("Expected ')' after for clauses"); }
        self.advance();

        let body = self.parse_statement(symbols);

        let mut while_body_stmts = Vec::new();
        while_body_stmts.push(body);
        if let Some(inc) = increment {
            while_body_stmts.push(*inc);
        }

        let while_loop = Stmt::While(condition, Box::new(Stmt::Block(while_body_stmts)));

        let mut outer_stmts = Vec::new();
        if let Some(init) = initializer {
            outer_stmts.push(*init);
        }
        outer_stmts.push(while_loop);

        Stmt::Block(outer_stmts)
    }

    fn parse_return(&mut self) -> Stmt {
        self.advance(); // skip 'return'
        let value = self.parse_expr();
        self.consume_semicolon();
        Stmt::Return(value)
    }

    fn parse_print(&mut self) -> Stmt {
        self.advance(); // skip 'print'
        let expr = self.parse_expr();
        self.consume_semicolon();
        Stmt::Print(expr)
    }

    fn peek(&self) -> Token { self.tokens[self.pos].token.clone() }
    fn advance(&mut self) -> Token {
        let tok = self.tokens[self.pos].token.clone();
        if !self.is_at_end() { self.pos += 1; }
        tok
    }
    fn peek_n(&self, n: usize) -> Token {
        if self.pos + n >= self.tokens.len() { return Token::EOF; }
        self.tokens[self.pos + n].token.clone()
    }
    fn is_at_end(&self) -> bool { self.peek() == Token::EOF }
}
