// amber-core/src/lexer.rs

#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    Var, Mut, Func, Class, Return, Print,
    Int, Void, String, Bool, Float, Char, List, // Types
    New, Init,
    If, Else, While, For,
    Public, Private, Protected,
    Static, Extends, Interface, Implements,
    True, False, // Boolean literals
    Identifier(String),
    Number(i64),
    FloatLit(String), // Keep as string to parse later or f64
    CharLit(char),
    StringLit(String),
    Equals, DoubleEquals, NotEquals, Plus, Minus, Star, Slash, Comma, Dot, LessThan, GreaterThan, Semicolon,
    LParen, RParen, LBrace, RBrace, LBracket, RBracket,
    Newline,
    EOF,
}

pub struct Lexer {
    input: Vec<char>,
    pos: usize,
}

impl Lexer {
    pub fn new(input: String) -> Self {
        Self { input: input.chars().collect(), pos: 0 }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        while self.pos < self.input.len() {
            let c = self.input[self.pos];
            match c {
                ' ' | '\r' | '\t' => { self.pos += 1; }
                '\n' => { tokens.push(Token::Newline); self.pos += 1; }
                '=' => {
                    if self.pos + 1 < self.input.len() && self.input[self.pos + 1] == '=' {
                        tokens.push(Token::DoubleEquals); self.pos += 2;
                    } else {
                        tokens.push(Token::Equals); self.pos += 1;
                    }
                }
                '!' => {
                    if self.pos + 1 < self.input.len() && self.input[self.pos + 1] == '=' {
                        tokens.push(Token::NotEquals); self.pos += 2;
                    } else {
                        self.pos += 1;
                    }
                }
                '+' => { tokens.push(Token::Plus); self.pos += 1; }
                '-' => { tokens.push(Token::Minus); self.pos += 1; }
                '*' => { tokens.push(Token::Star); self.pos += 1; }
                '/' => { 
                    if self.pos + 1 < self.input.len() && self.input[self.pos + 1] == '/' {
                        while self.pos < self.input.len() && self.input[self.pos] != '\n' {
                            self.pos += 1;
                        }
                    } else {
                        tokens.push(Token::Slash); self.pos += 1; 
                    }
                }
                '<' => { tokens.push(Token::LessThan); self.pos += 1; }
                '>' => { tokens.push(Token::GreaterThan); self.pos += 1; }
                '.' => { tokens.push(Token::Dot); self.pos += 1; }
                ',' => { tokens.push(Token::Comma); self.pos += 1; }
                ';' => { tokens.push(Token::Semicolon); self.pos += 1; }
                '(' => { tokens.push(Token::LParen); self.pos += 1; }
                ')' => { tokens.push(Token::RParen); self.pos += 1; }
                '{' => { tokens.push(Token::LBrace); self.pos += 1; }
                '}' => { tokens.push(Token::RBrace); self.pos += 1; }
                '[' => { tokens.push(Token::LBracket); self.pos += 1; }
                ']' => { tokens.push(Token::RBracket); self.pos += 1; }
                '\'' => tokens.push(self.read_char()),
                'a'..='z' | 'A'..='Z' | '_' => tokens.push(self.read_identifier()),
                '0'..='9' => tokens.push(self.read_number()),
                '"' => tokens.push(self.read_string()),
                _ => { self.pos += 1; } // Skip unknowns
            }
        }
        tokens.push(Token::EOF);
        tokens
    }

    fn read_identifier(&mut self) -> Token {
        let start = self.pos;
        while self.pos < self.input.len() && (self.input[self.pos].is_alphanumeric() || self.input[self.pos] == '_') {
            self.pos += 1;
        }
        let text: String = self.input[start..self.pos].iter().collect();
        match text.as_str() {
            "var" => Token::Var,
            "int" => Token::Int,
            "float" => Token::Float,
            "char" => Token::Char,
            "void" => Token::Void,
            "String" => Token::String,
            "bool" => Token::Bool,
            "List" => Token::List, // New Token
            "true" => Token::True,
            "false" => Token::False,
            "new" => Token::New,
            "init" => Token::Init,
            "mut" => Token::Mut,
            "func" => Token::Func,
            "class" => Token::Class,
            "return" => Token::Return,
            "print" => Token::Print,
            "if" => Token::If,
            "else" => Token::Else,
            "while" => Token::While,
            "for" => Token::For,
            "public" => Token::Public,
            "private" => Token::Private,
            "protected" => Token::Protected,
            "static" => Token::Static,
            "extends" => Token::Extends,
            "interface" => Token::Interface,
            "implements" => Token::Implements,
            _ => Token::Identifier(text),
        }
    }

    fn read_number(&mut self) -> Token {
        let start = self.pos;
        let mut is_float = false;

        while self.pos < self.input.len() {
            let c = self.input[self.pos];
            if c.is_digit(10) {
                self.pos += 1;
            } else if c == '.' && !is_float {
                // Check if next char is digit to avoid confusion with method call or property access (though usually space separates, but 1.toString() is valid in some langs)
                // In this simple lexer, we assume 1.2 is float. 1.method is not supported yet or requires lookahead.
                // Let's assume if we see dot followed by digit, it's float.
                if self.pos + 1 < self.input.len() && self.input[self.pos + 1].is_digit(10) {
                    is_float = true;
                    self.pos += 1;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        let text: String = self.input[start..self.pos].iter().collect();
        if is_float {
            Token::FloatLit(text)
        } else {
            Token::Number(text.parse().unwrap())
        }
    }

    fn read_string(&mut self) -> Token {
        self.pos += 1; // Skip opening quote
        let start = self.pos;
        while self.pos < self.input.len() && self.input[self.pos] != '"' {
            self.pos += 1;
        }
        let text: String = self.input[start..self.pos].iter().collect();
        if self.pos < self.input.len() { self.pos += 1; } // Skip closing quote
        Token::StringLit(text)
    }

    fn read_char(&mut self) -> Token {
        self.pos += 1; // Skip opening quote
        if self.pos >= self.input.len() { return Token::EOF; }

        let c = self.input[self.pos];
        self.pos += 1;

        if self.pos < self.input.len() && self.input[self.pos] == '\'' {
            self.pos += 1; // Skip closing quote
            Token::CharLit(c)
        } else {
            // Error or malformed char
            Token::CharLit(c)
        }
    }
}
