//! Java printf/format string support.

use crate::rir_interp::RVal;

/// Format a Java-style format string with arguments.
/// Supports: %s, %d, %f, %n, %%, %b, %c, %x, %o.
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
            while i < chars.len() && matches!(chars[i], '-' | '+' | ' ' | '0' | '#') {
                flags.push(chars[i]);
                i += 1;
            }
            let mut width: Option<usize> = None;
            let w_start = i;
            while i < chars.len() && chars[i].is_ascii_digit() { i += 1; }
            if i > w_start {
                width = chars[w_start..i].iter().collect::<String>().parse().ok();
            }
            let mut precision: Option<usize> = None;
            if i < chars.len() && chars[i] == '.' {
                i += 1;
                let p_start = i;
                while i < chars.len() && chars[i].is_ascii_digit() { i += 1; }
                precision = Some(chars[p_start..i].iter().collect::<String>().parse().unwrap_or(6));
            }
            if i >= chars.len() { break; }
            match chars[i] {
                's' => {
                    let val = args.get(arg_idx).map(|v| v.to_display()).unwrap_or_default();
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
                    if let Some(w) = width {
                        if flags.contains('0') {
                            result.push_str(&format!("{:0>width$}", val, width = w));
                        } else if flags.contains('-') {
                            result.push_str(&format!("{:<width$}", val, width = w));
                        } else {
                            result.push_str(&format!("{:width$}", val, width = w));
                        }
                    } else {
                        result.push_str(&val.to_string());
                    }
                    arg_idx += 1;
                }
                'f' => {
                    let val = args.get(arg_idx).map(|v| v.as_float()).unwrap_or(0.0);
                    let prec = precision.unwrap_or(6);
                    result.push_str(&format!("{:.prec$}", val, prec = prec));
                    arg_idx += 1;
                }
                'e' => {
                    let val = args.get(arg_idx).map(|v| v.as_float()).unwrap_or(0.0);
                    let prec = precision.unwrap_or(6);
                    result.push_str(&format!("{:.prec$e}", val, prec = prec));
                    arg_idx += 1;
                }
                'b' => {
                    let val = args.get(arg_idx).map(|v| v.is_truthy()).unwrap_or(false);
                    result.push_str(&val.to_string());
                    arg_idx += 1;
                }
                'c' => {
                    let val = args.get(arg_idx).map(|v| v.as_int()).unwrap_or(0);
                    if let Some(c) = char::from_u32(val as u32) { result.push(c); }
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
                _ => { result.push('%'); result.push(chars[i]); }
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
    for b in name.bytes() { h ^= b as u32; h = h.wrapping_mul(16777619); }
    h
}

/// Simple pseudo-random f64 in [0.0, 1.0).
pub fn rand_f64() -> f64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::cell::Cell;
    thread_local! {
        static SEED: Cell<u64> = Cell::new(0);
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

/// Simple regex-like match for String.matches().
pub fn regex_lite_match(pattern: &str, s: &str) -> bool {
    if pattern == ".*" { return true; }
    if pattern.starts_with('^') && pattern.ends_with('$') {
        return s == &pattern[1..pattern.len()-1];
    }
    s == pattern
}
