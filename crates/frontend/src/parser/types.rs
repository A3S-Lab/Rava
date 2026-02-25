use rava_common::error::{RavaError, Result};
use crate::ast::*;
use crate::lexer::Token;
use super::Parser;

impl Parser {
    pub(crate) fn parse_type_expr(&mut self) -> Result<TypeExpr> {
        let name = self.parse_type_name()?;
        let mut dims = 0u8;
        while self.peek() == &Token::LBracket && self.peek2() == &Token::RBracket {
            self.advance(); self.advance();
            dims += 1;
        }
        Ok(TypeExpr { name, array_dims: dims })
    }

    pub(crate) fn parse_type_name(&mut self) -> Result<String> {
        let mut name = match self.advance().clone() {
            Token::Ident(s) => s,
            Token::Int      => "int".into(),
            Token::Long     => "long".into(),
            Token::Double   => "double".into(),
            Token::Float    => "float".into(),
            Token::Boolean  => "boolean".into(),
            Token::Byte     => "byte".into(),
            Token::Short    => "short".into(),
            Token::Char     => "char".into(),
            Token::Void     => "void".into(),
            Token::Var      => "var".into(),
            got => return Err(RavaError::Parse {
                location: format!("pos {}", self.pos),
                message: format!("expected type, got {:?}", got),
            }),
        };
        // qualified name
        while self.peek() == &Token::Dot {
            if matches!(self.peek2(), Token::Ident(_)) {
                self.advance();
                name.push('.');
                name.push_str(&self.expect_ident()?);
            } else { break; }
        }
        // skip generic params
        self.skip_type_params();
        Ok(name)
    }

    pub(crate) fn skip_type_params(&mut self) {
        if self.peek() == &Token::Lt {
            let mut depth = 0i32;
            loop {
                match self.advance() {
                    Token::Lt  => depth += 1,
                    Token::Gt  => {
                        depth -= 1;
                        if depth <= 0 { break; }
                    }
                    // Handle >> as two closing >
                    Token::Shr => {
                        depth -= 2;
                        if depth <= 0 { break; }
                    }
                    Token::Eof => break,
                    _ => {}
                }
            }
        }
    }

    pub(crate) fn skip_throws(&mut self) {
        if self.peek() == &Token::Throws {
            self.advance();
            self.parse_type_name().ok();
            while self.eat(&Token::Comma) { self.parse_type_name().ok(); }
        }
    }

    pub(crate) fn parse_params(&mut self) -> Result<Vec<Param>> {
        self.expect(&Token::LParen)?;
        let mut params = Vec::new();
        while self.peek() != &Token::RParen && self.peek() != &Token::Eof {
            let annotations = self.parse_annotations()?;
            // skip final
            self.eat(&Token::Final);
            let ty = self.parse_type_expr()?;
            let variadic = self.eat(&Token::Ellipsis);
            let name = self.expect_ident()?;
            params.push(Param { name, ty, variadic, annotations });
            if !self.eat(&Token::Comma) { break; }
        }
        self.expect(&Token::RParen)?;
        Ok(params)
    }
}
