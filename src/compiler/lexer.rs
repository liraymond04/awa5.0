#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Ident(String),
    Number(String),
    StringLit(String),
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Eq,
    EqEq,
    Not,
    NotEq,
    Colon,
    Comma,
    Dot,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Lt,
    LtEq,
    Gt,
    GtEq,
    AndAnd,
    OrOr,
    Caret,
    Semicolon,
    DoubleSemicolon,
    Arrow, // ->
    FatArrow, // =>
    Bar,
    // Keywords as distinct tokens for easier parsing
    Module,
    ModuleType,
    Include,
    Extern,
    Let,
    LetRec,
    Fn,
    Func,
    Fun,
    Function,
    Struct,
    End,
    Functor,
    Sig,
    Type,
    If,
    Then,
    Else,
    Match,
    With,
    When,
    As,
    EOF,
}

pub struct Lexer<'a> {
    src: &'a str,
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Self { src, pos: 0 }
    }

    fn peek_char(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        if let Some(ch) = self.peek_char() {
            let ch_len = ch.len_utf8();
            self.pos += ch_len;
            Some(ch)
        } else {
            None
        }
    }

    fn eat_while<F: Fn(char) -> bool>(&mut self, f: F) -> &'a str {
        let start = self.pos;
        while let Some(ch) = self.peek_char() {
            if f(ch) {
                self.bump();
            } else {
                break;
            }
        }
        &self.src[start..self.pos]
    }

    fn skip_ws_and_comments(&mut self) {
        loop {
            self.eat_while(|c| c.is_whitespace());

            let pos_before = self.pos;

            // Try to match OCaml-style comment (* ... *)
            if let Some('(') = self.peek_char() {
                self.bump();
                if let Some('*') = self.peek_char() {
                    self.bump();
                    // We have a block comment
                    loop {
                        match self.peek_char() {
                            None => break,
                            Some('*') => {
                                self.bump();
                                if let Some(')') = self.peek_char() {
                                    self.bump();
                                    break;
                                }
                            }
                            _ => {
                                self.bump();
                            }
                        }
                    }
                    continue;
                } else {
                    // Not a comment; rewind
                    self.pos = pos_before;
                }
            }

            // Try to match C-style comments (// and /* */)
            if let Some('/') = self.peek_char() {
                self.bump();

                match self.peek_char() {
                    Some('/') => {
                        // line comment
                        self.bump();
                        while let Some(ch) = self.peek_char() {
                            self.bump();
                            if ch == '\n' {
                                break;
                            }
                        }
                    }
                    Some('*') => {
                        // block comment
                        self.bump();
                        while let Some(ch) = self.peek_char() {
                            if ch == '*' {
                                self.bump();
                                if let Some('/') = self.peek_char() {
                                    self.bump();
                                    break;
                                }
                            } else {
                                self.bump();
                            }
                        }
                    }
                    _ => {
                        // not actually a comment; rewind and stop
                        self.pos = pos_before;
                        break;
                    }
                }
            } else {
                break;
            }
        }
    }

    pub fn next_token(&mut self) -> Token {
        // skip whitespace and comments
        self.skip_ws_and_comments();

        match self.peek_char() {
            None => Token::EOF,
            Some('(') => {
                self.bump();
                Token::LParen
            }
            Some(')') => {
                self.bump();
                Token::RParen
            }
            Some('{') => {
                self.bump();
                Token::LBrace
            }
            Some('}') => {
                self.bump();
                Token::RBrace
            }
            Some('[') => {
                self.bump();
                Token::LBracket
            }
            Some(']') => {
                self.bump();
                Token::RBracket
            }
            Some('=') => {
                self.bump();
                match self.peek_char() {
                    Some('=') => {
                        self.bump();
                        Token::EqEq
                    }
                    Some('>') => {
                        self.bump();
                        Token::FatArrow
                    }
                    _ => Token::Eq,
                }
            }
            Some('!') => {
                self.bump();
                if let Some('=') = self.peek_char() {
                    self.bump();
                    Token::NotEq
                } else {
                    Token::Not
                }
            }
            Some(':') => {
                self.bump();
                Token::Colon
            }
            Some(',') => {
                self.bump();
                Token::Comma
            }
            Some('.') => {
                self.bump();
                Token::Dot
            }
            Some('+') => {
                self.bump();
                Token::Plus
            }
            Some('*') => {
                self.bump();
                Token::Star
            }
            Some('/') => {
                self.bump();
                Token::Slash
            }
            Some('%') => {
                self.bump();
                Token::Percent
            }
            Some('<') => {
                self.bump();
                if let Some('=') = self.peek_char() {
                    self.bump();
                    Token::LtEq
                } else {
                    Token::Lt
                }
            }
            Some('>') => {
                self.bump();
                if let Some('=') = self.peek_char() {
                    self.bump();
                    Token::GtEq
                } else {
                    Token::Gt
                }
            }
            Some('&') => {
                self.bump();
                if let Some('&') = self.peek_char() {
                    self.bump();
                    Token::AndAnd
                } else {
                    Token::Ident("&".to_string())
                }
            }
            Some('|') => {
                self.bump();
                if let Some('|') = self.peek_char() {
                    self.bump();
                    Token::OrOr
                } else {
                    Token::Bar
                }
            }
            Some('^') => {
                self.bump();
                Token::Caret
            }
            Some('-') => {
                // maybe arrow
                self.bump();
                if let Some('>') = self.peek_char() {
                    self.bump();
                    Token::Arrow
                } else {
                    Token::Minus
                }
            }
            Some(';') => {
                self.bump();
                if let Some(';') = self.peek_char() {
                    self.bump();
                    Token::DoubleSemicolon
                } else {
                    Token::Semicolon
                }
            }
            Some('"') => {
                // string literal
                self.bump();
                let start = self.pos;
                while let Some(ch) = self.peek_char() {
                    self.bump();
                    if ch == '"' {
                        // include closing quote in slice end
                        break;
                    }
                }
                // slice between start and current pos, excluding closing quote
                let raw = &self.src[start..self.pos];
                // trim trailing quote if present
                let lit = raw.trim_end_matches('"').to_string();
                Token::StringLit(lit)
            }
            Some(ch) if ch.is_ascii_digit() => {
                let s = self.eat_while(|c| c.is_ascii_digit());
                Token::Number(s.to_string())
            }
            Some(ch) if is_ident_start(ch) => {
                let s = self.eat_while(|c| is_ident_continue(c));
                // map keywords to dedicated token kinds
                match s {
                    "module" => Token::Module,
                    "type" => Token::Type,
                    "include" => Token::Include,
                    "extern" | "external" => Token::Extern,
                    "let" => Token::Let,
                    "letrec" => Token::LetRec,
                    "fn" => Token::Fn,
                    "func" => Token::Func,
                    "fun" => Token::Fun,
                    "function" => Token::Function,
                    "struct" => Token::Struct,
                    "end" => Token::End,
                    "functor" => Token::Functor,
                    "sig" => Token::Sig,
                    "if" => Token::If,
                    "then" => Token::Then,
                    "else" => Token::Else,
                    "match" => Token::Match,
                    "with" => Token::With,
                    "when" => Token::When,
                    "as" => Token::As,
                    other => Token::Ident(other.to_string()),
                }
            }
            Some(_) => {
                // unknown single-char -> emit as ident
                if let Some(ch) = self.bump() {
                    Token::Ident(ch.to_string())
                } else {
                    Token::EOF
                }
            }
        }
    }

    pub fn tokenize_all(mut self) -> Vec<Token> {
        let mut out = Vec::new();
        loop {
            let t = self.next_token();
            if t == Token::EOF {
                out.push(t);
                break;
            }
            out.push(t);
        }
        out
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_' 
}

fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '.'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_simple() {
        let src = "module M = struct end;; include Foo;; extern foo;;";
        let toks = Lexer::new(src).tokenize_all();
        // ensure some tokens produced and ends with EOF
        assert!(toks.len() > 0);
        assert_eq!(toks.last().unwrap(), &Token::EOF);
    }

    #[test]
    fn test_skip_comments_and_operators() {
        let src = "1 + 2 // line\n/* block */ 3 == 4 && 5 || 6 <= 7 >= 8 != 9";
        let toks = Lexer::new(src).tokenize_all();

        assert!(toks.contains(&Token::Plus));
        assert!(toks.contains(&Token::EqEq));
        assert!(toks.contains(&Token::AndAnd));
        assert!(toks.contains(&Token::OrOr));
        assert!(toks.contains(&Token::LtEq));
        assert!(toks.contains(&Token::GtEq));
        assert!(toks.contains(&Token::NotEq));
        assert!(!toks.iter().any(|t| matches!(t, Token::Slash | Token::Star)));
        assert_eq!(toks.last().unwrap(), &Token::EOF);
    }
}
