//! Java numeric wrapper classes: Integer, Long, Double, Float, Byte, Short, Character, Boolean.

use rava_common::error::Result;
use crate::rir_interp::RVal;
use super::format::fnv;

pub fn dispatch(func_id: u32, args: &[RVal]) -> Option<Result<RVal>> {
    match func_id {
        // ── Integer ───────────────────────────────────────────────────────────
        id if id == fnv("Integer.parseInt") || id == fnv("Integer.valueOf") => {
            let s = args.first().map(|v| v.to_display()).unwrap_or_default();
            // Support radix: parseInt(s, radix)
            let radix = args.get(1).map(|v| v.as_int()).unwrap_or(10) as u32;
            let n = if radix == 10 { s.trim().parse::<i64>().unwrap_or(0) }
                    else { i64::from_str_radix(s.trim(), radix).unwrap_or(0) };
            Some(Ok(RVal::Int(n)))
        }
        id if id == fnv("Integer.toString") => Some(Ok(RVal::Str(args.first().map(|v| v.to_display()).unwrap_or_default()))),
        id if id == fnv("Integer.toHexString")    => Some(Ok(RVal::Str(format!("{:x}", args.first().map(|v| v.as_int()).unwrap_or(0) as u64)))),
        id if id == fnv("Integer.toBinaryString") => Some(Ok(RVal::Str(format!("{:b}", args.first().map(|v| v.as_int()).unwrap_or(0) as u64)))),
        id if id == fnv("Integer.toOctalString")  => Some(Ok(RVal::Str(format!("{:o}", args.first().map(|v| v.as_int()).unwrap_or(0) as u64)))),
        id if id == fnv("Integer.compare") => {
            let a = args.first().map(|v| v.as_int()).unwrap_or(0);
            let b = args.get(1).map(|v| v.as_int()).unwrap_or(0);
            Some(Ok(RVal::Int(a.cmp(&b) as i64)))
        }
        id if id == fnv("Integer.max") => Some(Ok(RVal::Int(args.first().map(|v| v.as_int()).unwrap_or(0).max(args.get(1).map(|v| v.as_int()).unwrap_or(0))))),
        id if id == fnv("Integer.min") => Some(Ok(RVal::Int(args.first().map(|v| v.as_int()).unwrap_or(0).min(args.get(1).map(|v| v.as_int()).unwrap_or(0))))),
        id if id == fnv("Integer.sum") => Some(Ok(RVal::Int(args.first().map(|v| v.as_int()).unwrap_or(0) + args.get(1).map(|v| v.as_int()).unwrap_or(0)))),
        id if id == fnv("Integer.bitCount")             => Some(Ok(RVal::Int((args.first().map(|v| v.as_int()).unwrap_or(0) as u64).count_ones() as i64))),
        id if id == fnv("Integer.numberOfLeadingZeros") => Some(Ok(RVal::Int((args.first().map(|v| v.as_int()).unwrap_or(0) as u32).leading_zeros() as i64))),
        id if id == fnv("Integer.numberOfTrailingZeros")=> Some(Ok(RVal::Int((args.first().map(|v| v.as_int()).unwrap_or(0) as u32).trailing_zeros() as i64))),
        id if id == fnv("Integer.reverse")              => Some(Ok(RVal::Int((args.first().map(|v| v.as_int()).unwrap_or(0) as u32).reverse_bits() as i64))),
        id if id == fnv("Integer.highestOneBit") => {
            let n = args.first().map(|v| v.as_int()).unwrap_or(0) as u32;
            Some(Ok(RVal::Int(if n == 0 { 0 } else { 1i64 << (31 - n.leading_zeros()) })))
        }
        id if id == fnv("Integer.MAX_VALUE") => Some(Ok(RVal::Int(i32::MAX as i64))),
        id if id == fnv("Integer.MIN_VALUE") => Some(Ok(RVal::Int(i32::MIN as i64))),

        // ── Long ──────────────────────────────────────────────────────────────
        id if id == fnv("Long.parseLong") || id == fnv("Long.valueOf") => {
            let n = args.first().map(|v| v.to_display()).unwrap_or_default().trim().parse::<i64>().unwrap_or(0);
            Some(Ok(RVal::Int(n)))
        }
        id if id == fnv("Long.toString")    => Some(Ok(RVal::Str(args.first().map(|v| v.to_display()).unwrap_or_default()))),
        id if id == fnv("Long.toHexString") => Some(Ok(RVal::Str(format!("{:x}", args.first().map(|v| v.as_int()).unwrap_or(0) as u64)))),
        id if id == fnv("Long.compare")     => {
            let a = args.first().map(|v| v.as_int()).unwrap_or(0);
            let b = args.get(1).map(|v| v.as_int()).unwrap_or(0);
            Some(Ok(RVal::Int(a.cmp(&b) as i64)))
        }
        id if id == fnv("Long.MAX_VALUE") => Some(Ok(RVal::Int(i64::MAX))),
        id if id == fnv("Long.MIN_VALUE") => Some(Ok(RVal::Int(i64::MIN))),

        // ── Double ────────────────────────────────────────────────────────────
        id if id == fnv("Double.parseDouble") || id == fnv("Double.valueOf") => {
            let f = args.first().map(|v| v.to_display()).unwrap_or_default().trim().parse::<f64>().unwrap_or(0.0);
            Some(Ok(RVal::Float(f)))
        }
        id if id == fnv("Double.toString")   => Some(Ok(RVal::Str(args.first().map(|v| v.to_display()).unwrap_or_default()))),
        id if id == fnv("Double.compare")    => {
            let a = args.first().map(|v| v.as_float()).unwrap_or(0.0);
            let b = args.get(1).map(|v| v.as_float()).unwrap_or(0.0);
            Some(Ok(RVal::Int(a.partial_cmp(&b).map(|o| o as i64).unwrap_or(0))))
        }
        id if id == fnv("Double.isNaN")      => Some(Ok(RVal::Bool(args.first().map(|v| v.as_float()).unwrap_or(0.0).is_nan()))),
        id if id == fnv("Double.isInfinite") => Some(Ok(RVal::Bool(args.first().map(|v| v.as_float()).unwrap_or(0.0).is_infinite()))),
        id if id == fnv("Double.isFinite")   => Some(Ok(RVal::Bool(args.first().map(|v| v.as_float()).unwrap_or(0.0).is_finite()))),
        id if id == fnv("Double.MAX_VALUE")  => Some(Ok(RVal::Float(f64::MAX))),
        id if id == fnv("Double.MIN_VALUE")  => Some(Ok(RVal::Float(f64::MIN_POSITIVE))),

        // ── Float ─────────────────────────────────────────────────────────────
        id if id == fnv("Float.parseFloat") || id == fnv("Float.valueOf") => {
            let f = args.first().map(|v| v.to_display()).unwrap_or_default().trim().parse::<f64>().unwrap_or(0.0);
            Some(Ok(RVal::Float(f)))
        }
        id if id == fnv("Float.isNaN")      => Some(Ok(RVal::Bool(args.first().map(|v| v.as_float()).unwrap_or(0.0).is_nan()))),
        id if id == fnv("Float.isInfinite") => Some(Ok(RVal::Bool(args.first().map(|v| v.as_float()).unwrap_or(0.0).is_infinite()))),

        // ── Byte / Short ──────────────────────────────────────────────────────
        id if id == fnv("Byte.parseByte")   => Some(Ok(RVal::Int(args.first().map(|v| v.to_display()).unwrap_or_default().trim().parse::<i64>().unwrap_or(0)))),
        id if id == fnv("Short.parseShort") => Some(Ok(RVal::Int(args.first().map(|v| v.to_display()).unwrap_or_default().trim().parse::<i64>().unwrap_or(0)))),

        // ── Boolean ───────────────────────────────────────────────────────────
        id if id == fnv("Boolean.parseBoolean") || id == fnv("Boolean.valueOf") => {
            let b = args.first().map(|v| v.to_display()).unwrap_or_default().to_lowercase() == "true";
            Some(Ok(RVal::Bool(b)))
        }
        id if id == fnv("Boolean.toString") => Some(Ok(RVal::Str(args.first().map(|v| v.is_truthy().to_string()).unwrap_or_else(|| "false".into())))),

        // ── Character ─────────────────────────────────────────────────────────
        id if id == fnv("Character.isDigit")       => Some(Ok(RVal::Bool((args.first().map(|v| v.as_int()).unwrap_or(0) as u8 as char).is_ascii_digit()))),
        id if id == fnv("Character.isLetter")      => Some(Ok(RVal::Bool((args.first().map(|v| v.as_int()).unwrap_or(0) as u8 as char).is_alphabetic()))),
        id if id == fnv("Character.isLetterOrDigit")=> Some(Ok(RVal::Bool((args.first().map(|v| v.as_int()).unwrap_or(0) as u8 as char).is_alphanumeric()))),
        id if id == fnv("Character.isWhitespace") || id == fnv("Character.isSpaceChar") => Some(Ok(RVal::Bool((args.first().map(|v| v.as_int()).unwrap_or(0) as u8 as char).is_whitespace()))),
        id if id == fnv("Character.isUpperCase")   => Some(Ok(RVal::Bool((args.first().map(|v| v.as_int()).unwrap_or(0) as u8 as char).is_uppercase()))),
        id if id == fnv("Character.isLowerCase")   => Some(Ok(RVal::Bool((args.first().map(|v| v.as_int()).unwrap_or(0) as u8 as char).is_lowercase()))),
        id if id == fnv("Character.isAlphabetic")  => Some(Ok(RVal::Bool((args.first().map(|v| v.as_int()).unwrap_or(0) as u8 as char).is_alphabetic()))),
        id if id == fnv("Character.toUpperCase") => {
            let c = args.first().map(|v| v.as_int()).unwrap_or(0) as u8 as char;
            Some(Ok(RVal::Int(c.to_uppercase().next().unwrap_or(c) as i64)))
        }
        id if id == fnv("Character.toLowerCase") => {
            let c = args.first().map(|v| v.as_int()).unwrap_or(0) as u8 as char;
            Some(Ok(RVal::Int(c.to_lowercase().next().unwrap_or(c) as i64)))
        }
        id if id == fnv("Character.toString") => {
            let c = args.first().map(|v| v.as_int()).unwrap_or(0) as u8 as char;
            Some(Ok(RVal::Str(c.to_string())))
        }
        id if id == fnv("Character.getNumericValue") => {
            let c = args.first().map(|v| v.as_int()).unwrap_or(0) as u8 as char;
            Some(Ok(RVal::Int(c.to_digit(10).map(|d| d as i64).unwrap_or(-1))))
        }
        id if id == fnv("Character.digit") => {
            let c = args.first().map(|v| v.as_int()).unwrap_or(0) as u8 as char;
            let radix = args.get(1).map(|v| v.as_int()).unwrap_or(10) as u32;
            Some(Ok(RVal::Int(c.to_digit(radix).map(|d| d as i64).unwrap_or(-1))))
        }
        id if id == fnv("Character.forDigit") => {
            let digit = args.first().map(|v| v.as_int()).unwrap_or(0) as u32;
            let radix = args.get(1).map(|v| v.as_int()).unwrap_or(10) as u32;
            let c = char::from_digit(digit, radix).unwrap_or('\0');
            Some(Ok(RVal::Int(c as i64)))
        }

        // ── String static ─────────────────────────────────────────────────────
        id if id == fnv("String.valueOf") => Some(Ok(RVal::Str(args.first().map(|v| v.to_display()).unwrap_or_default()))),

        _ => None,
    }
}
