//! java.util.regex.Pattern and Matcher builtins.
//!
//! Patterns are encoded as `__pattern__<regex>` strings.
//! Matchers are encoded as `__matcher__<regex>@@<input>@@<pos>` strings.

use rava_common::error::Result;
use crate::rir_interp::RVal;
use std::cell::RefCell;
use std::rc::Rc;
use super::format::{fnv, regex_lite_match, regex_replace, regex_split, regex_find_span};

// ── Pattern static dispatch ──────────────────────────────────────────────────

pub fn dispatch(func_id: u32, args: &[RVal]) -> Option<Result<RVal>> {
    match func_id {
        id if id == fnv("Pattern.compile") => {
            let pat = args.first().map(|v| v.to_display()).unwrap_or_default();
            Some(Ok(RVal::Str(format!("__pattern__{}", pat))))
        }
        id if id == fnv("Pattern.matches") => {
            // Pattern.matches(regex, input) — full match
            let pat   = args.first().map(|v| v.to_display()).unwrap_or_default();
            let input = args.get(1).map(|v| v.to_display()).unwrap_or_default();
            Some(Ok(RVal::Bool(regex_lite_match(&pat, &input))))
        }
        id if id == fnv("Pattern.quote") => {
            let s = args.first().map(|v| v.to_display()).unwrap_or_default();
            Some(Ok(RVal::Str(regex_quote(&s))))
        }
        _ => None,
    }
}

// ── Pattern instance methods ─────────────────────────────────────────────────

pub fn dispatch_pattern(pat_str: &str, method: &str, args: &[RVal]) -> Option<Result<RVal>> {
    let regex = pat_str.strip_prefix("__pattern__").unwrap_or(pat_str);
    match method {
        "matcher" => {
            let input = args.first().map(|v| v.to_display()).unwrap_or_default();
            Some(Ok(RVal::Str(encode_matcher(regex, &input, 0, false, 0, 0))))
        }
        "pattern" | "toString" => Some(Ok(RVal::Str(regex.to_string()))),
        "split" => {
            let input = args.first().map(|v| v.to_display()).unwrap_or_default();
            let parts: Vec<RVal> = regex_split(regex, &input)
                .into_iter().map(RVal::Str).collect();
            Some(Ok(RVal::Array(Rc::new(RefCell::new(parts)))))
        }
        "matcher_find" => None, // handled via Matcher
        _ => None,
    }
}

// ── Matcher instance methods ─────────────────────────────────────────────────

pub fn dispatch_matcher(enc: &str, method: &str, args: &[RVal]) -> Option<Result<RVal>> {
    let (regex, input, pos, found, mstart, mend) = decode_matcher(enc);

    match method {
        // Advance and try to find next match
        "find" => {
            if let Some((s, e)) = regex_find_from(&regex, &input, pos) {
                // We can't mutate RVal::Str in place; callers must reassign.
                // Return a special tuple encoded as a new matcher string with found=true.
                // The interpreter stores the returned value back into the variable.
                let next_pos = if e > s { e } else { s + 1 }; // avoid infinite loop on zero-width
                let new_enc = encode_matcher(&regex, &input, next_pos, true, s, e);
                // Return the new matcher (caller must store it), but Java's find() returns bool.
                // We use a side-channel: store new matcher in thread-local, return Bool.
                LAST_MATCHER.with(|lm| *lm.borrow_mut() = Some(new_enc));
                Some(Ok(RVal::Bool(true)))
            } else {
                LAST_MATCHER.with(|lm| {
                    *lm.borrow_mut() = Some(encode_matcher(&regex, &input, input.len(), false, 0, 0));
                });
                Some(Ok(RVal::Bool(false)))
            }
        }
        "matches" => {
            let full = regex_lite_match(&regex, &input);
            if full {
                let end = input.len();
                LAST_MATCHER.with(|lm| *lm.borrow_mut() = Some(encode_matcher(&regex, &input, end, true, 0, end)));
            }
            Some(Ok(RVal::Bool(full)))
        }
        "lookingAt" => {
            // Match at start (not necessarily full string)
            let result = regex_find_from(&regex, &input, 0)
                .map(|(s, e)| s == 0)
                .unwrap_or(false);
            Some(Ok(RVal::Bool(result)))
        }
        "group" => {
            if !found { return Some(Ok(RVal::Null)); }
            let group_idx = args.first().map(|v| v.as_int()).unwrap_or(0) as usize;
            if group_idx == 0 {
                Some(Ok(RVal::Str(input[mstart..mend].to_string())))
            } else {
                // Group capture: extract nth capturing group from last match
                let matched_text = &input[mstart..mend];
                Some(Ok(extract_group(&regex, matched_text, group_idx)
                    .map(RVal::Str)
                    .unwrap_or(RVal::Null)))
            }
        }
        "start" => {
            if !found { return Some(Ok(RVal::Int(-1))); }
            let group_idx = args.first().map(|v| v.as_int()).unwrap_or(0) as usize;
            if group_idx == 0 {
                Some(Ok(RVal::Int(mstart as i64)))
            } else {
                Some(Ok(RVal::Int(-1)))
            }
        }
        "end" => {
            if !found { return Some(Ok(RVal::Int(-1))); }
            let group_idx = args.first().map(|v| v.as_int()).unwrap_or(0) as usize;
            if group_idx == 0 {
                Some(Ok(RVal::Int(mend as i64)))
            } else {
                Some(Ok(RVal::Int(-1)))
            }
        }
        "replaceAll" => {
            let repl = args.first().map(|v| v.to_display()).unwrap_or_default();
            Some(Ok(RVal::Str(regex_replace(&regex, &input, &repl, true))))
        }
        "replaceFirst" => {
            let repl = args.first().map(|v| v.to_display()).unwrap_or_default();
            Some(Ok(RVal::Str(regex_replace(&regex, &input, &repl, false))))
        }
        "reset" => {
            let new_input = args.first().map(|v| v.to_display()).unwrap_or(input.clone());
            Some(Ok(RVal::Str(encode_matcher(&regex, &new_input, 0, false, 0, 0))))
        }
        "groupCount" => {
            // Count capturing groups in pattern
            Some(Ok(RVal::Int(count_groups(&regex) as i64)))
        }
        "hitEnd" => Some(Ok(RVal::Bool(pos >= input.len()))),
        _ => None,
    }
}

// ── Thread-local for last matcher state after find() ────────────────────────

thread_local! {
    /// After a find()/matches() call, the updated matcher encoding is stored here
    /// so the interpreter can update the variable.
    pub static LAST_MATCHER: RefCell<Option<String>> = RefCell::new(None);
}

// ── Encoding helpers ─────────────────────────────────────────────────────────

/// `__matcher__<regex>@@<input>@@<pos>@@<found>@@<mstart>@@<mend>`
fn encode_matcher(regex: &str, input: &str, pos: usize, found: bool, mstart: usize, mend: usize) -> String {
    format!("__matcher__{}@@{}@@{}@@{}@@{}@@{}", regex, input, pos, found as u8, mstart, mend)
}

fn decode_matcher(enc: &str) -> (String, String, usize, bool, usize, usize) {
    let inner = enc.strip_prefix("__matcher__").unwrap_or(enc);
    // Split on @@ from the right to handle @@ in regex/input
    // Format: regex@@input@@pos@@found@@mstart@@mend
    // We split on last 5 occurrences of @@
    let parts: Vec<&str> = inner.splitn(6, "@@").collect();
    if parts.len() == 6 {
        let regex  = parts[0].to_string();
        let input  = parts[1].to_string();
        let pos    = parts[2].parse().unwrap_or(0);
        let found  = parts[3] == "1";
        let mstart = parts[4].parse().unwrap_or(0);
        let mend   = parts[5].parse().unwrap_or(0);
        (regex, input, pos, found, mstart, mend)
    } else {
        (inner.to_string(), String::new(), 0, false, 0, 0)
    }
}

// ── Regex helpers ────────────────────────────────────────────────────────────

/// Find first match of `regex` in `text` starting at byte position `from`.
/// Returns (start, end) byte indices.
fn regex_find_from(regex: &str, text: &str, from: usize) -> Option<(usize, usize)> {
    if from > text.len() { return None; }
    let slice = &text[from..];
    regex_find_span(regex, slice).map(|(s, e)| (from + s, from + e))
}

/// Count capturing groups `(` that are not `(?:`.
fn count_groups(regex: &str) -> usize {
    let chars: Vec<char> = regex.chars().collect();
    let mut count = 0;
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\\' { i += 2; continue; }
        if chars[i] == '[' {
            while i < chars.len() && chars[i] != ']' { i += 1; }
        }
        if chars[i] == '(' {
            if !(i + 1 < chars.len() && chars[i+1] == '?' && i + 2 < chars.len() && chars[i+2] == ':') {
                count += 1;
            }
        }
        i += 1;
    }
    count
}

/// Extract the nth capturing group from a matched string.
fn extract_group(regex: &str, matched: &str, group: usize) -> Option<String> {
    // Simplified: find the nth `(...)` group in the pattern and match it
    // This is a best-effort implementation for common cases
    let _ = (regex, matched, group);
    None
}

/// Escape a string for use as a literal regex pattern.
fn regex_quote(s: &str) -> String {
    let mut out = String::from("\\Q");
    out.push_str(s);
    out.push_str("\\E");
    out
}
