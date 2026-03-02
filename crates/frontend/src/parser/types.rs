use super::Parser;
use crate::ast::*;
use crate::lexer::Token;
use rava_common::error::{RavaError, Result};

impl Parser {
    pub(crate) fn parse_type_expr(&mut self) -> Result<TypeExpr> {
        let name = self.parse_type_name()?;
        let generic_args_raw = self.parse_angle_raw();
        let generic_args = generic_args_raw
            .as_ref()
            .map(|raw| Self::parse_type_args_from_raw(raw));
        let mut dims = 0u8;
        while self.peek() == &Token::LBracket && self.peek2() == &Token::RBracket {
            self.advance();
            self.advance();
            dims += 1;
        }
        Ok(TypeExpr {
            name,
            array_dims: dims,
            generic_args_raw,
            generic_args,
        })
    }

    pub(crate) fn parse_type_name(&mut self) -> Result<String> {
        let mut name = match self.advance().clone() {
            Token::Ident(s) => s,
            Token::Int => "int".into(),
            Token::Long => "long".into(),
            Token::Double => "double".into(),
            Token::Float => "float".into(),
            Token::Boolean => "boolean".into(),
            Token::Byte => "byte".into(),
            Token::Short => "short".into(),
            Token::Char => "char".into(),
            Token::Void => "void".into(),
            Token::Var => "var".into(),
            got => {
                return Err(RavaError::Parse {
                    location: format!("pos {}", self.pos),
                    message: format!("expected type, got {:?}", got),
                })
            }
        };
        // qualified name
        while self.peek() == &Token::Dot {
            if matches!(self.peek2(), Token::Ident(_)) {
                self.advance();
                name.push('.');
                name.push_str(&self.expect_ident()?);
            } else {
                break;
            }
        }
        Ok(name)
    }

    pub(crate) fn parse_angle_raw(&mut self) -> Option<String> {
        if self.peek() != &Token::Lt {
            return None;
        }
        self.advance(); // consume '<'

        let mut depth = 1i32;
        let mut out = String::new();

        while depth > 0 && self.peek() != &Token::Eof {
            let tok = self.advance().clone();
            match tok {
                Token::Lt => {
                    depth += 1;
                    out.push('<');
                }
                Token::Gt => {
                    Self::consume_combined_gt(1, &mut depth, &mut out);
                }
                Token::Shr => {
                    Self::consume_combined_gt(2, &mut depth, &mut out);
                }
                Token::UShr => {
                    Self::consume_combined_gt(3, &mut depth, &mut out);
                }
                Token::Comma => out.push_str(", "),
                other => {
                    let text = self.token_text(&other);
                    let needs_space = !out.is_empty()
                        && out
                            .chars()
                            .last()
                            .map(|c| c.is_ascii_alphanumeric() || c == '>')
                            .unwrap_or(false)
                        && text
                            .chars()
                            .next()
                            .map(|c| c.is_ascii_alphanumeric())
                            .unwrap_or(false);
                    if needs_space {
                        out.push(' ');
                    }
                    out.push_str(&text);
                }
            }
        }

        Some(out.trim().to_string())
    }

    fn consume_combined_gt(n: u8, depth: &mut i32, out: &mut String) {
        for _ in 0..n {
            if *depth <= 0 {
                break;
            }
            *depth -= 1;
            if *depth > 0 {
                out.push('>');
            } else {
                break;
            }
        }
    }

    fn token_text(&self, tok: &Token) -> String {
        match tok {
            Token::Ident(s) => s.clone(),
            Token::Int => "int".into(),
            Token::Long => "long".into(),
            Token::Double => "double".into(),
            Token::Float => "float".into(),
            Token::Boolean => "boolean".into(),
            Token::Byte => "byte".into(),
            Token::Short => "short".into(),
            Token::Char => "char".into(),
            Token::Void => "void".into(),
            Token::Var => "var".into(),
            Token::Extends => "extends".into(),
            Token::Super => "super".into(),
            Token::Question => "?".into(),
            Token::BitAnd => "&".into(),
            Token::Dot => ".".into(),
            Token::LBracket => "[".into(),
            Token::RBracket => "]".into(),
            Token::At => "@".into(),
            _ => String::new(),
        }
    }

    fn parse_type_args_from_raw(raw: &str) -> Vec<TypeArg> {
        Self::split_top_level_commas(raw)
            .into_iter()
            .map(|arg| {
                let arg = arg.trim();
                if let Some(rest) = arg.strip_prefix('?') {
                    let rest = rest.trim_start();
                    if rest.is_empty() {
                        TypeArg::Wildcard
                    } else if let Some(bound) = rest.strip_prefix("extends") {
                        TypeArg::WildcardExtends(Self::parse_type_arg_type_expr(bound.trim()))
                    } else if let Some(bound) = rest.strip_prefix("super") {
                        TypeArg::WildcardSuper(Self::parse_type_arg_type_expr(bound.trim()))
                    } else {
                        TypeArg::Wildcard
                    }
                } else {
                    TypeArg::Type(Self::parse_type_arg_type_expr(arg))
                }
            })
            .collect()
    }

    fn parse_type_arg_type_expr(raw: &str) -> TypeExpr {
        let mut s = raw.trim().to_string();
        let mut dims = 0u8;
        while let Some(prefix) = s.strip_suffix("]") {
            if let Some(prefix2) = prefix.trim_end().strip_suffix("[") {
                dims += 1;
                s = prefix2.trim_end().to_string();
            } else {
                break;
            }
        }

        let (name, generic_args_raw) = Self::split_type_name_and_args(&s);
        let generic_args = generic_args_raw
            .as_ref()
            .map(|inner| Self::parse_type_args_from_raw(inner));

        TypeExpr {
            name,
            array_dims: dims,
            generic_args_raw,
            generic_args,
        }
    }

    fn split_type_name_and_args(raw: &str) -> (String, Option<String>) {
        let mut depth = 0i32;
        let mut start = None;
        let mut end = None;
        for (i, ch) in raw.char_indices() {
            match ch {
                '<' => {
                    if depth == 0 {
                        start = Some(i);
                    }
                    depth += 1;
                }
                '>' => {
                    if depth > 0 {
                        depth -= 1;
                        if depth == 0 {
                            end = Some(i);
                            break;
                        }
                    }
                }
                _ => {}
            }
        }

        match (start, end) {
            (Some(s), Some(e)) if e > s => {
                let name = raw[..s].trim().to_string();
                let inner = raw[s + 1..e].trim().to_string();
                (name, Some(inner))
            }
            _ => (raw.trim().to_string(), None),
        }
    }

    fn split_top_level_commas(raw: &str) -> Vec<String> {
        let mut parts = Vec::new();
        let mut buf = String::new();
        let mut depth = 0i32;
        for ch in raw.chars() {
            match ch {
                '<' => {
                    depth += 1;
                    buf.push(ch);
                }
                '>' => {
                    if depth > 0 {
                        depth -= 1;
                    }
                    buf.push(ch);
                }
                ',' if depth == 0 => {
                    parts.push(buf.trim().to_string());
                    buf.clear();
                }
                _ => buf.push(ch),
            }
        }
        if !buf.trim().is_empty() || raw.is_empty() {
            parts.push(buf.trim().to_string());
        }
        parts
    }

    pub(crate) fn skip_throws(&mut self) {
        if self.peek() == &Token::Throws {
            self.advance();
            self.parse_type_name().ok();
            self.parse_angle_raw();
            while self.eat(&Token::Comma) {
                self.parse_type_name().ok();
                self.parse_angle_raw();
            }
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
            params.push(Param {
                name,
                ty,
                variadic,
                annotations,
            });
            if !self.eat(&Token::Comma) {
                break;
            }
        }
        self.expect(&Token::RParen)?;
        Ok(params)
    }
}
