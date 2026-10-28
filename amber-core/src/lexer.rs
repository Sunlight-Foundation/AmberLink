// amber-core/src/lexer.rs

#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    Var,
    Mut,
    Func,
    Class,
    Return,
    Print,
    Int,
    Void,
    String,
    Bool,
    Float,
    Char,
    List,
    New,
    Init,
    If,
    Else,
    While,
    For,
    Public,
    Private,
    Protected,
    Static,
    Extends,
    Interface,
    Implements,
    Import,
    Extern,
    Spawn,
    Null,
    Match,
    Try,
    Catch,
    Throw,
    QuestionMark,
    True,
    False,
    Identifier(String),
    Number(i64),
    FloatLit(String),
    CharLit(char),
    StringLit(String),
    Equals,
    DoubleEquals,
    NotEquals,
    LessEquals,
    GreaterEquals,
    Plus,
    Minus,
    Star,
    Slash,
    Comma,
    Dot,
    LessThan,
    GreaterThan,
    Semicolon,
    Arrow,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Newline,
    Eof,
}

#[derive(Debug, Clone)]
pub struct SpannedToken {
    pub token: Token,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone)]
pub struct SpannedToken {
    pub token: Token,
    pub line: usize,
    pub column: usize,
}

pub struct Lexer {
    input: Vec<char>,
    pos: usize,
    line: usize,
    column: usize,
}

impl Lexer {
    pub fn new(input: String) -> Self {
        Self {
            input: input.chars().collect(),
            pos: 0,
            line: 1,
            column: 1,
        }
    }

    fn push(&self, tokens: &mut Vec<SpannedToken>, token: Token) {
        tokens.push(SpannedToken {
            token,
            line: self.line,
            column: self.column,
        });
    }

    fn current_line_column(&self) -> (usize, usize) {
        (self.line, self.column)
    }

    fn advance_col(&mut self) {
        self.column += 1;
    }

    pub fn tokenize(&mut self, errors: &mut crate::error::ErrorList) -> Vec<SpannedToken> {
        let mut tokens = Vec::new();
        while self.pos < self.input.len() {
            let c = self.input[self.pos];
            match c {
                ' ' | '\r' | '\t' => {
                    self.pos += 1;
                    self.advance_col();
                }
                '\n' => {
                    self.push(&mut tokens, Token::Newline);
                    self.line += 1;
                    self.column = 1;
                    self.pos += 1;
                }
                '=' => {
                    if self.pos + 1 < self.input.len() && self.input[self.pos + 1] == '=' {
                        self.push(&mut tokens, Token::DoubleEquals);
                        self.pos += 2;
                        self.column += 2;
                    } else if self.pos + 1 < self.input.len() && self.input[self.pos + 1] == '>' {
                        self.push(&mut tokens, Token::Arrow);
                        self.pos += 2;
                        self.column += 2;
                    } else {
                        self.push(&mut tokens, Token::Equals);
                        self.pos += 1;
                        self.advance_col();
                    }
                }
                '!' => {
                    if self.pos + 1 < self.input.len() && self.input[self.pos + 1] == '=' {
                        self.push(&mut tokens, Token::NotEquals);
                        self.pos += 2;
                        self.column += 2;
                    } else {
                        // Bare '!' is not a valid operator yet; report it.
                        let (l, c) = self.current_line_column();
                        errors.push(crate::error::CompileError::new(
                            l,
                            c,
                            "Unexpected character '!'. Use '!=' for inequality.",
                        ));
                        self.pos += 1;
                        self.advance_col();
                    }
                }
                '+' => {
                    self.push(&mut tokens, Token::Plus);
                    self.pos += 1;
                    self.advance_col();
                }
                '-' => {
                    self.push(&mut tokens, Token::Minus);
                    self.pos += 1;
                    self.advance_col();
                }
                '*' => {
                    self.push(&mut tokens, Token::Star);
                    self.pos += 1;
                    self.advance_col();
                }
                '/' => {
                    if self.pos + 1 < self.input.len() && self.input[self.pos + 1] == '/' {
                        // Consume the whole comment line (column irrelevant past here).
                        while self.pos < self.input.len() && self.input[self.pos] != '\n' {
                            self.pos += 1;
                            self.advance_col();
                        }
                    } else {
                        self.push(&mut tokens, Token::Slash);
                        self.pos += 1;
                        self.advance_col();
                    }
                }
                '<' => {
                    if self.pos + 1 < self.input.len() && self.input[self.pos + 1] == '=' {
                        self.push(&mut tokens, Token::LessEquals);
                        self.pos += 2;
                        self.column += 2;
                    } else {
                        self.push(&mut tokens, Token::LessThan);
                        self.pos += 1;
                        self.advance_col();
                    }
                }
                '>' => {
                    if self.pos + 1 < self.input.len() && self.input[self.pos + 1] == '=' {
                        self.push(&mut tokens, Token::GreaterEquals);
                        self.pos += 2;
                        self.column += 2;
                    } else {
                        self.push(&mut tokens, Token::GreaterThan);
                        self.pos += 1;
                        self.advance_col();
                    }
                }
                '.' => {
                    self.push(&mut tokens, Token::Dot);
                    self.pos += 1;
                    self.advance_col();
                }
                ',' => {
                    self.push(&mut tokens, Token::Comma);
                    self.pos += 1;
                    self.advance_col();
                }
                ';' => {
                    self.push(&mut tokens, Token::Semicolon);
                    self.pos += 1;
                    self.advance_col();
                }
                '(' => {
                    self.push(&mut tokens, Token::LParen);
                    self.pos += 1;
                    self.advance_col();
                }
                ')' => {
                    self.push(&mut tokens, Token::RParen);
                    self.pos += 1;
                    self.advance_col();
                }
                '{' => {
                    self.push(&mut tokens, Token::LBrace);
                    self.pos += 1;
                    self.advance_col();
                }
                '}' => {
                    self.push(&mut tokens, Token::RBrace);
                    self.pos += 1;
                    self.advance_col();
                }
                '[' => {
                    self.push(&mut tokens, Token::LBracket);
                    self.pos += 1;
                    self.advance_col();
                }
                ']' => {
                    self.push(&mut tokens, Token::RBracket);
                    self.pos += 1;
                    self.advance_col();
                }
                '\'' => self.push_char(&mut tokens, errors),
                'a'..='z' | 'A'..='Z' | '_' => self.push_identifier(&mut tokens),
                '0'..='9' => self.push_number(&mut tokens),
                '"' => self.push_string(&mut tokens, errors),
                '?' => {
                    self.push(&mut tokens, Token::QuestionMark);
                    self.pos += 1;
                    self.advance_col();
                }
                _ => {
                    let (l, c) = self.current_line_column();
                    errors.push(crate::error::CompileError::new(
                        l,
                        c,
                        format!("Unexpected character '{}'.", c),
                    ));
                    self.pos += 1;
                    self.advance_col();
                }
            }
        }
        self.push(&mut tokens, Token::Eof);
        tokens
    }

    fn push_identifier(&mut self, tokens: &mut Vec<SpannedToken>) {
        let start = self.pos;
        while self.pos < self.input.len()
            && (self.input[self.pos].is_alphanumeric() || self.input[self.pos] == '_')
        {
            self.pos += 1;
            self.advance_col();
        }
        let text: String = self.input[start..self.pos].iter().collect();
        let token = match text.as_str() {
            "var" => Token::Var,
            "int" => Token::Int,
            "float" => Token::Float,
            "char" => Token::Char,
            "void" => Token::Void,
            "String" => Token::String,
            "bool" => Token::Bool,
            "List" => Token::List,
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
            "import" => Token::Import,
            "extern" => Token::Extern,
            "spawn" => Token::Spawn,
            "null" => Token::Null,
            "match" => Token::Match,
            "try" => Token::Try,
            "catch" => Token::Catch,
            "throw" => Token::Throw,
            _ => Token::Identifier(text),
        };
        self.push(tokens, token);
    }

    fn push_number(&mut self, tokens: &mut Vec<SpannedToken>) {
        let start = self.pos;
        let mut is_float = false;

        while self.pos < self.input.len() {
            let c = self.input[self.pos];
            if c.is_ascii_digit() {
                self.pos += 1;
                self.advance_col();
            } else if c == '.' && !is_float {
                if self.pos + 1 < self.input.len() && self.input[self.pos + 1].is_ascii_digit() {
                    is_float = true;
                    self.pos += 1;
                    self.advance_col();
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        let text: String = self.input[start..self.pos].iter().collect();
        if is_float {
            self.push(tokens, Token::FloatLit(text));
        } else {
            self.push(tokens, Token::Number(text.parse().unwrap()));
        }
    }

    fn push_string(
        &mut self,
        tokens: &mut Vec<SpannedToken>,
        errors: &mut crate::error::ErrorList,
    ) {
        let start_line = self.line;
        let start_col = self.column;
        self.pos += 1;
        self.advance_col();
        let start = self.pos;
        while self.pos < self.input.len() && self.input[self.pos] != '"' {
            if self.input[self.pos] == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.advance_col();
            }
            self.pos += 1;
        }
        let text: String = self.input[start..self.pos].iter().collect();
        if self.pos < self.input.len() {
            self.pos += 1;
            self.advance_col();
            self.push(tokens, Token::StringLit(text));
        } else {
            self.push(tokens, Token::StringLit(text));
            errors.push(crate::error::CompileError::new(
                start_line,
                start_col,
                "Unterminated string literal.",
            ));
        }
    }

    fn push_char(&mut self, tokens: &mut Vec<SpannedToken>, errors: &mut crate::error::ErrorList) {
        let start_line = self.line;
        let start_col = self.column;
        self.pos += 1;
        self.advance_col();
        if self.pos >= self.input.len() {
            self.push(tokens, Token::CharLit('\0'));
            errors.push(crate::error::CompileError::new(
                start_line,
                start_col,
                "Unterminated character literal.",
            ));
            return;
        }

        let c = self.input[self.pos];
        self.pos += 1;
        self.advance_col();

        if self.pos < self.input.len() && self.input[self.pos] == '\'' {
            self.pos += 1;
            self.advance_col();
            self.push(tokens, Token::CharLit(c));
        } else {
            self.push(tokens, Token::CharLit(c));
            errors.push(crate::error::CompileError::new(
                start_line,
                start_col,
                "Unterminated character literal.",
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Lexer, Token};
    use crate::error::ErrorList;

    fn tokenize(source: &str) -> (Vec<Token>, ErrorList) {
        let mut lexer = Lexer::new(source.to_owned());
        let mut errors = ErrorList::new();
        let tokens = lexer
            .tokenize(&mut errors)
            .into_iter()
            .map(|t| t.token)
            .collect();
        (tokens, errors)
    }

    #[test]
    fn tokenizes_keywords_operators_and_literals() {
        let (tokens, errors) = tokenize("var total = 12 + 3.5 == 15.5\nprint \"ok\"");
        assert!(!errors.has_errors(), "{errors}");
        assert_eq!(
            tokens,
            vec![
                Token::Var,
                Token::Identifier("total".into()),
                Token::Equals,
                Token::Number(12),
                Token::Plus,
                Token::FloatLit("3.5".into()),
                Token::DoubleEquals,
                Token::FloatLit("15.5".into()),
                Token::Newline,
                Token::Print,
                Token::StringLit("ok".into()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn reports_invalid_characters_with_location() {
        let (_, errors) = tokenize("var x = 1\n!");
        assert_eq!(errors.len(), 1);
        assert_eq!(errors.errors[0].line, 2);
        assert_eq!(errors.errors[0].column, 1);
        assert!(errors.errors[0].message.contains("Unexpected character"));
    }

    #[test]
    fn reports_unterminated_strings() {
        let (_, errors) = tokenize("\"unfinished");
        assert_eq!(errors.len(), 1);
        assert!(errors.errors[0]
            .message
            .contains("Unterminated string literal"));
    }
}
