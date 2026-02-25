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
                    let plus = flags.contains('+');
                    if let Some(w) = width {
                        if flags.contains('0') {
                            let s = if plus && val >= 0 { format!("+{:0>width$}", val, width = w.saturating_sub(1)) } else { format!("{:0>width$}", val, width = w) };
                            result.push_str(&s);
                        } else if flags.contains('-') {
                            let s = if plus && val >= 0 { format!("+{:<width$}", val, width = w.saturating_sub(1)) } else { format!("{:<width$}", val, width = w) };
                            result.push_str(&s);
                        } else {
                            let s = if plus && val >= 0 { format!("+{:>width$}", val, width = w.saturating_sub(1)) } else { format!("{:>width$}", val, width = w) };
                            result.push_str(&s);
                        }
                    } else if plus && val >= 0 {
                        result.push_str(&format!("+{}", val));
                    } else {
                        result.push_str(&val.to_string());
                    }
                    arg_idx += 1;
                }
                'f' => {
                    let val = args.get(arg_idx).map(|v| v.as_float()).unwrap_or(0.0);
                    let prec = precision.unwrap_or(6);
                    if let Some(w) = width {
                        if flags.contains('-') {
                            result.push_str(&format!("{:<width$.prec$}", val, width = w, prec = prec));
                        } else {
                            result.push_str(&format!("{:>width$.prec$}", val, width = w, prec = prec));
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
                        let exp_str = &raw[e_pos+1..];
                        let exp: i32 = exp_str.parse().unwrap_or(0);
                        format!("{}e{:+03}", mantissa, exp)
                    } else { raw };
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

/// NFA-based regex engine supporting Java regex subset.
/// Supports: `.`, `*`, `+`, `?`, `^`, `$`, `[]`, `[^]`, `\d`, `\w`, `\s`,
///           `\D`, `\W`, `\S`, `{n}`, `{n,}`, `{n,m}`, `|`, `()`, `(?:)`.
pub fn regex_lite_match(pattern: &str, text: &str) -> bool {
    // Java String.matches() anchors the entire string
    let pat = if pattern.starts_with('^') && pattern.ends_with('$') {
        pattern[1..pattern.len()-1].to_string()
    } else if pattern.starts_with('^') {
        pattern[1..].to_string()
    } else {
        pattern.to_string()
    };
    let anchored = !pattern.starts_with(".*");
    if anchored {
        regex_match_full(&pat, text)
    } else {
        regex_match_anywhere(&pat, text)
    }
}

fn regex_match_full(pattern: &str, text: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    let pat_chars: Vec<char> = pattern.chars().collect();
    matches_here(&pat_chars, 0, &chars, 0, true)
}

fn regex_match_anywhere(pattern: &str, text: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    let pat_chars: Vec<char> = pattern.chars().collect();
    for start in 0..=chars.len() {
        if matches_here(&pat_chars, 0, &chars, start, false) {
            return true;
        }
    }
    false
}

/// Returns true if pattern[pi..] matches text[ti..] (optionally requiring full consumption).
fn matches_here(pat: &[char], pi: usize, text: &[char], ti: usize, full: bool) -> bool {
    if pi >= pat.len() {
        return if full { ti == text.len() } else { true };
    }

    // End anchor
    if pat[pi] == '$' && pi + 1 == pat.len() {
        return ti == text.len();
    }

    // Alternation: split on top-level `|`
    if let Some(alt_pos) = find_top_level_alt(pat, pi) {
        let left  = &pat[pi..alt_pos];
        let right = &pat[alt_pos+1..];
        return matches_here(left, 0, text, ti, full)
            || matches_here(right, 0, text, ti, full);
    }

    // Group: `(...)` or `(?:...)`
    if pat[pi] == '(' {
        let (group_end, inner_start) = find_group_end(pat, pi);
        let inner = &pat[inner_start..group_end];
        let after_group = group_end + 1;
        // Check for quantifier after group
        let (min, max, skip) = quantifier(pat, after_group);
        return match_repeat(inner, true, min, max, pat, after_group + skip, text, ti, full);
    }

    // Character class `[...]`
    if pat[pi] == '[' {
        if let Some(class_end) = find_class_end(pat, pi) {
            let class_pat = &pat[pi..=class_end];
            let after = class_end + 1;
            let (min, max, skip) = quantifier(pat, after);
            return match_char_repeat(class_pat, min, max, pat, after + skip, text, ti, full);
        }
    }

    // Escape sequences or literal with quantifier
    let (atom_len, atom_end) = atom_length(pat, pi);
    let atom = &pat[pi..pi+atom_len];
    let after = pi + atom_len;
    let (min, max, skip) = quantifier(pat, after);
    match_char_repeat(atom, min, max, pat, after + skip, text, ti, full)
}

fn match_repeat(
    inner: &[char], is_group: bool,
    min: usize, max: Option<usize>,
    rest_pat: &[char], rest_pi: usize,
    text: &[char], ti: usize, full: bool,
) -> bool {
    // Greedy: try max first, then back off
    let mut positions = vec![ti];
    let mut count = 0usize;
    let mut cur = ti;
    loop {
        if let Some(m) = max { if count >= m { break; } }
        // Try to match inner once at cur
        if is_group {
            // Find how far inner matches
            if let Some(next) = try_match_group(inner, text, cur) {
                cur = next;
                count += 1;
                positions.push(cur);
            } else {
                break;
            }
        } else {
            break;
        }
    }
    // Try from longest match down to min
    for i in (min..=positions.len().saturating_sub(1)).rev() {
        let pos = positions[i];
        if matches_here(rest_pat, rest_pi, text, pos, full) {
            return true;
        }
    }
    false
}

fn try_match_group(inner: &[char], text: &[char], ti: usize) -> Option<usize> {
    // Try to match inner at ti, return new position if successful
    for end in (ti..=text.len()).rev() {
        let slice = &text[ti..end];
        let slice_str: String = slice.iter().collect();
        if regex_match_full(&inner.iter().collect::<String>(), &slice_str) {
            return Some(end);
        }
    }
    None
}

fn match_char_repeat(
    atom: &[char],
    min: usize, max: Option<usize>,
    rest_pat: &[char], rest_pi: usize,
    text: &[char], ti: usize, full: bool,
) -> bool {
    // Greedy: collect all positions where atom matches consecutively
    let mut positions = vec![ti];
    let mut cur = ti;
    let mut count = 0usize;
    loop {
        if let Some(m) = max { if count >= m { break; } }
        if cur >= text.len() { break; }
        if char_matches(atom, text[cur]) {
            cur += 1;
            count += 1;
            positions.push(cur);
        } else {
            break;
        }
    }
    // Try from longest down to min
    for i in (min..=positions.len().saturating_sub(1)).rev() {
        let pos = positions[i];
        if matches_here(rest_pat, rest_pi, text, pos, full) {
            return true;
        }
    }
    false
}

/// Returns (min, max, chars_consumed) for a quantifier at pat[pi].
fn quantifier(pat: &[char], pi: usize) -> (usize, Option<usize>, usize) {
    if pi >= pat.len() { return (1, Some(1), 0); }
    match pat[pi] {
        '*' => (0, None, 1),
        '+' => (1, None, 1),
        '?' => (0, Some(1), 1),
        '{' => {
            // {n}, {n,}, {n,m}
            let mut i = pi + 1;
            let mut n_str = String::new();
            while i < pat.len() && pat[i].is_ascii_digit() { n_str.push(pat[i]); i += 1; }
            let n: usize = n_str.parse().unwrap_or(1);
            if i < pat.len() && pat[i] == '}' {
                return (n, Some(n), i - pi + 1);
            }
            if i < pat.len() && pat[i] == ',' {
                i += 1;
                let mut m_str = String::new();
                while i < pat.len() && pat[i].is_ascii_digit() { m_str.push(pat[i]); i += 1; }
                if i < pat.len() && pat[i] == '}' {
                    let max = if m_str.is_empty() { None } else { m_str.parse().ok() };
                    return (n, max, i - pi + 1);
                }
            }
            (1, Some(1), 0)
        }
        _ => (1, Some(1), 0),
    }
}

/// Returns (atom_len, atom_end) — how many pattern chars form one atom.
fn atom_length(pat: &[char], pi: usize) -> (usize, usize) {
    if pi >= pat.len() { return (0, pi); }
    if pat[pi] == '\\' && pi + 1 < pat.len() { return (2, pi + 2); }
    (1, pi + 1)
}

/// Check if a single text char matches an atom (literal, `.`, `\d`, etc.) or char class `[...]`.
fn char_matches(atom: &[char], c: char) -> bool {
    if atom.is_empty() { return false; }
    if atom[0] == '.' { return c != '\n'; }
    if atom[0] == '\\' && atom.len() >= 2 {
        return match atom[1] {
            'd' => c.is_ascii_digit(),
            'D' => !c.is_ascii_digit(),
            'w' => c.is_alphanumeric() || c == '_',
            'W' => !(c.is_alphanumeric() || c == '_'),
            's' => c.is_whitespace(),
            'S' => !c.is_whitespace(),
            'n' => c == '\n',
            'r' => c == '\r',
            't' => c == '\t',
            other => c == other,
        };
    }
    if atom[0] == '[' {
        return char_class_matches(atom, c);
    }
    atom[0] == c
}

/// Match a char against a `[...]` or `[^...]` class.
fn char_class_matches(class: &[char], c: char) -> bool {
    if class.len() < 2 { return false; }
    let (negate, start) = if class[1] == '^' { (true, 2) } else { (false, 1) };
    let end = class.len() - 1; // skip closing ]
    let mut i = start;
    let mut matched = false;
    while i < end {
        if i + 2 < end && class[i + 1] == '-' {
            // range a-z
            if c >= class[i] && c <= class[i + 2] { matched = true; }
            i += 3;
        } else if class[i] == '\\' && i + 1 < end {
            let atom = &class[i..i+2];
            if char_matches(atom, c) { matched = true; }
            i += 2;
        } else {
            if class[i] == c { matched = true; }
            i += 1;
        }
    }
    if negate { !matched } else { matched }
}

/// Find the end index of a `[...]` class starting at pi.
fn find_class_end(pat: &[char], pi: usize) -> Option<usize> {
    let mut i = pi + 1;
    if i < pat.len() && pat[i] == '^' { i += 1; }
    if i < pat.len() && pat[i] == ']' { i += 1; } // ] at start is literal
    while i < pat.len() {
        if pat[i] == ']' { return Some(i); }
        if pat[i] == '\\' { i += 1; } // skip escaped char
        i += 1;
    }
    None
}

/// Find the closing `)` of a group starting at pi, return (close_idx, inner_start).
fn find_group_end(pat: &[char], pi: usize) -> (usize, usize) {
    let inner_start = if pat.len() > pi + 2 && pat[pi+1] == '?' && pat[pi+2] == ':' {
        pi + 3
    } else {
        pi + 1
    };
    let mut depth = 1i32;
    let mut i = inner_start;
    while i < pat.len() {
        match pat[i] {
            '(' => depth += 1,
            ')' => { depth -= 1; if depth == 0 { return (i, inner_start); } }
            '\\' => { i += 1; }
            _ => {}
        }
        i += 1;
    }
    (pat.len(), inner_start)
}

/// Find top-level `|` in pat[pi..], ignoring those inside groups/classes.
fn find_top_level_alt(pat: &[char], pi: usize) -> Option<usize> {
    let mut depth = 0i32;
    let mut i = pi;
    while i < pat.len() {
        match pat[i] {
            '(' => depth += 1,
            ')' => depth -= 1,
            '[' => {
                // skip class
                i += 1;
                while i < pat.len() && pat[i] != ']' { i += 1; }
            }
            '\\' => { i += 1; }
            '|' if depth == 0 => return Some(i),
            _ => {}
        }
        i += 1;
    }
    None
}

/// Apply regex to find all non-overlapping matches, return split parts.
pub fn regex_split(pattern: &str, text: &str) -> Vec<String> {
    // Simple: split on literal pattern if no special chars, else fall back
    let special = |c: char| ".+*?[](){}\\|^$".contains(c);
    if !pattern.chars().any(special) {
        return text.split(pattern).map(|s| s.to_string()).collect();
    }
    // Fallback: split on each char boundary where pattern matches
    let chars: Vec<char> = text.chars().collect();
    let mut result = Vec::new();
    let mut last = 0usize;
    let mut i = 0usize;
    while i <= chars.len() {
        // Try to match pattern at position i
        let slice: String = chars[i..].iter().collect();
        if regex_match_full(pattern, &chars[i..i+1].iter().collect::<String>())
            && i < chars.len()
        {
            result.push(chars[last..i].iter().collect());
            last = i + 1;
        }
        i += 1;
    }
    result.push(chars[last..].iter().collect());
    result
}

/// Replace first/all regex matches.
pub fn regex_replace(pattern: &str, text: &str, replacement: &str, all: bool) -> String {
    let special = |c: char| ".+*?[](){}\\|^$".contains(c);
    if !pattern.chars().any(special) {
        if all {
            return text.replace(pattern, replacement);
        } else {
            return if let Some(pos) = text.find(pattern) {
                format!("{}{}{}", &text[..pos], replacement, &text[pos+pattern.len()..])
            } else {
                text.to_string()
            };
        }
    }
    // For regex patterns, do character-by-character matching
    let chars: Vec<char> = text.chars().collect();
    let mut result = String::new();
    let mut i = 0usize;
    let mut replaced = false;
    while i < chars.len() {
        if !all && replaced {
            result.push(chars[i]);
            i += 1;
            continue;
        }
        // Try to find longest match starting at position i
        let mut matched_len = 0usize;
        for end in (i+1..=chars.len()).rev() {
            let slice: String = chars[i..end].iter().collect();
            if regex_match_full(pattern, &slice) {
                matched_len = end - i;
                break;
            }
        }
        if matched_len > 0 {
            result.push_str(replacement);
            i += matched_len;
            replaced = true;
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_regex_replace_digit() {
        assert_eq!(regex_replace(r"\d+", "hello world 123", "NUM", true), "hello world NUM");
        assert_eq!(regex_replace(r"[a-z]+", "hello world 123", "X", false), "X world 123");
        assert!(regex_lite_match(r".*\d+.*", "hello world 123"));
    }
}
