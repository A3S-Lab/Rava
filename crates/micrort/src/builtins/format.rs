//! Java printf/format string support.

use crate::rir_interp::RVal;

/// Format a Java-style format string with arguments.
/// Supports: %s, %d, %f, %n, %%, %b, %c, %x, %o.
/// Insert ',' every three digits from the right (Java `%,d` grouping).
fn group_thousands(digits: &str) -> String {
    let len = digits.len();
    let mut out = String::with_capacity(len + len / 3);
    for (idx, ch) in digits.chars().enumerate() {
        if idx > 0 && (len - idx) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

pub fn format_java_string(args: &[RVal]) -> String {
    let fmt = args.first().map(|v| v.to_display()).unwrap_or_default();
    let mut result = String::new();
    let mut arg_idx = 1usize;
    let chars: Vec<char> = fmt.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '%' && i + 1 < chars.len() {
            i += 1;
            let mut flags = String::new();
            while i < chars.len() && matches!(chars[i], '-' | '+' | ' ' | '0' | '#' | ',') {
                flags.push(chars[i]);
                i += 1;
            }
            let mut width: Option<usize> = None;
            let w_start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            if i > w_start {
                width = chars[w_start..i].iter().collect::<String>().parse().ok();
            }
            let mut precision: Option<usize> = None;
            if i < chars.len() && chars[i] == '.' {
                i += 1;
                let p_start = i;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
                precision = Some(
                    chars[p_start..i]
                        .iter()
                        .collect::<String>()
                        .parse()
                        .unwrap_or(6),
                );
            }
            if i >= chars.len() {
                break;
            }
            match chars[i] {
                's' => {
                    let val = args
                        .get(arg_idx)
                        .map(|v| v.to_display())
                        .unwrap_or_default();
                    if let Some(w) = width {
                        if flags.contains('-') {
                            result.push_str(&format!("{:<width$}", val, width = w));
                        } else {
                            result.push_str(&format!("{:>width$}", val, width = w));
                        }
                    } else {
                        result.push_str(&val);
                    }
                    arg_idx += 1;
                }
                'd' => {
                    let val = args.get(arg_idx).map(|v| v.as_int()).unwrap_or(0);
                    arg_idx += 1;
                    let digits = if flags.contains(',') {
                        group_thousands(&val.unsigned_abs().to_string())
                    } else {
                        val.unsigned_abs().to_string()
                    };
                    let sign = if val < 0 {
                        "-"
                    } else if flags.contains('+') {
                        "+"
                    } else if flags.contains(' ') {
                        " "
                    } else {
                        ""
                    };
                    let s = match width {
                        Some(w) if flags.contains('-') => {
                            format!("{:<width$}", format!("{sign}{digits}"), width = w)
                        }
                        Some(w) if flags.contains('0') => {
                            let pad = w.saturating_sub(sign.len() + digits.len());
                            format!("{sign}{}{digits}", "0".repeat(pad))
                        }
                        Some(w) => format!("{:>width$}", format!("{sign}{digits}"), width = w),
                        None => format!("{sign}{digits}"),
                    };
                    result.push_str(&s);
                }
                'f' => {
                    let val = args.get(arg_idx).map(|v| v.as_float()).unwrap_or(0.0);
                    let prec = precision.unwrap_or(6);
                    if let Some(w) = width {
                        if flags.contains('-') {
                            result.push_str(&format!(
                                "{:<width$.prec$}",
                                val,
                                width = w,
                                prec = prec
                            ));
                        } else {
                            result.push_str(&format!(
                                "{:>width$.prec$}",
                                val,
                                width = w,
                                prec = prec
                            ));
                        }
                    } else {
                        result.push_str(&format!("{:.prec$}", val, prec = prec));
                    }
                    arg_idx += 1;
                }
                'e' => {
                    let val = args.get(arg_idx).map(|v| v.as_float()).unwrap_or(0.0);
                    let prec = precision.unwrap_or(6);
                    // Rust uses e5, Java uses e+05 — reformat
                    let raw = format!("{:.prec$e}", val, prec = prec);
                    let formatted = if let Some(e_pos) = raw.find('e') {
                        let mantissa = &raw[..e_pos];
                        let exp_str = &raw[e_pos + 1..];
                        let exp: i32 = exp_str.parse().unwrap_or(0);
                        format!("{}e{:+03}", mantissa, exp)
                    } else {
                        raw
                    };
                    result.push_str(&formatted);
                    arg_idx += 1;
                }
                'b' => {
                    let val = args.get(arg_idx).map(|v| v.is_truthy()).unwrap_or(false);
                    result.push_str(&val.to_string());
                    arg_idx += 1;
                }
                'c' => {
                    let val = args.get(arg_idx).map(|v| v.as_int()).unwrap_or(0);
                    if let Some(c) = char::from_u32(val as u32) {
                        result.push(c);
                    }
                    arg_idx += 1;
                }
                'x' => {
                    let val = args.get(arg_idx).map(|v| v.as_int()).unwrap_or(0);
                    if let Some(w) = width {
                        if flags.contains('0') {
                            result.push_str(&format!("{:0>width$x}", val as u64, width = w));
                        } else {
                            result.push_str(&format!("{:width$x}", val as u64, width = w));
                        }
                    } else {
                        result.push_str(&format!("{:x}", val as u64));
                    }
                    arg_idx += 1;
                }
                'X' => {
                    let val = args.get(arg_idx).map(|v| v.as_int()).unwrap_or(0);
                    result.push_str(&format!("{:X}", val as u64));
                    arg_idx += 1;
                }
                'o' => {
                    let val = args.get(arg_idx).map(|v| v.as_int()).unwrap_or(0);
                    result.push_str(&format!("{:o}", val as u64));
                    arg_idx += 1;
                }
                'n' => result.push('\n'),
                '%' => result.push('%'),
                _ => {
                    result.push('%');
                    result.push(chars[i]);
                }
            }
        } else {
            result.push(chars[i]);
        }
        i += 1;
    }
    result
}

/// FNV-1a hash for builtin name lookup.
pub fn fnv(name: &str) -> u32 {
    let mut h: u32 = 2166136261;
    for b in name.bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(16777619);
    }
    h
}

/// Simple pseudo-random f64 in [0.0, 1.0).
pub fn rand_f64() -> f64 {
    use std::cell::Cell;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    thread_local! {
        static SEED: Cell<u64> = const { Cell::new(0) };
    }
    SEED.with(|s| {
        let mut val = s.get();
        if val == 0 {
            let mut h = DefaultHasher::new();
            std::time::SystemTime::now().hash(&mut h);
            val = h.finish();
        }
        val ^= val << 13;
        val ^= val >> 7;
        val ^= val << 17;
        s.set(val);
        (val as f64) / (u64::MAX as f64)
    })
}

/// NFA-based regex engine supporting Java regex subset.
/// Supports: `.`, `*`, `+`, `?`, `^`, `$`, `[]`, `[^]`, `\d`, `\w`, `\s`,
///           `\D`, `\W`, `\S`, `{n}`, `{n,}`, `{n,m}`, `|`, `()`, `(?:)`.
/// Compile a Java regex into a Rust `regex::Regex` (cached). Java and Rust regex syntax mostly
/// agree (`\d \w \s [...] (...) * + ? | ^ $ {n,m}` all match); the main fixup is named groups
/// `(?<n>..)` → `(?P<n>..)`. Returns `None` for patterns using engine features Rust lacks
/// (lookaround, backreferences) so callers degrade gracefully instead of failing hard.
pub fn compile_java_regex(pattern: &str) -> Option<regex::Regex> {
    thread_local! {
        static CACHE: std::cell::RefCell<std::collections::HashMap<String, Option<regex::Regex>>> =
            std::cell::RefCell::new(std::collections::HashMap::new());
    }
    CACHE.with(|c| {
        c.borrow_mut()
            .entry(pattern.to_string())
            .or_insert_with(|| regex::Regex::new(&pattern.replace("(?<", "(?P<")).ok())
            .clone()
    })
}

pub fn regex_lite_match(pattern: &str, text: &str) -> bool {
    // Java String.matches() / Matcher.matches() require the whole input to match.
    match compile_java_regex(&format!("^(?:{pattern})$")) {
        Some(re) => re.is_match(text),
        None => false,
    }
}

/// Find the first match of `pattern` in `text`, returning (start, end) char indices.
pub fn regex_find_span(pattern: &str, text: &str) -> Option<(usize, usize)> {
    compile_java_regex(pattern)?
        .find(text)
        .map(|m| (m.start(), m.end()))
}

/// Split on the regex. Java's default `split` drops trailing empty strings.
pub fn regex_split(pattern: &str, text: &str) -> Vec<String> {
    match compile_java_regex(pattern) {
        Some(re) => {
            let mut parts: Vec<String> = re.split(text).map(|s| s.to_string()).collect();
            while parts.len() > 1 && parts.last().is_some_and(|s| s.is_empty()) {
                parts.pop();
            }
            parts
        }
        None => vec![text.to_string()],
    }
}

/// Replace first/all regex matches. `$1`-style group references in `replacement` are honored.
pub fn regex_replace(pattern: &str, text: &str, replacement: &str, all: bool) -> String {
    match compile_java_regex(pattern) {
        Some(re) => {
            if all {
                re.replace_all(text, replacement).into_owned()
            } else {
                re.replace(text, replacement).into_owned()
            }
        }
        None => text.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_regex_replace_digit() {
        assert_eq!(
            regex_replace(r"\d+", "hello world 123", "NUM", true),
            "hello world NUM"
        );
        assert_eq!(
            regex_replace(r"[a-z]+", "hello world 123", "X", false),
            "X world 123"
        );
        assert!(regex_lite_match(r".*\d+.*", "hello world 123"));
    }

    #[test]
    fn test_regex_replace_negated_class() {
        assert_eq!(
            regex_replace("[^a-z]", "hello world", "", true),
            "helloworld"
        );
        assert_eq!(
            regex_replace("[^a-z0-9]", "a man a plan", "", true),
            "amanaplan"
        );
    }
}
