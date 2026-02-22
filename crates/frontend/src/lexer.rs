//! Java lexer — tokenizes Java source into a flat token stream.
//!
//! Full Java 21 token coverage: all keywords, operators, literals (hex, octal,
//! binary, char escapes, unicode escapes), and punctuation.

use rava_common::error::{RavaError, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Literals
    IntLit(i64),
    FloatLit(f64),
    StrLit(String),
    CharLit(i64),
    BoolLit(bool),
    Null,

    // Identifiers & keywords
    Ident(String),
    Class,
    Public,
    Private,
    Protected,
    Static,
    Void,
    Return,
    New,
    If,
    Else,
    While,
    For,
    Do,
    This,
    Super,
    Import,
    Package,
    Final,
    Extends,
    Implements,
    Interface,
    Throws,
    Throw,
    Try,
    Catch,
    Finally,
    Var,
    Switch,
    Case,
    Default,
    Break,
    Continue,
    Abstract,
    Synchronized,
    Native,
    Volatile,
    Transient,
    Strictfp,
    Enum,
    Instanceof,
    Assert,
    Yield,
    Record,
    Sealed,
    Permits,

    // Primitive type keywords
    Int,
    Long,
    Double,
    Float,
    Boolean,
    Byte,
    Short,
    Char,

    // Punctuation
    LBrace,
    RBrace,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Semi,
    Comma,
    Dot,
    Ellipsis,
    Arrow,      // ->
    ColonColon, // ::
    Tilde,      // ~

    // Operators
    Assign,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Not,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    UShr,
    PlusAssign,
    MinusAssign,
    StarAssign,
    SlashAssign,
    PercentAssign,
    BitAndAssign,
    BitOrAssign,
    BitXorAssign,
    ShlAssign,
    ShrAssign,
    UShrAssign,
    PlusPlus,
    MinusMinus,
    Question,
    Colon,
    At,

    Eof,
}

pub struct Lexer<'a> {
    src:  &'a [u8],
    pos:  usize,
    line: u32,
    col:  u32,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Self { src: src.as_bytes(), pos: 0, line: 1, col: 1 }
    }

    pub fn tokenize(mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();
        loop {
            let tok = self.next_token()?;
            let done = tok == Token::Eof;
            tokens.push(tok);
            if done { break; }
        }
        Ok(tokens)
    }

    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    fn peek2(&self) -> Option<u8> {
        self.src.get(self.pos + 1).copied()
    }

    #[allow(dead_code)]
    fn peek3(&self) -> Option<u8> {
        self.src.get(self.pos + 2).copied()
    }

    fn advance(&mut self) -> Option<u8> {
        let ch = self.src.get(self.pos).copied()?;
        self.pos += 1;
        if ch == b'\n' { self.line += 1; self.col = 1; } else { self.col += 1; }
        Some(ch)
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            // whitespace
            while matches!(self.peek(), Some(b' ' | b'\t' | b'\r' | b'\n')) {
                self.advance();
            }
            // line comment
            if self.peek() == Some(b'/') && self.peek2() == Some(b'/') {
                while !matches!(self.peek(), Some(b'\n') | None) { self.advance(); }
                continue;
            }
            // block comment
            if self.peek() == Some(b'/') && self.peek2() == Some(b'*') {
                self.advance(); self.advance();
                loop {
                    match self.advance() {
                        None => break,
                        Some(b'*') if self.peek() == Some(b'/') => { self.advance(); break; }
                        _ => {}
                    }
                }
                continue;
            }
            break;
        }
    }

    fn read_string(&mut self) -> Result<Token> {
        let mut s = String::new();
        loop {
            match self.advance() {
                None | Some(b'\n') => return Err(RavaError::Parse {
                    location: format!("{}:{}", self.line, self.col),
                    message: "unterminated string literal".into(),
                }),
                Some(b'"') => break,
                Some(b'\\') => s.push(self.read_escape()?),
                Some(c) => s.push(c as char),
            }
        }
        Ok(Token::StrLit(s))
    }

    /// Read a text block: everything between """ and """.
    /// Skips the first newline after opening """, strips common leading whitespace.
    fn read_text_block(&mut self) -> Result<Token> {
        // Skip optional whitespace + mandatory newline after opening """
        while self.peek() == Some(b' ') || self.peek() == Some(b'\t') {
            self.advance();
        }
        if self.peek() == Some(b'\n') {
            self.advance();
        } else if self.peek() == Some(b'\r') {
            self.advance();
            if self.peek() == Some(b'\n') { self.advance(); }
        }

        // Read until closing """
        let mut raw = String::new();
        loop {
            match self.peek() {
                None => return Err(RavaError::Parse {
                    location: format!("{}:{}", self.line, self.col),
                    message: "unterminated text block".into(),
                }),
                Some(b'"') if self.peek2() == Some(b'"') && self.peek3() == Some(b'"') => {
                    self.advance(); self.advance(); self.advance();
                    break;
                }
                Some(b'\\') => {
                    self.advance();
                    raw.push(self.read_escape()?);
                }
                _ => {
                    let c = self.advance().unwrap();
                    raw.push(c as char);
                }
            }
        }

        // Strip trailing newline before closing """
        if raw.ends_with('\n') {
            raw.pop();
            if raw.ends_with('\r') { raw.pop(); }
        }

        // Strip common leading whitespace (Java text block spec)
        let lines: Vec<&str> = raw.split('\n').collect();
        let min_indent = lines.iter()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.len() - l.trim_start().len())
            .min()
            .unwrap_or(0);

        let stripped: Vec<&str> = lines.iter()
            .map(|l| if l.len() >= min_indent { &l[min_indent..] } else { l.trim_start() })
            .collect();

        Ok(Token::StrLit(stripped.join("\n")))
    }

    fn read_escape(&mut self) -> Result<char> {
        match self.advance() {
            Some(b'n')  => Ok('\n'),
            Some(b't')  => Ok('\t'),
            Some(b'r')  => Ok('\r'),
            Some(b'"')  => Ok('"'),
            Some(b'\'') => Ok('\''),
            Some(b'\\') => Ok('\\'),
            Some(b'0')  => Ok('\0'),
            Some(b'b')  => Ok('\u{0008}'), // backspace
            Some(b'f')  => Ok('\u{000C}'), // form feed
            Some(b'u')  => {
                // Unicode escape: \uXXXX
                let mut hex = String::with_capacity(4);
                for _ in 0..4 {
                    match self.advance() {
                        Some(c) if (c as char).is_ascii_hexdigit() => hex.push(c as char),
                        _ => return Err(RavaError::Parse {
                            location: format!("{}:{}", self.line, self.col),
                            message: "invalid unicode escape".into(),
                        }),
                    }
                }
                let code = u32::from_str_radix(&hex, 16).unwrap_or(0);
                Ok(char::from_u32(code).unwrap_or('\u{FFFD}'))
            }
            // Octal escape: \0-\377
            Some(c) if c.is_ascii_digit() => {
                let mut oct = String::new();
                oct.push(c as char);
                for _ in 0..2 {
                    if let Some(d) = self.peek() {
                        if d.is_ascii_digit() && d <= b'7' {
                            self.advance();
                            oct.push(d as char);
                        } else { break; }
                    }
                }
                let code = u32::from_str_radix(&oct, 8).unwrap_or(0);
                Ok(char::from_u32(code).unwrap_or('\0'))
            }
            Some(c) => Ok(c as char),
            None => Err(RavaError::Parse {
                location: format!("{}:{}", self.line, self.col),
                message: "unterminated escape sequence".into(),
            }),
        }
    }

    fn read_char_literal(&mut self) -> Result<Token> {
        let c = match self.advance() {
            Some(b'\\') => self.read_escape()?,
            Some(c) => c as char,
            None => return Err(RavaError::Parse {
                location: format!("{}:{}", self.line, self.col),
                message: "unterminated char literal".into(),
            }),
        };
        // consume closing quote
        match self.advance() {
            Some(b'\'') => {}
            _ => return Err(RavaError::Parse {
                location: format!("{}:{}", self.line, self.col),
                message: "unterminated char literal".into(),
            }),
        }
        Ok(Token::CharLit(c as i64))
    }

    fn read_number(&mut self, first: u8) -> Token {
        // Hex: 0x or 0X
        if first == b'0' && matches!(self.peek(), Some(b'x' | b'X')) {
            self.advance();
            let mut hex = String::new();
            while let Some(c) = self.peek() {
                if c.is_ascii_hexdigit() || c == b'_' {
                    self.advance();
                    if c != b'_' { hex.push(c as char); }
                } else { break; }
            }
            // skip type suffix
            if matches!(self.peek(), Some(b'l' | b'L')) { self.advance(); }
            return Token::IntLit(i64::from_str_radix(&hex, 16).unwrap_or(0));
        }
        // Binary: 0b or 0B
        if first == b'0' && matches!(self.peek(), Some(b'b' | b'B')) {
            self.advance();
            let mut bin = String::new();
            while let Some(c) = self.peek() {
                if c == b'0' || c == b'1' || c == b'_' {
                    self.advance();
                    if c != b'_' { bin.push(c as char); }
                } else { break; }
            }
            if matches!(self.peek(), Some(b'l' | b'L')) { self.advance(); }
            return Token::IntLit(i64::from_str_radix(&bin, 2).unwrap_or(0));
        }
        // Octal: starts with 0 and followed by digits (not a float)
        if first == b'0' && matches!(self.peek(), Some(b'0'..=b'7')) {
            let mut oct = String::new();
            while let Some(c) = self.peek() {
                if (b'0'..=b'7').contains(&c) || c == b'_' {
                    self.advance();
                    if c != b'_' { oct.push(c as char); }
                } else { break; }
            }
            if matches!(self.peek(), Some(b'l' | b'L')) { self.advance(); }
            return Token::IntLit(i64::from_str_radix(&oct, 8).unwrap_or(0));
        }

        // Decimal (or float)
        let mut num = String::new();
        num.push(first as char);
        let mut is_float = false;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || c == b'_' {
                self.advance();
                if c != b'_' { num.push(c as char); }
            } else if c == b'.' && !is_float {
                // check it's not a method call like 123.toString()
                if matches!(self.peek2(), Some(d) if d.is_ascii_digit()) || self.peek2() == None {
                    is_float = true;
                    self.advance();
                    num.push('.');
                } else if matches!(self.peek2(), Some(b'.')) {
                    // 0.. range — don't consume
                    break;
                } else {
                    is_float = true;
                    self.advance();
                    num.push('.');
                }
            } else if matches!(c, b'e' | b'E') {
                is_float = true;
                self.advance();
                num.push('e');
                if matches!(self.peek(), Some(b'+' | b'-')) {
                    let sign = self.advance().unwrap();
                    num.push(sign as char);
                }
                // consume exponent digits
                while let Some(d) = self.peek() {
                    if d.is_ascii_digit() {
                        self.advance();
                        num.push(d as char);
                    } else { break; }
                }
            } else if matches!(c, b'l' | b'L') {
                self.advance();
                break;
            } else if matches!(c, b'f' | b'F' | b'd' | b'D') {
                is_float = true;
                self.advance();
                break;
            } else {
                break;
            }
        }
        if is_float {
            Token::FloatLit(num.parse().unwrap_or(0.0))
        } else {
            Token::IntLit(num.parse().unwrap_or(0))
        }
    }

    fn read_ident(&mut self, first: u8) -> Token {
        let mut s = String::new();
        s.push(first as char);
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == b'_' || c == b'$' {
                self.advance();
                s.push(c as char);
            } else {
                break;
            }
        }
        match s.as_str() {
            "class"        => Token::Class,
            "public"       => Token::Public,
            "private"      => Token::Private,
            "protected"    => Token::Protected,
            "static"       => Token::Static,
            "void"         => Token::Void,
            "return"       => Token::Return,
            "new"          => Token::New,
            "if"           => Token::If,
            "else"         => Token::Else,
            "while"        => Token::While,
            "for"          => Token::For,
            "do"           => Token::Do,
            "this"         => Token::This,
            "super"        => Token::Super,
            "import"       => Token::Import,
            "package"      => Token::Package,
            "final"        => Token::Final,
            "extends"      => Token::Extends,
            "implements"   => Token::Implements,
            "interface"    => Token::Interface,
            "throws"       => Token::Throws,
            "throw"        => Token::Throw,
            "try"          => Token::Try,
            "catch"        => Token::Catch,
            "finally"      => Token::Finally,
            "var"          => Token::Var,
            "switch"       => Token::Switch,
            "case"         => Token::Case,
            "default"      => Token::Default,
            "break"        => Token::Break,
            "continue"     => Token::Continue,
            "abstract"     => Token::Abstract,
            "synchronized" => Token::Synchronized,
            "native"       => Token::Native,
            "volatile"     => Token::Volatile,
            "transient"    => Token::Transient,
            "strictfp"     => Token::Strictfp,
            "enum"         => Token::Enum,
            "instanceof"   => Token::Instanceof,
            "assert"       => Token::Assert,
            "yield"        => Token::Yield,
            "record"       => Token::Record,
            "sealed"       => Token::Sealed,
            "permits"      => Token::Permits,
            "int"          => Token::Int,
            "long"         => Token::Long,
            "double"       => Token::Double,
            "float"        => Token::Float,
            "boolean"      => Token::Boolean,
            "byte"         => Token::Byte,
            "short"        => Token::Short,
            "char"         => Token::Char,
            "true"         => Token::BoolLit(true),
            "false"        => Token::BoolLit(false),
            "null"         => Token::Null,
            _              => Token::Ident(s),
        }
    }

    fn next_token(&mut self) -> Result<Token> {
        self.skip_whitespace_and_comments();
        let ch = match self.advance() {
            None    => return Ok(Token::Eof),
            Some(c) => c,
        };
        let tok = match ch {
            b'{' => Token::LBrace,
            b'}' => Token::RBrace,
            b'(' => Token::LParen,
            b')' => Token::RParen,
            b'[' => Token::LBracket,
            b']' => Token::RBracket,
            b';' => Token::Semi,
            b',' => Token::Comma,
            b'@' => Token::At,
            b'?' => Token::Question,
            b'~' => Token::Tilde,
            b':' => {
                if self.peek() == Some(b':') { self.advance(); Token::ColonColon }
                else { Token::Colon }
            }
            b'.' => {
                if self.peek() == Some(b'.') && self.peek2() == Some(b'.') {
                    self.advance(); self.advance();
                    Token::Ellipsis
                } else {
                    Token::Dot
                }
            }
            b'=' => if self.peek() == Some(b'=') { self.advance(); Token::Eq   } else { Token::Assign },
            b'!' => if self.peek() == Some(b'=') { self.advance(); Token::Ne   } else { Token::Not    },
            b'<' => {
                if self.peek() == Some(b'=') { self.advance(); Token::Le }
                else if self.peek() == Some(b'<') {
                    self.advance();
                    if self.peek() == Some(b'=') { self.advance(); Token::ShlAssign }
                    else { Token::Shl }
                }
                else { Token::Lt }
            }
            b'>' => {
                if self.peek() == Some(b'=') { self.advance(); Token::Ge }
                else if self.peek() == Some(b'>') {
                    self.advance();
                    if self.peek() == Some(b'>') {
                        self.advance();
                        if self.peek() == Some(b'=') { self.advance(); Token::UShrAssign }
                        else { Token::UShr }
                    } else if self.peek() == Some(b'=') {
                        self.advance(); Token::ShrAssign
                    } else { Token::Shr }
                }
                else { Token::Gt }
            }
            b'&' => {
                if self.peek() == Some(b'&') { self.advance(); Token::And }
                else if self.peek() == Some(b'=') { self.advance(); Token::BitAndAssign }
                else { Token::BitAnd }
            }
            b'|' => {
                if self.peek() == Some(b'|') { self.advance(); Token::Or }
                else if self.peek() == Some(b'=') { self.advance(); Token::BitOrAssign }
                else { Token::BitOr }
            }
            b'^' => {
                if self.peek() == Some(b'=') { self.advance(); Token::BitXorAssign }
                else { Token::BitXor }
            }
            b'+' => {
                if self.peek() == Some(b'+') { self.advance(); Token::PlusPlus }
                else if self.peek() == Some(b'=') { self.advance(); Token::PlusAssign }
                else { Token::Plus }
            }
            b'-' => {
                if self.peek() == Some(b'-') { self.advance(); Token::MinusMinus }
                else if self.peek() == Some(b'=') { self.advance(); Token::MinusAssign }
                else if self.peek() == Some(b'>') { self.advance(); Token::Arrow }
                else { Token::Minus }
            }
            b'*' => if self.peek() == Some(b'=') { self.advance(); Token::StarAssign  } else { Token::Star    },
            b'/' => if self.peek() == Some(b'=') { self.advance(); Token::SlashAssign } else { Token::Slash   },
            b'%' => if self.peek() == Some(b'=') { self.advance(); Token::PercentAssign } else { Token::Percent },
            b'"' => {
                // Check for text block: """
                if self.peek() == Some(b'"') && self.peek2() == Some(b'"') {
                    self.advance(); // consume second "
                    self.advance(); // consume third "
                    self.read_text_block()?
                } else {
                    self.read_string()?
                }
            },
            b'\'' => self.read_char_literal()?,
            c if c.is_ascii_digit() => self.read_number(c),
            c if c.is_ascii_alphabetic() || c == b'_' || c == b'$' => self.read_ident(c),
            c => return Err(RavaError::Parse {
                location: format!("{}:{}", self.line, self.col),
                message: format!("unexpected character: {:?}", c as char),
            }),
        };
        Ok(tok)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_hello_world() {
        let src = r#"class Main { public static void main(String[] args) { System.out.println("Hello"); } }"#;
        let tokens = Lexer::new(src).tokenize().unwrap();
        assert!(tokens.contains(&Token::Class));
        assert!(tokens.contains(&Token::Ident("Main".into())));
        assert!(tokens.contains(&Token::StrLit("Hello".into())));
    }

    #[test]
    fn tokenize_operators() {
        let tokens = Lexer::new("== != <= >= && ||").tokenize().unwrap();
        assert!(tokens.contains(&Token::Eq));
        assert!(tokens.contains(&Token::Ne));
        assert!(tokens.contains(&Token::Le));
        assert!(tokens.contains(&Token::Ge));
        assert!(tokens.contains(&Token::And));
        assert!(tokens.contains(&Token::Or));
    }

    #[test]
    fn skip_line_comment() {
        let tokens = Lexer::new("// comment\nclass").tokenize().unwrap();
        assert_eq!(tokens[0], Token::Class);
    }

    #[test]
    fn tokenize_hex_literal() {
        let tokens = Lexer::new("0xFF 0x1A3F").tokenize().unwrap();
        assert_eq!(tokens[0], Token::IntLit(0xFF));
        assert_eq!(tokens[1], Token::IntLit(0x1A3F));
    }

    #[test]
    fn tokenize_binary_literal() {
        let tokens = Lexer::new("0b1010 0B1111_0000").tokenize().unwrap();
        assert_eq!(tokens[0], Token::IntLit(0b1010));
        assert_eq!(tokens[1], Token::IntLit(0b11110000));
    }

    #[test]
    fn tokenize_arrow_and_coloncolon() {
        let tokens = Lexer::new("-> ::").tokenize().unwrap();
        assert_eq!(tokens[0], Token::Arrow);
        assert_eq!(tokens[1], Token::ColonColon);
    }

    #[test]
    fn tokenize_compound_assigns() {
        let tokens = Lexer::new("%=  &= |= ^= <<= >>= >>>=").tokenize().unwrap();
        assert!(tokens.contains(&Token::PercentAssign));
        assert!(tokens.contains(&Token::BitAndAssign));
        assert!(tokens.contains(&Token::BitOrAssign));
        assert!(tokens.contains(&Token::BitXorAssign));
        assert!(tokens.contains(&Token::ShlAssign));
        assert!(tokens.contains(&Token::ShrAssign));
        assert!(tokens.contains(&Token::UShrAssign));
    }

    #[test]
    fn tokenize_char_literal() {
        let tokens = Lexer::new(r"'A' '\n' '\u0041'").tokenize().unwrap();
        assert_eq!(tokens[0], Token::CharLit(65));
        assert_eq!(tokens[1], Token::CharLit(10));
        assert_eq!(tokens[2], Token::CharLit(65)); // \u0041 = 'A'
    }

    #[test]
    fn tokenize_new_keywords() {
        let tokens = Lexer::new("do abstract synchronized enum instanceof").tokenize().unwrap();
        assert_eq!(tokens[0], Token::Do);
        assert_eq!(tokens[1], Token::Abstract);
        assert_eq!(tokens[2], Token::Synchronized);
        assert_eq!(tokens[3], Token::Enum);
        assert_eq!(tokens[4], Token::Instanceof);
    }
}
