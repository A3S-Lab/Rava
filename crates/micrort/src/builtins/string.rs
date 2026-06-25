//! Java String instance methods and static String methods.

use super::format::{fnv, format_java_string, regex_lite_match, regex_replace};
use crate::rir_interp::RVal;
use rava_common::error::Result;
use std::cell::RefCell;
use std::rc::Rc;

/// Static String methods dispatched by func_id.
pub fn dispatch_static(func_id: u32, args: &[RVal]) -> Option<Result<RVal>> {
    match func_id {
        id if id == fnv("String.format") => Some(Ok(RVal::Str(format_java_string(args)))),
        id if id == fnv("String.join") => {
            let delim = args.first().map(|v| v.to_display()).unwrap_or_default();
            if let Some(RVal::Array(arr)) = args.get(1) {
                let s = arr
                    .borrow()
                    .iter()
                    .map(|v| v.to_display())
                    .collect::<Vec<_>>()
                    .join(&delim);
                return Some(Ok(RVal::Str(s)));
            }
            let parts: Vec<String> = args[1..].iter().map(|v| v.to_display()).collect();
            Some(Ok(RVal::Str(parts.join(&delim))))
        }
        id if id == fnv("String.valueOf") || id == fnv("String.copyValueOf") => {
            match args.first() {
                // valueOf(char[]) / copyValueOf(char[]) → the characters joined ("hi", not "[h, i]").
                Some(RVal::Array(arr)) => {
                    let joined: String = arr.borrow().iter().map(|v| v.to_display()).collect();
                    Some(Ok(RVal::Str(joined)))
                }
                Some(v) => Some(Ok(RVal::Str(v.to_display()))),
                None => Some(Ok(RVal::Str("null".into()))),
            }
        }
        _ => None,
    }
}

/// Instance String methods dispatched by method name.
pub fn dispatch_named(s: &str, method: &str, args: &[RVal]) -> Option<Result<RVal>> {
    match method {
        // Java's length() counts UTF-16 code units, not UTF-8 bytes (so "résumé" is 6, not 12).
        "length" => Some(Ok(RVal::Int(s.encode_utf16().count() as i64))),
        "isEmpty" => Some(Ok(RVal::Bool(s.is_empty()))),
        "isBlank" => Some(Ok(RVal::Bool(s.trim().is_empty()))),
        "toUpperCase" => Some(Ok(RVal::Str(s.to_uppercase()))),
        "toLowerCase" => Some(Ok(RVal::Str(s.to_lowercase()))),
        "trim" => Some(Ok(RVal::Str(s.trim().to_string()))),
        "strip" => Some(Ok(RVal::Str(s.trim().to_string()))),
        "stripLeading" => Some(Ok(RVal::Str(s.trim_start().to_string()))),
        "stripTrailing" => Some(Ok(RVal::Str(s.trim_end().to_string()))),
        "intern" => Some(Ok(RVal::Str(s.to_string()))),
        "toString" => Some(Ok(RVal::Str(s.to_string()))),
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
            let end = args.get(1).map(|v| v.as_int()).unwrap_or(s.len() as i64) as usize;
            Some(Ok(RVal::Str(
                s.get(start..end.min(s.len())).unwrap_or("").to_string(),
            )))
        }
        "contains" => Some(Ok(RVal::Bool(
            s.contains(
                args.first()
                    .map(|v| v.to_display())
                    .unwrap_or_default()
                    .as_str(),
            ),
        ))),
        "startsWith" => Some(Ok(RVal::Bool(
            s.starts_with(
                args.first()
                    .map(|v| v.to_display())
                    .unwrap_or_default()
                    .as_str(),
            ),
        ))),
        "endsWith" => Some(Ok(RVal::Bool(
            s.ends_with(
                args.first()
                    .map(|v| v.to_display())
                    .unwrap_or_default()
                    .as_str(),
            ),
        ))),
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
            let to = args.get(1).map(|v| v.to_display()).unwrap_or_default();
            Some(Ok(RVal::Str(s.replace(from.as_str(), to.as_str()))))
        }
        "replaceAll" => {
            let pat = args.first().map(|v| v.to_display()).unwrap_or_default();
            let to = args.get(1).map(|v| v.to_display()).unwrap_or_default();
            Some(Ok(RVal::Str(regex_replace(&pat, s, &to, true))))
        }
        "replaceFirst" => {
            let pat = args.first().map(|v| v.to_display()).unwrap_or_default();
            let to = args.get(1).map(|v| v.to_display()).unwrap_or_default();
            Some(Ok(RVal::Str(regex_replace(&pat, s, &to, false))))
        }
        "indexOf" => {
            let arg = args.first().cloned().unwrap_or(RVal::Null);
            let pat = match &arg {
                RVal::Int(n) => char::from_u32(*n as u32)
                    .map(|c| c.to_string())
                    .unwrap_or_default(),
                _ => arg.to_display(),
            };
            let from = args.get(1).map(|v| v.as_int()).unwrap_or(0).max(0) as usize;
            let result = if from == 0 {
                s.find(pat.as_str()).map(|i| i as i64).unwrap_or(-1)
            } else {
                let chars: Vec<char> = s.chars().collect();
                if from >= chars.len() {
                    -1i64
                } else {
                    let sub: String = chars[from..].iter().collect();
                    sub.find(pat.as_str())
                        .map(|byte_off| (from + sub[..byte_off].chars().count()) as i64)
                        .unwrap_or(-1)
                }
            };
            Some(Ok(RVal::Int(result)))
        }
        "lastIndexOf" => {
            let pat = args.first().map(|v| v.to_display()).unwrap_or_default();
            Some(Ok(RVal::Int(
                s.rfind(pat.as_str()).map(|i| i as i64).unwrap_or(-1),
            )))
        }
        "split" => {
            let pat = args.first().map(|v| v.to_display()).unwrap_or_default();
            let limit = args.get(1).map(|v| v.as_int()).unwrap_or(0);
            let parts: Vec<RVal> = match super::format::compile_java_regex(&pat) {
                Some(re) => {
                    if limit > 0 {
                        // At most `limit` parts (trailing empties kept).
                        re.splitn(s, limit as usize)
                            .map(|x| RVal::Str(x.to_string()))
                            .collect()
                    } else {
                        let mut v: Vec<String> = re.split(s).map(|x| x.to_string()).collect();
                        // limit == 0 drops trailing empty strings (Java semantics).
                        if limit == 0 {
                            while v.len() > 1 && v.last().is_some_and(|x| x.is_empty()) {
                                v.pop();
                            }
                        }
                        v.into_iter().map(RVal::Str).collect()
                    }
                }
                None => vec![RVal::Str(s.to_string())],
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
            Some(Ok(RVal::Int(java_string_compare(s, &other))))
        }
        "compareToIgnoreCase" => {
            let other = args.first().map(|v| v.to_display()).unwrap_or_default();
            Some(Ok(RVal::Int(java_string_compare(
                &s.to_lowercase(),
                &other.to_lowercase(),
            ))))
        }
        "hashCode" => {
            let mut h: i32 = 0;
            for c in s.chars() {
                h = h.wrapping_mul(31).wrapping_add(c as i32);
            }
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
            let prefix = if n > 0 {
                " ".repeat(n as usize)
            } else {
                String::new()
            };
            let result = s
                .lines()
                .map(|l| format!("{}{}", prefix, l))
                .collect::<Vec<_>>()
                .join("\n");
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
                .chain(args.iter().cloned())
                .collect();
            Some(Ok(RVal::Str(format_java_string(&fmt_args))))
        }
        "translateEscapes" => {
            let result = s
                .replace("\\n", "\n")
                .replace("\\t", "\t")
                .replace("\\r", "\r")
                .replace("\\\\", "\\");
            Some(Ok(RVal::Str(result)))
        }
        "regionMatches" => Some(Ok(RVal::Bool(false))), // simplified stub
        _ => None,
    }
}

/// Java's `String.compareTo`: the difference of the first differing UTF-16 code unit, or the
/// length difference if one string is a prefix of the other (not just the sign, -1/0/1).
fn java_string_compare(a: &str, b: &str) -> i64 {
    let av: Vec<u16> = a.encode_utf16().collect();
    let bv: Vec<u16> = b.encode_utf16().collect();
    for k in 0..av.len().min(bv.len()) {
        if av[k] != bv[k] {
            return av[k] as i64 - bv[k] as i64;
        }
    }
    av.len() as i64 - bv.len() as i64
}
