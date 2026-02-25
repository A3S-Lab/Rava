//! Java parser — tokens → AST.
//!
//! Recursive-descent parser with full Java 21 syntax coverage:
//! class/interface/enum, all statements, all expressions including
//! lambda, method reference, ternary, instanceof pattern matching.

use rava_common::error::{RavaError, Result};
use crate::ast::*;
use crate::lexer::Token;

mod class;
mod types;
mod stmt;
mod expr;

pub struct Parser {
    pub(crate) tokens:         Vec<Token>,
    pub(crate) pos:            usize,
    pub(crate) pending_fields: Vec<crate::ast::Member>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0, pending_fields: Vec::new() }
    }

    // ── Token navigation ──────────────────────────────────────────────────────

    pub(crate) fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }

    pub(crate) fn peek2(&self) -> &Token {
        self.tokens.get(self.pos + 1).unwrap_or(&Token::Eof)
    }

    #[allow(dead_code)]
    pub(crate) fn peek_at(&self, offset: usize) -> &Token {
        self.tokens.get(self.pos + offset).unwrap_or(&Token::Eof)
    }

    pub(crate) fn advance(&mut self) -> &Token {
        let t = self.tokens.get(self.pos).unwrap_or(&Token::Eof);
        if self.pos < self.tokens.len() { self.pos += 1; }
        t
    }

    pub(crate) fn expect(&mut self, expected: &Token) -> Result<()> {
        let got = self.advance().clone();
        if &got == expected {
            Ok(())
        } else {
            Err(RavaError::Parse {
                location: format!("pos {}", self.pos),
                message: format!("expected {:?}, got {:?}", expected, got),
            })
        }
    }

    pub(crate) fn eat(&mut self, tok: &Token) -> bool {
        if self.peek() == tok { self.advance(); true } else { false }
    }

    pub(crate) fn expect_ident(&mut self) -> Result<String> {
        match self.advance().clone() {
            Token::Ident(s) => Ok(s),
            got => Err(RavaError::Parse {
                location: format!("pos {}", self.pos),
                message: format!("expected identifier, got {:?}", got),
            }),
        }
    }

    /// Parse zero or more annotations: `@Name`, `@Name(value)`, `@Name(key=value, ...)`.
    pub(crate) fn parse_annotations(&mut self) -> Result<Vec<crate::ast::Annotation>> {
        let mut annotations = Vec::new();
        while self.peek() == &Token::At {
            self.advance(); // consume @
            // @interface is an annotation type declaration — skip it entirely
            if self.peek() == &Token::Interface {
                self.advance(); // consume 'interface'
                let _name = self.expect_ident()?; // annotation type name
                self.skip_balanced(Token::LBrace, Token::RBrace);
                continue;
            }
            let name = self.expect_ident()?;
            let mut attrs = Vec::new();
            if self.peek() == &Token::LParen {
                self.advance(); // consume (
                if self.peek() != &Token::RParen {
                    loop {
                        // key=value or just value
                        let (key, val) = if matches!(self.peek(), Token::Ident(_)) && self.peek2() == &Token::Assign {
                            let k = self.expect_ident()?;
                            self.advance(); // consume =
                            let v = self.parse_expr()?;
                            (k, v)
                        } else {
                            let v = self.parse_expr()?;
                            (String::new(), v)
                        };
                        attrs.push((key, val));
                        if !self.eat(&Token::Comma) { break; }
                    }
                }
                self.expect(&Token::RParen)?;
            }
            annotations.push(crate::ast::Annotation { name, attrs });
        }
        Ok(annotations)
    }

    // ── Top-level ───────────────────────────────────────────────────────────��─

    pub fn parse_file(&mut self) -> Result<SourceFile> {
        let mut package = None;
        let mut imports = Vec::new();

        // module-info.java: `[open] module <name> { requires ...; exports ...; }` — parse and discard
        if matches!(self.peek(), Token::Ident(s) if s == "open") {
            if matches!(self.peek2(), Token::Ident(s) if s == "module") {
                self.advance(); // open
            }
        }
        if matches!(self.peek(), Token::Ident(s) if s == "module") {
            self.advance(); // module
            self.parse_qualified_name()?; // module name
            self.skip_balanced(Token::LBrace, Token::RBrace);
            return Ok(SourceFile { package, imports, classes: Vec::new() });
        }

        if self.eat(&Token::Package) {
            package = Some(self.parse_qualified_name()?);
            self.expect(&Token::Semi)?;
        }

        while self.peek() == &Token::Import {
            self.advance();
            // skip `static` in static imports
            self.eat(&Token::Static);
            let mut name = self.parse_qualified_name()?;
            if self.eat(&Token::Dot) {
                self.expect(&Token::Star)?;
                name.push_str(".*");
            }
            self.expect(&Token::Semi)?;
            imports.push(name);
        }

        let mut classes = Vec::new();
        while self.peek() != &Token::Eof {
            classes.push(self.parse_class()?);
        }

        Ok(SourceFile { package, imports, classes })
    }

    pub(crate) fn parse_qualified_name(&mut self) -> Result<String> {
        let mut name = self.expect_ident()?;
        while self.peek() == &Token::Dot {
            if self.peek2() == &Token::Star { break; }
            if !matches!(self.peek2(), Token::Ident(_)) { break; }
            self.advance();
            name.push('.');
            name.push_str(&self.expect_ident()?);
        }
        Ok(name)
    }

    pub(crate) fn skip_balanced(&mut self, open: Token, close: Token) {
        if self.peek() != &open { return; }
        let mut depth = 0i32;
        loop {
            let t = self.advance().clone();
            if t == open  { depth += 1; }
            if t == close { depth -= 1; if depth <= 0 { break; } }
            if t == Token::Eof { break; }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn parse(src: &str) -> SourceFile {
        let tokens = Lexer::new(src).tokenize().unwrap();
        Parser::new(tokens).parse_file().unwrap()
    }

    #[test]
    fn parse_hello_world() {
        let src = r#"
            class Main {
                public static void main(String[] args) {
                    System.out.println("Hello, World!");
                }
            }
        "#;
        let file = parse(src);
        assert_eq!(file.classes.len(), 1);
        assert_eq!(file.classes[0].name, "Main");
        let Member::Method(m) = &file.classes[0].members[0] else { panic!() };
        assert_eq!(m.name, "main");
    }

    #[test]
    fn parse_local_var_and_return() {
        let src = r#"
            class Foo {
                int add(int a, int b) {
                    int result = a + b;
                    return result;
                }
            }
        "#;
        let file = parse(src);
        let Member::Method(m) = &file.classes[0].members[0] else { panic!() };
        assert_eq!(m.params.len(), 2);
        assert_eq!(m.body.as_ref().unwrap().0.len(), 2);
    }

    #[test]
    fn parse_do_while() {
        let src = r#"
            class T {
                void f() {
                    int i = 0;
                    do { i++; } while (i < 10);
                }
            }
        "#;
        let file = parse(src);
        let Member::Method(m) = &file.classes[0].members[0] else { panic!() };
        let stmts = &m.body.as_ref().unwrap().0;
        assert!(matches!(stmts[1], Stmt::DoWhile { .. }));
    }

    #[test]
    fn parse_for_each() {
        let src = r#"
            class T {
                void f() {
                    for (String s : args) { System.out.println(s); }
                }
            }
        "#;
        let file = parse(src);
        let Member::Method(m) = &file.classes[0].members[0] else { panic!() };
        assert!(matches!(m.body.as_ref().unwrap().0[0], Stmt::ForEach { .. }));
    }

    #[test]
    fn parse_break_continue() {
        let src = r#"
            class T {
                void f() {
                    while (true) { break; }
                    while (true) { continue; }
                }
            }
        "#;
        let file = parse(src);
        let Member::Method(m) = &file.classes[0].members[0] else { panic!() };
        let stmts = &m.body.as_ref().unwrap().0;
        assert!(matches!(stmts[0], Stmt::While { .. }));
    }

    #[test]
    fn parse_try_catch_finally() {
        let src = r#"
            class T {
                void f() {
                    try {
                        int x = 1;
                    } catch (Exception e) {
                        System.out.println(e);
                    } finally {
                        System.out.println("done");
                    }
                }
            }
        "#;
        let file = parse(src);
        let Member::Method(m) = &file.classes[0].members[0] else { panic!() };
        assert!(matches!(m.body.as_ref().unwrap().0[0], Stmt::TryCatch { .. }));
    }

    #[test]
    fn parse_lambda() {
        let src = r#"
            class T {
                void f() {
                    var fn = (int x) -> x + 1;
                    var fn2 = x -> x * 2;
                }
            }
        "#;
        let file = parse(src);
        let Member::Method(m) = &file.classes[0].members[0] else { panic!() };
        let stmts = &m.body.as_ref().unwrap().0;
        if let Stmt::LocalVar { init: Some(Expr::Lambda { params, .. }), .. } = &stmts[0] {
            assert_eq!(params.len(), 1);
            assert_eq!(params[0].name, "x");
        } else { panic!("expected lambda"); }
    }

    #[test]
    fn parse_enum() {
        let src = r#"
            enum Color { RED, GREEN, BLUE }
        "#;
        let file = parse(src);
        assert_eq!(file.classes[0].kind, ClassKind::Enum);
        assert_eq!(file.classes[0].members.len(), 3);
    }

    #[test]
    fn parse_instanceof_pattern() {
        let src = r#"
            class T {
                void f(Object obj) {
                    if (obj instanceof String s) {
                        System.out.println(s);
                    }
                }
            }
        "#;
        let file = parse(src);
        let Member::Method(m) = &file.classes[0].members[0] else { panic!() };
        if let Stmt::If { cond: Expr::InstanceofPattern { name, .. }, .. } = &m.body.as_ref().unwrap().0[0] {
            assert_eq!(name, "s");
        } else { panic!("expected instanceof pattern"); }
    }

    #[test]
    fn parse_method_ref() {
        let src = r#"
            class T {
                void f() {
                    var fn = System.out::println;
                }
            }
        "#;
        let file = parse(src);
        let Member::Method(m) = &file.classes[0].members[0] else { panic!() };
        if let Stmt::LocalVar { init: Some(Expr::MethodRef { name, .. }), .. } = &m.body.as_ref().unwrap().0[0] {
            assert_eq!(name, "println");
        } else { panic!("expected method ref"); }
    }

    #[test]
    fn parse_record() {
        let src = r#"
            record Point(int x, int y) {
            }
        "#;
        let file = parse(src);
        assert_eq!(file.classes[0].kind, ClassKind::Record);
        assert_eq!(file.classes[0].name, "Point");
        // Should have 2 fields + 1 constructor + 2 accessors = 5 members
        assert_eq!(file.classes[0].members.len(), 5);
    }

    #[test]
    fn parse_text_block() {
        let src = "class T { void f() { String s = \"\"\"\n            hello\n            \"\"\"; } }";
        let file = parse(src);
        let Member::Method(m) = &file.classes[0].members[0] else { panic!() };
        if let Stmt::LocalVar { init: Some(Expr::StrLit(s)), .. } = &m.body.as_ref().unwrap().0[0] {
            assert!(s.contains("hello"));
        } else { panic!("expected text block string"); }
    }

    #[test]
    fn parse_switch_arrow() {
        let src = r#"
            class T {
                void f() {
                    switch (x) {
                        case 1 -> System.out.println("one");
                        case 2 -> { System.out.println("two"); }
                        default -> System.out.println("other");
                    }
                }
            }
        "#;
        let file = parse(src);
        let Member::Method(m) = &file.classes[0].members[0] else { panic!() };
        if let Stmt::Switch { cases, .. } = &m.body.as_ref().unwrap().0[0] {
            assert_eq!(cases.len(), 3);
        } else { panic!("expected switch"); }
    }

    #[test]
    fn parse_sealed_class() {
        let src = r#"
            sealed class Shape permits Circle, Rect {
            }
            class Circle extends Shape {}
        "#;
        let file = parse(src);
        assert_eq!(file.classes[0].name, "Shape");
        assert_eq!(file.classes[1].name, "Circle");
    }

    #[test]
    fn parse_yield_stmt() {
        let src = r#"
            class T {
                void f() {
                    yield 42;
                }
            }
        "#;
        let file = parse(src);
        let Member::Method(m) = &file.classes[0].members[0] else { panic!() };
        if let Stmt::Yield(Expr::IntLit(42)) = &m.body.as_ref().unwrap().0[0] {
            // ok
        } else { panic!("expected yield 42"); }
    }
}
