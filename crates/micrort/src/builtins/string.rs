//! Java String instance methods and static String methods.

use rava_common::error::Result;
use crate::rir_interp::RVal;
use std::cell::RefCell;
use std::rc::Rc;
use super::format::{fnv, format_java_string, regex_lite_match, regex_replace, regex_split};

/// Static String methods dispatched by func_id.
pub fn dispatch_static(func_id: u32, args: &[RVal]) -> Option<Result<RVal>> {
    match func_id {
        id if id == fnv("String.format") => Some(Ok(RVal::Str(format_java_string(args)))),
        id if id == fnv("String.join") => {
            let delim = args.first().map(|v| v.to_display()).unwrap_or_default();
            if let Some(RVal::Array(arr)) = args.get(1) {
                let s = arr.borrow().iter().map(|v| v.to_display()).collect::<Vec<_>>().join(&delim);
                return Some(Ok(RVal::Str(s)));
            }
            let parts: Vec<String> = args[1..].iter().map(|v| v.to_display()).collect();
            Some(Ok(RVal::Str(parts.join(&delim))))
        }
        id if id == fnv("String.copyValueOf") => Some(Ok(RVal::Str(args.first().map(|v| v.to_display()).unwrap_or_default()))),
        _ => None,
    }
}

/// Instance String methods dispatched by method name.
pub fn dispatch_named(s: &str, method: &str, args: &[RVal]) -> Option<Result<RVal>> {
    match method {
        "length"         => Some(Ok(RVal::Int(s.len() as i64))),
        "isEmpty"        => Some(Ok(RVal::Bool(s.is_empty()))),
        "isBlank"        => Some(Ok(RVal::Bool(s.trim().is_empty()))),
        "toUpperCase"    => Some(Ok(RVal::Str(s.to_uppercase()))),
        "toLowerCase"    => Some(Ok(RVal::Str(s.to_lowercase()))),
        "trim"           => Some(Ok(RVal::Str(s.trim().to_string()))),
        "strip"          => Some(Ok(RVal::Str(s.trim().to_string()))),
        "stripLeading"   => Some(Ok(RVal::Str(s.trim_start().to_string()))),
        "stripTrailing"  => Some(Ok(RVal::Str(s.trim_end().to_string()))),
        "intern"         => Some(Ok(RVal::Str(s.to_string()))),
        "toString"       => Some(Ok(RVal::Str(s.to_string()))),
        "charAt" => {
            let i = args.first().map(|v| v.as_int()).unwrap_or(0) as usize;
            Some(Ok(RVal::Str(s.chars().nth(i).unwrap_or('\0').to_string())))
        }
        "codePointAt" => {
            let i = args.first().map(|v| v.as_int()).unwrap_or(0) as usize;
            Some(Ok(RVal::Int(s.chars().nth(i).unwrap_or('\0') as i64)))
        }
        "substring" => {
            let start = args.first().map(|v| v.as_int()).unwrap_or(0) as usize;
            let end   = args.get(1).map(|v| v.as_int()).unwrap_or(s.len() as i64) as usize;
            Some(Ok(RVal::Str(s.get(start..end.min(s.len())).unwrap_or("").to_string())))
        }
        "contains"   => Some(Ok(RVal::Bool(s.contains(args.first().map(|v| v.to_display()).unwrap_or_default().as_str())))),
        "startsWith" => Some(Ok(RVal::Bool(s.starts_with(args.first().map(|v| v.to_display()).unwrap_or_default().as_str())))),
        "endsWith"   => Some(Ok(RVal::Bool(s.ends_with(args.first().map(|v| v.to_display()).unwrap_or_default().as_str())))),
        "equals" => {
            let other = args.first().map(|v| v.to_display()).unwrap_or_default();
            Some(Ok(RVal::Bool(s == other.as_str())))
        }
        "equalsIgnoreCase" => {
            let other = args.first().map(|v| v.to_display()).unwrap_or_default();
            Some(Ok(RVal::Bool(s.to_lowercase() == other.to_lowercase())))
        }
        "contentEquals" => {
            let other = args.first().map(|v| v.to_display()).unwrap_or_default();
            Some(Ok(RVal::Bool(s == other.as_str())))
        }
        "replace" => {
            let from = args.first().map(|v| v.to_display()).unwrap_or_default();
            let to   = args.get(1).map(|v| v.to_display()).unwrap_or_default();
            Some(Ok(RVal::Str(s.replace(from.as_str(), to.as_str()))))
        }
        "replaceAll" => {
            let pat = args.first().map(|v| v.to_display()).unwrap_or_default();
            let to  = args.get(1).map(|v| v.to_display()).unwrap_or_default();
            Some(Ok(RVal::Str(regex_replace(&pat, s, &to, true))))
        }
        "replaceFirst" => {
            let pat = args.first().map(|v| v.to_display()).unwrap_or_default();
            let to  = args.get(1).map(|v| v.to_display()).unwrap_or_default();
            Some(Ok(RVal::Str(regex_replace(&pat, s, &to, false))))
        }
        "indexOf" => {
            let arg = args.first().cloned().unwrap_or(RVal::Null);
            let pat = match &arg {
                RVal::Int(n) => char::from_u32(*n as u32).map(|c| c.to_string()).unwrap_or_default(),
                _ => arg.to_display(),
            };
            Some(Ok(RVal::Int(s.find(pat.as_str()).map(|i| i as i64).unwrap_or(-1))))
        }
        "lastIndexOf" => {
            let pat = args.first().map(|v| v.to_display()).unwrap_or_default();
            Some(Ok(RVal::Int(s.rfind(pat.as_str()).map(|i| i as i64).unwrap_or(-1))))
        }
        "split" => {
            let pat   = args.first().map(|v| v.to_display()).unwrap_or_default();
            let limit = args.get(1).map(|v| v.as_int()).unwrap_or(0);
            // Handle common Java regex patterns
            let parts: Vec<RVal> = {
                let split_fn = |text: &str| -> Vec<String> {
                    match pat.as_str() {
                        "\\s+" | "\\s" => text.split_whitespace().map(|s| s.to_string()).collect(),
                        "\\d+" => text.split(|c: char| c.is_ascii_digit()).filter(|s| !s.is_empty()).map(|s| s.to_string()).collect(),
                        "\\w+" => text.split(|c: char| !c.is_alphanumeric() && c != '_').filter(|s| !s.is_empty()).map(|s| s.to_string()).collect(),
                        _ => {
                            // Strip anchors and treat as literal for simple patterns
                            let p = pat.replace("\\.", ".").replace("\\,", ",");
                            if limit > 0 {
                                text.splitn(limit as usize, p.as_str()).map(|s| s.to_string()).collect()
                            } else {
                                text.split(p.as_str()).map(|s| s.to_string()).collect()
                            }
                        }
                    }
                };
                split_fn(s).into_iter().map(|p| RVal::Str(p)).collect()
            };
            Some(Ok(RVal::Array(Rc::new(RefCell::new(parts)))))
        }
        "concat" => {
            let other = args.first().map(|v| v.to_display()).unwrap_or_default();
            Some(Ok(RVal::Str(format!("{}{}", s, other))))
        }
        "repeat" => {
            let n = args.first().map(|v| v.as_int()).unwrap_or(0).max(0) as usize;
            Some(Ok(RVal::Str(s.repeat(n))))
        }
        "compareTo" => {
            let other = args.first().map(|v| v.to_display()).unwrap_or_default();
            Some(Ok(RVal::Int(s.cmp(other.as_str()) as i64)))
        }
        "compareToIgnoreCase" => {
            let other = args.first().map(|v| v.to_display()).unwrap_or_default();
            Some(Ok(RVal::Int(s.to_lowercase().cmp(&other.to_lowercase()) as i64)))
        }
        "hashCode" => {
            let mut h: i32 = 0;
            for c in s.chars() { h = h.wrapping_mul(31).wrapping_add(c as i32); }
            Some(Ok(RVal::Int(h as i64)))
        }
        "toCharArray" => {
            let chars: Vec<RVal> = s.chars().map(|c| RVal::Str(c.to_string())).collect();
            Some(Ok(RVal::Array(Rc::new(RefCell::new(chars)))))
        }
        "chars" => {
            // chars() returns int code points (used in stream operations like indexOf)
            let chars: Vec<RVal> = s.chars().map(|c| RVal::Int(c as i64)).collect();
            Some(Ok(RVal::Array(Rc::new(RefCell::new(chars)))))
        }
        "getBytes" => {
            let bytes: Vec<RVal> = s.bytes().map(|b| RVal::Int(b as i64)).collect();
            Some(Ok(RVal::Array(Rc::new(RefCell::new(bytes)))))
        }
        "lines" => {
            let lines: Vec<RVal> = s.lines().map(|l| RVal::Str(l.to_string())).collect();
            Some(Ok(RVal::Array(Rc::new(RefCell::new(lines)))))
        }
        "indent" => {
            let n = args.first().map(|v| v.as_int()).unwrap_or(0);
            let prefix = if n > 0 { " ".repeat(n as usize) } else { String::new() };
            let result = s.lines().map(|l| format!("{}{}", prefix, l)).collect::<Vec<_>>().join("\n");
            Some(Ok(RVal::Str(result)))
        }
        "matches" => {
            let pat = args.first().map(|v| v.to_display()).unwrap_or_default();
            // Java String.matches() anchors the full string implicitly
            let anchored = format!("^(?:{})$", pat);
            Some(Ok(RVal::Bool(regex_lite_match(&anchored, s))))
        }
        "formatted" => {
            let fmt_args: Vec<RVal> = std::iter::once(RVal::Str(s.to_string()))
                .chain(args.iter().cloned()).collect();
            Some(Ok(RVal::Str(format_java_string(&fmt_args))))
        }
        "translateEscapes" => {
            let result = s.replace("\\n", "\n").replace("\\t", "\t")
                .replace("\\r", "\r").replace("\\\\", "\\");
            Some(Ok(RVal::Str(result)))
        }
        "regionMatches" => Some(Ok(RVal::Bool(false))), // simplified stub
        _ => None,
    }
}
