//! java.util.Scanner builtin.
//!
//! Scanners are encoded as `__scanner__<pos>@@<input>` strings.

use rava_common::error::Result;
use crate::rir_interp::RVal;
use std::cell::RefCell;
use super::format::fnv;

thread_local! {
    /// After a Scanner method call, the updated scanner encoding is stored here.
    pub static LAST_SCANNER: RefCell<Option<String>> = RefCell::new(None);
}

pub fn dispatch(func_id: u32, args: &[RVal]) -> Option<Result<RVal>> {
    // new Scanner(String) or new Scanner(System.in)
    if func_id == fnv("Scanner") || func_id == fnv("Scanner.<init>") {
        let input = args.first().map(|v| v.to_display()).unwrap_or_default();
        // If it's System.in (encoded as "__stdin__"), start with empty buffer
        let content = if input == "__stdin__" || input.contains("System.in") {
            String::new()
        } else {
            input
        };
        return Some(Ok(RVal::Str(encode_scanner(0, &content))));
    }
    None
}

pub fn dispatch_scanner(enc: &str, method: &str, _args: &[RVal]) -> Option<Result<RVal>> {
    let (pos, input) = decode_scanner(enc);

    match method {
        "hasNext" | "hasNextLine" => {
            let remaining = input[pos..].trim_start();
            Some(Ok(RVal::Bool(!remaining.is_empty())))
        }
        "hasNextInt" => {
            let token = next_token(&input, pos);
            Some(Ok(RVal::Bool(token.parse::<i64>().is_ok())))
        }
        "hasNextDouble" | "hasNextFloat" => {
            let token = next_token(&input, pos);
            Some(Ok(RVal::Bool(token.parse::<f64>().is_ok())))
        }
        "nextLine" => {
            let slice = &input[pos..];
            let (line, consumed) = if let Some(nl) = slice.find('\n') {
                (&slice[..nl], nl + 1)
            } else {
                (slice, slice.len())
            };
            let new_enc = encode_scanner(pos + consumed, &input);
            LAST_SCANNER.with(|ls| *ls.borrow_mut() = Some(new_enc));
            Some(Ok(RVal::Str(line.to_string())))
        }
        "next" => {
            let slice = &input[pos..];
            let trimmed_offset = slice.len() - slice.trim_start().len();
            let trimmed = &slice[trimmed_offset..];
            let end = trimmed.find(|c: char| c.is_whitespace()).unwrap_or(trimmed.len());
            let token = &trimmed[..end];
            let new_pos = pos + trimmed_offset + end;
            let new_enc = encode_scanner(new_pos, &input);
            LAST_SCANNER.with(|ls| *ls.borrow_mut() = Some(new_enc));
            Some(Ok(RVal::Str(token.to_string())))
        }
        "nextInt" => {
            let token = next_token(&input, pos);
            let new_pos = advance_past_token(&input, pos);
            let new_enc = encode_scanner(new_pos, &input);
            LAST_SCANNER.with(|ls| *ls.borrow_mut() = Some(new_enc));
            Some(Ok(RVal::Int(token.parse::<i64>().unwrap_or(0))))
        }
        "nextLong" => {
            let token = next_token(&input, pos);
            let new_pos = advance_past_token(&input, pos);
            let new_enc = encode_scanner(new_pos, &input);
            LAST_SCANNER.with(|ls| *ls.borrow_mut() = Some(new_enc));
            Some(Ok(RVal::Int(token.parse::<i64>().unwrap_or(0))))
        }
        "nextDouble" | "nextFloat" => {
            let token = next_token(&input, pos);
            let new_pos = advance_past_token(&input, pos);
            let new_enc = encode_scanner(new_pos, &input);
            LAST_SCANNER.with(|ls| *ls.borrow_mut() = Some(new_enc));
            Some(Ok(RVal::Float(token.parse::<f64>().unwrap_or(0.0))))
        }
        "nextBoolean" => {
            let token = next_token(&input, pos);
            let new_pos = advance_past_token(&input, pos);
            let new_enc = encode_scanner(new_pos, &input);
            LAST_SCANNER.with(|ls| *ls.borrow_mut() = Some(new_enc));
            Some(Ok(RVal::Bool(token.eq_ignore_ascii_case("true"))))
        }
        "close" => Some(Ok(RVal::Void)),
        "useDelimiter" => {
            // Return same scanner (delimiter customization not fully supported)
            Some(Ok(RVal::Str(enc.to_string())))
        }
        _ => None,
    }
}

fn encode_scanner(pos: usize, input: &str) -> String {
    format!("__scanner__{}@@{}", pos, input)
}

fn decode_scanner(enc: &str) -> (usize, String) {
    let inner = enc.strip_prefix("__scanner__").unwrap_or(enc);
    if let Some(sep) = inner.find("@@") {
        let pos = inner[..sep].parse().unwrap_or(0);
        let input = inner[sep + 2..].to_string();
        (pos, input)
    } else {
        (0, inner.to_string())
    }
}

fn next_token(input: &str, pos: usize) -> String {
    let slice = &input[pos..];
    let trimmed = slice.trim_start();
    let end = trimmed.find(|c: char| c.is_whitespace()).unwrap_or(trimmed.len());
    trimmed[..end].to_string()
}

fn advance_past_token(input: &str, pos: usize) -> usize {
    let slice = &input[pos..];
    let trimmed_offset = slice.len() - slice.trim_start().len();
    let trimmed = &slice[trimmed_offset..];
    let end = trimmed.find(|c: char| c.is_whitespace()).unwrap_or(trimmed.len());
    pos + trimmed_offset + end
}
