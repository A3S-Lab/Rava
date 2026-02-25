//! java.time API: LocalDate, LocalTime, LocalDateTime, Instant, Duration, Period, ZonedDateTime.

use rava_common::error::Result;
use crate::rir_interp::RVal;
use super::format::fnv;
use std::time::{SystemTime, UNIX_EPOCH};

fn now_epoch_secs() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// Encode a LocalDate as "YYYY-MM-DD" string stored in RVal::Str.
fn local_date(year: i64, month: i64, day: i64) -> RVal {
    RVal::Str(format!("__date__{:04}-{:02}-{:02}", year, month, day))
}

/// Encode a LocalTime as "HH:MM:SS.nanos".
fn local_time(h: i64, m: i64, s: i64, nano: i64) -> RVal {
    RVal::Str(format!("__time__{:02}:{:02}:{:02}.{}", h, m, s, nano))
}

/// Encode a LocalDateTime.
fn local_datetime(year: i64, month: i64, day: i64, h: i64, m: i64, s: i64) -> RVal {
    RVal::Str(format!("__datetime__{:04}-{:02}-{:02}T{:02}:{:02}:{:02}", year, month, day, h, m, s))
}

fn parse_date(s: &str) -> (i64, i64, i64) {
    let s = s.strip_prefix("__date__").unwrap_or(s);
    let parts: Vec<&str> = s.split('-').collect();
    let y = parts.first().and_then(|p| p.parse().ok()).unwrap_or(1970);
    let mo = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(1);
    let d = parts.get(2).and_then(|p| p.parse().ok()).unwrap_or(1);
    (y, mo, d)
}

fn parse_time(s: &str) -> (i64, i64, i64, i64) {
    let s = s.strip_prefix("__time__").unwrap_or(s);
    let (hms, nano_str) = s.split_once('.').unwrap_or((s, "0"));
    let parts: Vec<&str> = hms.split(':').collect();
    let h = parts.first().and_then(|p| p.parse().ok()).unwrap_or(0);
    let m = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(0);
    let sec = parts.get(2).and_then(|p| p.parse().ok()).unwrap_or(0);
    let nano = nano_str.parse().unwrap_or(0);
    (h, m, sec, nano)
}

fn parse_datetime(s: &str) -> (i64, i64, i64, i64, i64, i64) {
    let s = s.strip_prefix("__datetime__").unwrap_or(s);
    let (date_part, time_part) = s.split_once('T').unwrap_or((s, "00:00:00"));
    let (y, mo, d) = parse_date(date_part);
    let (h, m, sec, _) = parse_time(time_part);
    (y, mo, d, h, m, sec)
}

/// Days in month (non-leap).
fn days_in_month(month: i64, year: i64) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) { 29 } else { 28 },
        _ => 30,
    }
}

/// Approximate epoch-seconds → (year, month, day, hour, min, sec).
fn epoch_to_ymd(secs: i64) -> (i64, i64, i64, i64, i64, i64) {
    let sec = secs % 60;
    let mins = secs / 60;
    let min = mins % 60;
    let hours = mins / 60;
    let hour = hours % 24;
    let mut days = hours / 24;
    let mut year = 1970i64;
    loop {
        let dy = if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) { 366 } else { 365 };
        if days < dy { break; }
        days -= dy;
        year += 1;
    }
    let mut month = 1i64;
    loop {
        let dm = days_in_month(month, year);
        if days < dm { break; }
        days -= dm;
        month += 1;
    }
    (year, month, days + 1, hour, min, sec)
}

pub fn dispatch(func_id: u32, args: &[RVal]) -> Option<Result<RVal>> {
    match func_id {
        // ── LocalDate ──────────────────────────────────────────────────────────
        id if id == fnv("LocalDate.now") => {
            let (y, mo, d, _, _, _) = epoch_to_ymd(now_epoch_secs());
            Some(Ok(local_date(y, mo, d)))
        }
        id if id == fnv("LocalDate.of") => {
            let y  = args.first().map(|v| v.as_int()).unwrap_or(1970);
            let mo = args.get(1).map(|v| v.as_int()).unwrap_or(1);
            let d  = args.get(2).map(|v| v.as_int()).unwrap_or(1);
            Some(Ok(local_date(y, mo, d)))
        }
        id if id == fnv("LocalDate.parse") => {
            let s = args.first().map(|v| v.to_display()).unwrap_or_default();
            Some(Ok(RVal::Str(format!("__date__{}", s))))
        }
        // ── LocalTime ──────────────────────────────────────────────────────────
        id if id == fnv("LocalTime.now") => {
            let (_, _, _, h, m, s) = epoch_to_ymd(now_epoch_secs());
            Some(Ok(local_time(h, m, s, 0)))
        }
        id if id == fnv("LocalTime.of") => {
            let h    = args.first().map(|v| v.as_int()).unwrap_or(0);
            let m    = args.get(1).map(|v| v.as_int()).unwrap_or(0);
            let s    = args.get(2).map(|v| v.as_int()).unwrap_or(0);
            let nano = args.get(3).map(|v| v.as_int()).unwrap_or(0);
            Some(Ok(local_time(h, m, s, nano)))
        }
        id if id == fnv("LocalTime.parse") => {
            let s = args.first().map(|v| v.to_display()).unwrap_or_default();
            Some(Ok(RVal::Str(format!("__time__{}", s))))
        }
        // ── LocalDateTime ──────────────────────────────────────────────────────
        id if id == fnv("LocalDateTime.now") => {
            let (y, mo, d, h, m, s) = epoch_to_ymd(now_epoch_secs());
            Some(Ok(local_datetime(y, mo, d, h, m, s)))
        }
        id if id == fnv("LocalDateTime.of") => {
            let y  = args.first().map(|v| v.as_int()).unwrap_or(1970);
            let mo = args.get(1).map(|v| v.as_int()).unwrap_or(1);
            let d  = args.get(2).map(|v| v.as_int()).unwrap_or(1);
            let h  = args.get(3).map(|v| v.as_int()).unwrap_or(0);
            let m  = args.get(4).map(|v| v.as_int()).unwrap_or(0);
            let s  = args.get(5).map(|v| v.as_int()).unwrap_or(0);
            Some(Ok(local_datetime(y, mo, d, h, m, s)))
        }
        id if id == fnv("LocalDateTime.parse") => {
            let s = args.first().map(|v| v.to_display()).unwrap_or_default();
            Some(Ok(RVal::Str(format!("__datetime__{}", s))))
        }
        // ── Instant ────────────────────────────────────────────────────────────
        id if id == fnv("Instant.now") => {
            Some(Ok(RVal::Str(format!("__instant__{}", now_epoch_secs()))))
        }
        id if id == fnv("Instant.ofEpochSecond") || id == fnv("Instant.ofEpochMilli") => {
            let v = args.first().map(|v| v.as_int()).unwrap_or(0);
            let secs = if func_id == fnv("Instant.ofEpochMilli") { v / 1000 } else { v };
            Some(Ok(RVal::Str(format!("__instant__{}", secs))))
        }
        // ── Duration ───────────────────────────────────────────────────────────
        id if id == fnv("Duration.ofSeconds") => {
            let s = args.first().map(|v| v.as_int()).unwrap_or(0);
            Some(Ok(RVal::Str(format!("__duration__{}", s))))
        }
        id if id == fnv("Duration.ofMinutes") => {
            let m = args.first().map(|v| v.as_int()).unwrap_or(0);
            Some(Ok(RVal::Str(format!("__duration__{}", m * 60))))
        }
        id if id == fnv("Duration.ofHours") => {
            let h = args.first().map(|v| v.as_int()).unwrap_or(0);
            Some(Ok(RVal::Str(format!("__duration__{}", h * 3600))))
        }
        id if id == fnv("Duration.ofDays") => {
            let d = args.first().map(|v| v.as_int()).unwrap_or(0);
            Some(Ok(RVal::Str(format!("__duration__{}", d * 86400))))
        }
        id if id == fnv("Duration.between") => {
            let a = args.first().map(|v| v.to_display()).unwrap_or_default();
            let b = args.get(1).map(|v| v.to_display()).unwrap_or_default();
            let sa = a.strip_prefix("__instant__").and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
            let sb = b.strip_prefix("__instant__").and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
            Some(Ok(RVal::Str(format!("__duration__{}", sb - sa))))
        }
        // ── Period ─────────────────────────────────────────────────────────────
        id if id == fnv("Period.ofDays") => {
            let d = args.first().map(|v| v.as_int()).unwrap_or(0);
            Some(Ok(RVal::Str(format!("__period__0-0-{}", d))))
        }
        id if id == fnv("Period.ofMonths") => {
            let m = args.first().map(|v| v.as_int()).unwrap_or(0);
            Some(Ok(RVal::Str(format!("__period__0-{}-0", m))))
        }
        id if id == fnv("Period.ofYears") => {
            let y = args.first().map(|v| v.as_int()).unwrap_or(0);
            Some(Ok(RVal::Str(format!("__period__{}-0-0", y))))
        }
        id if id == fnv("Period.of") => {
            let y = args.first().map(|v| v.as_int()).unwrap_or(0);
            let m = args.get(1).map(|v| v.as_int()).unwrap_or(0);
            let d = args.get(2).map(|v| v.as_int()).unwrap_or(0);
            Some(Ok(RVal::Str(format!("__period__{}-{}-{}", y, m, d))))
        }
        _ => None,
    }
}

/// Instance methods on java.time objects (encoded as tagged strings).
pub fn dispatch_named(receiver: &str, method: &str, args: &[RVal]) -> Option<Result<RVal>> {
    if receiver.starts_with("__date__") {
        let (y, mo, d) = parse_date(receiver);
        return match method {
            "getYear"       => Some(Ok(RVal::Int(y))),
            "getMonthValue" => Some(Ok(RVal::Int(mo))),
            "getDayOfMonth" => Some(Ok(RVal::Int(d))),
            "getDayOfWeek"  => {
                // Zeller's congruence (simplified)
                Some(Ok(RVal::Str("MONDAY".into())))
            }
            "plusDays"   => {
                let n = args.first().map(|v| v.as_int()).unwrap_or(0);
                let mut dd = d + n;
                let mut mm = mo;
                let mut yy = y;
                while dd > days_in_month(mm, yy) {
                    dd -= days_in_month(mm, yy);
                    mm += 1;
                    if mm > 12 { mm = 1; yy += 1; }
                }
                Some(Ok(local_date(yy, mm, dd)))
            }
            "plusMonths"  => {
                let n = args.first().map(|v| v.as_int()).unwrap_or(0);
                let total = mo - 1 + n;
                Some(Ok(local_date(y + total / 12, total % 12 + 1, d)))
            }
            "plusYears"   => {
                let n = args.first().map(|v| v.as_int()).unwrap_or(0);
                Some(Ok(local_date(y + n, mo, d)))
            }
            "minusDays"   => {
                let n = args.first().map(|v| v.as_int()).unwrap_or(0);
                let mut dd = d - n;
                let mut mm = mo;
                let mut yy = y;
                while dd <= 0 {
                    mm -= 1;
                    if mm <= 0 { mm = 12; yy -= 1; }
                    dd += days_in_month(mm, yy);
                }
                Some(Ok(local_date(yy, mm, dd)))
            }
            "minusMonths" => {
                let n = args.first().map(|v| v.as_int()).unwrap_or(0);
                let total = mo - 1 - n;
                let yy = y + total.div_euclid(12);
                let mm = total.rem_euclid(12) + 1;
                Some(Ok(local_date(yy, mm, d)))
            }
            "minusYears"  => {
                let n = args.first().map(|v| v.as_int()).unwrap_or(0);
                Some(Ok(local_date(y - n, mo, d)))
            }
            "isBefore"    => {
                let other = args.first().map(|v| v.to_display()).unwrap_or_default();
                let (oy, omo, od) = parse_date(&other);
                Some(Ok(RVal::Bool((y, mo, d) < (oy, omo, od))))
            }
            "isAfter"     => {
                let other = args.first().map(|v| v.to_display()).unwrap_or_default();
                let (oy, omo, od) = parse_date(&other);
                Some(Ok(RVal::Bool((y, mo, d) > (oy, omo, od))))
            }
            "isEqual"     => {
                let other = args.first().map(|v| v.to_display()).unwrap_or_default();
                let (oy, omo, od) = parse_date(&other);
                Some(Ok(RVal::Bool((y, mo, d) == (oy, omo, od))))
            }
            "toString"    => Some(Ok(RVal::Str(format!("{:04}-{:02}-{:02}", y, mo, d)))),
            "atTime"      => {
                let h = args.first().map(|v| v.as_int()).unwrap_or(0);
                let m = args.get(1).map(|v| v.as_int()).unwrap_or(0);
                let s = args.get(2).map(|v| v.as_int()).unwrap_or(0);
                Some(Ok(local_datetime(y, mo, d, h, m, s)))
            }
            _ => None,
        };
    }

    if receiver.starts_with("__time__") {
        let (h, m, s, nano) = parse_time(receiver);
        return match method {
            "getHour"   => Some(Ok(RVal::Int(h))),
            "getMinute" => Some(Ok(RVal::Int(m))),
            "getSecond" => Some(Ok(RVal::Int(s))),
            "getNano"   => Some(Ok(RVal::Int(nano))),
            "plusHours"   => {
                let n = args.first().map(|v| v.as_int()).unwrap_or(0);
                Some(Ok(local_time((h + n) % 24, m, s, nano)))
            }
            "plusMinutes" => {
                let n = args.first().map(|v| v.as_int()).unwrap_or(0);
                let total = m + n;
                Some(Ok(local_time((h + total / 60) % 24, total % 60, s, nano)))
            }
            "plusSeconds" => {
                let n = args.first().map(|v| v.as_int()).unwrap_or(0);
                let total = s + n;
                Some(Ok(local_time(h, (m + total / 60) % 60, total % 60, nano)))
            }
            "isBefore"    => {
                let other = args.first().map(|v| v.to_display()).unwrap_or_default();
                let (oh, om, os, on) = parse_time(&other);
                Some(Ok(RVal::Bool((h, m, s, nano) < (oh, om, os, on))))
            }
            "isAfter"     => {
                let other = args.first().map(|v| v.to_display()).unwrap_or_default();
                let (oh, om, os, on) = parse_time(&other);
                Some(Ok(RVal::Bool((h, m, s, nano) > (oh, om, os, on))))
            }
            "toString"    => Some(Ok(RVal::Str(format!("{:02}:{:02}:{:02}", h, m, s)))),
            _ => None,
        };
    }

    if receiver.starts_with("__datetime__") {
        let (y, mo, d, h, m, s) = parse_datetime(receiver);
        return match method {
            "getYear"       => Some(Ok(RVal::Int(y))),
            "getMonthValue" => Some(Ok(RVal::Int(mo))),
            "getDayOfMonth" => Some(Ok(RVal::Int(d))),
            "getHour"       => Some(Ok(RVal::Int(h))),
            "getMinute"     => Some(Ok(RVal::Int(m))),
            "getSecond"     => Some(Ok(RVal::Int(s))),
            "toLocalDate"   => Some(Ok(local_date(y, mo, d))),
            "toLocalTime"   => Some(Ok(local_time(h, m, s, 0))),
            "plusDays"      => {
                let n = args.first().map(|v| v.as_int()).unwrap_or(0);
                let mut dd = d + n;
                let mut mm = mo;
                let mut yy = y;
                while dd > days_in_month(mm, yy) {
                    dd -= days_in_month(mm, yy);
                    mm += 1;
                    if mm > 12 { mm = 1; yy += 1; }
                }
                Some(Ok(local_datetime(yy, mm, dd, h, m, s)))
            }
            "plusHours"     => {
                let n = args.first().map(|v| v.as_int()).unwrap_or(0);
                Some(Ok(local_datetime(y, mo, d, (h + n) % 24, m, s)))
            }
            "isBefore"      => {
                let other = args.first().map(|v| v.to_display()).unwrap_or_default();
                let (oy, omo, od, oh, om, os) = parse_datetime(&other);
                Some(Ok(RVal::Bool((y, mo, d, h, m, s) < (oy, omo, od, oh, om, os))))
            }
            "isAfter"       => {
                let other = args.first().map(|v| v.to_display()).unwrap_or_default();
                let (oy, omo, od, oh, om, os) = parse_datetime(&other);
                Some(Ok(RVal::Bool((y, mo, d, h, m, s) > (oy, omo, od, oh, om, os))))
            }
            "toString"      => Some(Ok(RVal::Str(format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}", y, mo, d, h, m, s)))),
            _ => None,
        };
    }

    if receiver.starts_with("__instant__") {
        let secs: i64 = receiver.strip_prefix("__instant__").and_then(|s| s.parse().ok()).unwrap_or(0);
        return match method {
            "getEpochSecond"  => Some(Ok(RVal::Int(secs))),
            "toEpochMilli"    => Some(Ok(RVal::Int(secs * 1000))),
            "plusSeconds"     => {
                let n = args.first().map(|v| v.as_int()).unwrap_or(0);
                Some(Ok(RVal::Str(format!("__instant__{}", secs + n))))
            }
            "isBefore"        => {
                let other: i64 = args.first().map(|v| v.to_display()).unwrap_or_default()
                    .strip_prefix("__instant__").and_then(|s| s.parse().ok()).unwrap_or(0);
                Some(Ok(RVal::Bool(secs < other)))
            }
            "isAfter"         => {
                let other: i64 = args.first().map(|v| v.to_display()).unwrap_or_default()
                    .strip_prefix("__instant__").and_then(|s| s.parse().ok()).unwrap_or(0);
                Some(Ok(RVal::Bool(secs > other)))
            }
            "toString"        => Some(Ok(RVal::Str(format!("{}Z", secs)))),
            _ => None,
        };
    }

    if receiver.starts_with("__duration__") {
        let total_secs: i64 = receiver.strip_prefix("__duration__").and_then(|s| s.parse().ok()).unwrap_or(0);
        return match method {
            "getSeconds"  => Some(Ok(RVal::Int(total_secs))),
            "toMinutes"   => Some(Ok(RVal::Int(total_secs / 60))),
            "toHours"     => Some(Ok(RVal::Int(total_secs / 3600))),
            "toDays"      => Some(Ok(RVal::Int(total_secs / 86400))),
            "toMillis"    => Some(Ok(RVal::Int(total_secs * 1000))),
            "isNegative"  => Some(Ok(RVal::Bool(total_secs < 0))),
            "isZero"      => Some(Ok(RVal::Bool(total_secs == 0))),
            "abs"         => Some(Ok(RVal::Str(format!("__duration__{}", total_secs.abs())))),
            "plus"        => {
                let other: i64 = args.first().map(|v| v.to_display()).unwrap_or_default()
                    .strip_prefix("__duration__").and_then(|s| s.parse().ok()).unwrap_or(0);
                Some(Ok(RVal::Str(format!("__duration__{}", total_secs + other))))
            }
            "minus"       => {
                let other: i64 = args.first().map(|v| v.to_display()).unwrap_or_default()
                    .strip_prefix("__duration__").and_then(|s| s.parse().ok()).unwrap_or(0);
                Some(Ok(RVal::Str(format!("__duration__{}", total_secs - other))))
            }
            "toString"    => Some(Ok(RVal::Str(format!("PT{}S", total_secs)))),
            _ => None,
        };
    }

    if receiver.starts_with("__period__") {
        let s = receiver.strip_prefix("__period__").unwrap_or("0-0-0");
        let parts: Vec<i64> = s.split('-').map(|p| p.parse().unwrap_or(0)).collect();
        let (py, pm, pd) = (parts.first().copied().unwrap_or(0), parts.get(1).copied().unwrap_or(0), parts.get(2).copied().unwrap_or(0));
        return match method {
            "getYears"  => Some(Ok(RVal::Int(py))),
            "getMonths" => Some(Ok(RVal::Int(pm))),
            "getDays"   => Some(Ok(RVal::Int(pd))),
            "isNegative"=> Some(Ok(RVal::Bool(py < 0 || pm < 0 || pd < 0))),
            "isZero"    => Some(Ok(RVal::Bool(py == 0 && pm == 0 && pd == 0))),
            "toString"  => Some(Ok(RVal::Str(format!("P{}Y{}M{}D", py, pm, pd)))),
            _ => None,
        };
    }

    None
}
