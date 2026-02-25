// amber-core/src/parser.rs
use crate::lexer::Token;
use crate::semant::SymbolTable;
use crate::ast::{Stmt, Expr, Op, Type};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    pub fn parse(&mut self, symbols: &mut SymbolTable) -> Vec<Stmt> {
        let mut statements = Vec::new();
        while !self.is_at_end() {
            match self.peek() {
                // Skip empty lines between top-level statements
                Token::Newline => { self.advance(); }
                _ => statements.push(self.parse_statement(symbols)),
            }
        }
        statements
    }

    fn parse_statement(&mut self, symbols: &mut SymbolTable) -> Stmt {
        match self.peek() {
            Token::Var => self.parse_declaration(None),
            Token::Int | Token::Void | Token::String | Token::Bool | Token::Float | Token::Char => {
                // Lookahead to distinguish Variable Declaration vs Function Definition
                // int x = 5;       (Type -> Identifier -> Equals)
                // int x() { ... }  (Type -> Identifier -> LParen)
                if matches!(self.peek_n(1), Token::Identifier(_)) && self.peek_n(2) == Token::LParen {
                    self.parse_function(symbols)
                } else {
                    let _type_tok = self.peek();
                    let explicit_type = self.parse_type();
                    self.parse_declaration(Some(explicit_type))
                }
            }
            Token::If => self.parse_if(symbols),
            Token::While => self.parse_while(symbols),
            Token::For => self.parse_for(symbols),
            Token::LBrace => self.parse_block(symbols),
            Token::Class => self.parse_class_decl(symbols),
            Token::Return => self.parse_return(),
            Token::Print => self.parse_print(),
            // Token::Func is deprecated in favor of C-style types
            Token::Identifier(_) => {
                // Parse as expression first to handle L-values (Variable or ArrayAccess)
                let expr = self.parse_expr();
                
                if self.peek() == Token::Equals {
                    self.advance(); // consume '='
                    let value = self.parse_expr();
                    match expr {
                        Expr::Variable(name) => Stmt::Assign(name, value),
                        Expr::ArrayAccess(name, index) => Stmt::ArraySet(name, *index, value),
                        Expr::GetField(obj, field) => Stmt::FieldSet(obj, field, value),
                        _ => panic!("Invalid assignment target. Only variables, array elements, and fields can be assigned."),
                    }
                } else {
                    Stmt::Expression(expr)
                }
            }
            _ => Stmt::Expression(self.parse_expr()),
        }
    }

    fn parse_type(&mut self) -> Type {
        match self.advance() {
            Token::Int => Type::Int,
            Token::Float => Type::Float,
            Token::Bool => Type::Bool,
            Token::Char => Type::Char,
            Token::String => Type::String,
            Token::Void => Type::Void,
            Token::Identifier(name) => Type::Class(name),
            // TODO: Array types like int[]
            t => panic!("Expected type, found {:?}", t),
        }
    }

    // --- Expression Parsing (Recursive Descent) ---

    fn parse_expr(&mut self) -> Expr {
        self.parse_comparison()
    }

    fn parse_comparison(&mut self) -> Expr {
        let mut expr = self.parse_term();
        while matches!(self.peek(), Token::LessThan) {
            self.advance(); // consume '<'
            let right = self.parse_term();
            expr = Expr::Binary(Box::new(expr), Op::LessThan, Box::new(right));
        }
        expr
    }

    // Handles + and -
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

    // Handles * and /
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
        match self.advance() {
            Token::Number(val) => Expr::Integer(val as i32),
            Token::FloatLit(val) => Expr::Float(val.parse().unwrap()),
            Token::CharLit(val) => Expr::Char(val),
            Token::True => Expr::Boolean(true),
            Token::False => Expr::Boolean(false),
            Token::New => {
                // new int[size] OR new MyClass()
                let type_token = self.advance();
                match type_token {
                    Token::Int | Token::String | Token::Bool | Token::Float | Token::Char => {
                        if self.advance() != Token::LBracket { panic!("Expected '[' after type"); }
                        let size = self.parse_expr();
                        if self.advance() != Token::RBracket { panic!("Expected ']' after size"); }
                        Expr::NewArray(Box::new(size))
                    },
                    Token::Identifier(name) => {
                        if self.advance() != Token::LParen { panic!("Expected '(' after class name"); }

                        let mut args = Vec::new();
                        if self.peek() != Token::RParen {
                            loop {
                                args.push(self.parse_expr());
                                if self.peek() == Token::Comma { self.advance(); } else { break; }
                            }
                        }

                        if self.advance() != Token::RParen { panic!("Expected ')' after arguments"); }
                        Expr::NewInstance(name, args)
                    },
                    _ => panic!("Expected type or class name after 'new'"),
                }
            }
            Token::StringLit(s) => Expr::StringLiteral(s),
            Token::Identifier(name) => {
                if self.peek() == Token::LParen {
                    self.advance(); // skip '('
                    let mut args = Vec::new();
                    if self.peek() != Token::RParen {
                        loop {
                            args.push(self.parse_expr());
                            if self.peek() == Token::Comma { self.advance(); } else { break; }
                        }
                    }
                    if self.advance() != Token::RParen {
                        panic!("Expected ')' after arguments");
                    }
                    Expr::Call(name, args)
                } else if self.peek() == Token::LBracket {
                    self.advance(); // [
                    let index = self.parse_expr();
                    if self.advance() != Token::RBracket { panic!("Expected ']'"); }
                    Expr::ArrayAccess(name, Box::new(index))
                } else if self.peek() == Token::Dot {
                    self.advance(); // consume '.'
                    let member = match self.advance() { Token::Identifier(f) => f, _ => panic!("Expected member name") };
                    
                    if self.peek() == Token::LParen {
                        self.advance(); // consume '('
                        let mut args = Vec::new();
                        if self.peek() != Token::RParen {
                            loop {
                                args.push(self.parse_expr());
                                if self.peek() == Token::Comma { self.advance(); } else { break; }
                            }
                        }
                        if self.advance() != Token::RParen { panic!("Expected ')' after arguments"); }
                        Expr::MethodCall(Box::new(Expr::Variable(name)), member, args)
                    } else {
                        Expr::GetField(Box::new(Expr::Variable(name)), member)
                    }
                } else {
                    Expr::Variable(name)
                }
            }
            tok => panic!(
                "Unexpected token in expression: {:?}. Expected a number or identifier.",
                tok
            ),
        }
    }

    fn parse_function(&mut self, symbols: &mut SymbolTable) -> Stmt {
        let _return_type = self.parse_type();

        let name_token = self.advance();
        let name = match name_token {
            Token::Identifier(n) => n,
            _ => panic!("Expected function name, found {:?}", name_token),
        };

        // Parse Parameters
        if self.advance() != Token::LParen { panic!("Expected '(' after function name"); }
        let mut params: Vec<(String, Type)> = Vec::new();
        if self.peek() != Token::RParen {
            loop {
                let param_type = self.parse_type();
                match self.advance() {
                    Token::Identifier(param_name) => params.push((param_name, param_type)),
                    _ => panic!("Expected parameter name"),
                }
                if self.peek() == Token::Comma { self.advance(); } else { break; }
            }
        }
        if self.advance() != Token::RParen { panic!("Expected ')' after parameters"); }

        // Register function in the symbol table (Pass 1: Discovery)
        symbols.functions.insert(name.clone(), crate::semant::FunctionInfo {
            name: name.clone(),
            address: 0, // Placeholder: Will be resolved during emission
        });

        // Setup scope for function body
        let old_locals = symbols.locals.clone();
        let old_local_index = symbols.next_local_index;
        symbols.locals.clear();
        symbols.next_local_index = 0;

        // Register parameters as locals
        for (param_name, _) in &params {
            symbols.locals.insert(param_name.clone(), symbols.next_local_index);
            symbols.next_local_index += 1;
        }

        // Parse Body
        let body_stmt = self.parse_block(symbols);
        let body = match body_stmt { Stmt::Block(stmts) => stmts, _ => vec![] };

        // Restore scope
        symbols.locals = old_locals;
        symbols.next_local_index = old_local_index;

        Stmt::Function(name, params, body)
    }

    fn parse_class_decl(&mut self, symbols: &mut SymbolTable) -> Stmt {
        self.advance(); // consume 'class'
        let name = match self.advance() {
            Token::Identifier(n) => n,
            _ => panic!("Expected class name"),
        };

        if self.advance() != Token::LBrace { panic!("Expected '{{' after class name"); }

        let mut fields = Vec::new();
        let mut methods = Vec::new();

        while !self.is_at_end() && self.peek() != Token::RBrace {
            if self.peek() == Token::Newline { self.advance(); continue; }
            
            // Lookahead: Type -> Name. If next is '(', it's a method. Else field.
            if matches!(self.peek_n(1), Token::Identifier(_)) && self.peek_n(2) == Token::LParen {
                methods.push(self.parse_method(symbols, &name));
            } else {
                // Parse field
                let field_type = self.parse_type();
                let field_name = match self.advance() { Token::Identifier(n) => n, _ => panic!("Expected field name") };
                fields.push((field_name, field_type));
            }
        }
        if self.advance() != Token::RBrace { panic!("Expected '}}' after class body"); }

        Stmt::Class(name, fields, methods)
    }

    fn parse_method(&mut self, symbols: &mut SymbolTable, class_name: &str) -> Stmt {
        let _return_type = self.parse_type();
        
        let name_token = self.advance();
        let method_name = match name_token { Token::Identifier(n) => n, _ => panic!("Expected method name") };
        
        // Mangle name: Class_Method
        let full_name = format!("{}_{}", class_name, method_name);

        // Parse Parameters
        if self.advance() != Token::LParen { panic!("Expected '(' after method name"); }
        let mut params: Vec<(String, Type)> = Vec::new();
        
        if self.peek() != Token::RParen {
            loop {
                let param_type = self.parse_type();
                match self.advance() {
                    Token::Identifier(param_name) => params.push((param_name, param_type)),
                    _ => panic!("Expected parameter name"),
                }
                if self.peek() == Token::Comma { self.advance(); } else { break; }
            }
        }
        if self.advance() != Token::RParen { panic!("Expected ')' after parameters"); }

        // Register function
        symbols.functions.insert(full_name.clone(), crate::semant::FunctionInfo {
            name: full_name.clone(),
            address: 0,
        });

        // Setup Scope
        let old_locals = symbols.locals.clone();
        let old_local_index = symbols.next_local_index;
        symbols.locals.clear();
        symbols.next_local_index = 0;

        // 1. Inject 'this' as the first local variable (index 0)
        symbols.locals.insert("this".to_string(), symbols.next_local_index);
        symbols.next_local_index += 1;

        // 2. Register other parameters
        for (param_name, _) in &params {
            symbols.locals.insert(param_name.clone(), symbols.next_local_index);
            symbols.next_local_index += 1;
        }

        let body_stmt = self.parse_block(symbols);
        let body = match body_stmt { Stmt::Block(stmts) => stmts, _ => vec![] };

        symbols.locals = old_locals;
        symbols.next_local_index = old_local_index;

        // Prepend 'this' to params for the AST so the Emitter knows it's a local variable
        params.insert(0, ("this".to_string(), Type::Class(class_name.to_string())));

        Stmt::Function(full_name, params, body)
    }

    fn parse_declaration(&mut self, explicit_type: Option<Type>) -> Stmt {
        if explicit_type.is_none() {
            self.advance(); // consume 'var'
        }
        
        let name = match self.advance() {
            Token::Identifier(n) => n,
            _ => panic!("Expected variable name"),
        };

        if self.advance() != Token::Equals { panic!("Expected '=' after variable name"); }
        
        let initializer = self.parse_expr();
        Stmt::VarDecl(name, explicit_type, initializer)
    }

    fn parse_block(&mut self, symbols: &mut SymbolTable) -> Stmt {
        self.advance(); // skip '{'
        let mut statements = Vec::new();
        
        while !self.is_at_end() && self.peek() != Token::RBrace {
            if self.peek() == Token::Newline { self.advance(); continue; }
            statements.push(self.parse_statement(symbols));
        }

        if self.peek() == Token::RBrace {
            self.advance(); // skip '}'
        } else {
            panic!("Expected '}}' after block");
        }
        
        Stmt::Block(statements)
    }

    fn parse_if(&mut self, symbols: &mut SymbolTable) -> Stmt {
        self.advance(); // skip 'if'
        let condition = self.parse_expr();
        let then_branch = Box::new(self.parse_statement(symbols));
        let mut else_branch = None;

        // Consume newlines looking for else
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
        if self.advance() != Token::LParen { panic!("Expected '(' after 'for'"); }

        // Initializer
        let initializer = if self.peek() == Token::Semicolon {
            self.advance();
            None
        } else if matches!(self.peek(), Token::Var | Token::Int | Token::String | Token::Bool | Token::Float | Token::Char) {
            // We need to handle type parsing here too if it's not 'var'
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

        // Condition
        let condition = if self.peek() == Token::Semicolon {
            Expr::Boolean(true) // Default true
        } else {
            self.parse_expr()
        };
        if self.advance() != Token::Semicolon { panic!("Expected ';' after loop condition"); }

        // Increment
        let increment = if self.peek() == Token::RParen {
            None
        } else {
            // Parse assignment or expression
             let expr = self.parse_expr();
             if self.peek() == Token::Equals {
                self.advance(); // consume '='
                let value = self.parse_expr();
                match expr {
                    Expr::Variable(name) => Some(Box::new(Stmt::Assign(name, value))),
                    Expr::ArrayAccess(name, index) => Some(Box::new(Stmt::ArraySet(name, *index, value))),
                    Expr::GetField(obj, field) => Some(Box::new(Stmt::FieldSet(obj, field, value))),
                    _ => panic!("Invalid assignment target in for loop increment."),
                }
            } else {
                Some(Box::new(Stmt::Expression(expr)))
            }
        };

        if self.advance() != Token::RParen { panic!("Expected ')' after for clauses"); }

        let body = self.parse_statement(symbols);

        // Desugar to while loop:
        // {
        //   initializer;
        //   while (condition) {
        //     body;
        //     increment;
        //   }
        // }

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
        Stmt::Return(value)
    }

    fn parse_print(&mut self) -> Stmt {
        self.advance(); // skip 'print'
        let expr = self.parse_expr();
        Stmt::Print(expr)
    }

    fn peek(&self) -> Token { self.tokens[self.pos].clone() }
    fn advance(&mut self) -> Token { 
        let tok = self.tokens[self.pos].clone();
        if !self.is_at_end() { self.pos += 1; }
        tok
    }
    fn peek_n(&self, n: usize) -> Token {
        if self.pos + n >= self.tokens.len() { return Token::EOF; }
        self.tokens[self.pos + n].clone()
    }
    fn is_at_end(&self) -> bool { self.peek() == Token::EOF }
}
