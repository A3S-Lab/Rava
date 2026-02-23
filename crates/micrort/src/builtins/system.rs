//! Java System class builtins.

use rava_common::error::Result;
use crate::rir_interp::RVal;
use super::format::{fnv, format_java_string};

pub fn dispatch(func_id: u32, args: &[RVal]) -> Option<Result<RVal>> {
    match func_id {
        id if id == fnv("System.out.println") => {
            println!("{}", args.first().map(|v| v.to_display()).unwrap_or_default());
            Some(Ok(RVal::Void))
        }
        id if id == fnv("System.out.print") => {
            print!("{}", args.first().map(|v| v.to_display()).unwrap_or_default());
            Some(Ok(RVal::Void))
        }
        id if id == fnv("System.out.printf") || id == fnv("System.out.format") => {
            print!("{}", format_java_string(args));
            Some(Ok(RVal::Void))
        }
        id if id == fnv("System.err.println") => {
            eprintln!("{}", args.first().map(|v| v.to_display()).unwrap_or_default());
            Some(Ok(RVal::Void))
        }
        id if id == fnv("System.err.print") => {
            eprint!("{}", args.first().map(|v| v.to_display()).unwrap_or_default());
            Some(Ok(RVal::Void))
        }
        id if id == fnv("System.exit") => {
            std::process::exit(args.first().map(|v| v.as_int()).unwrap_or(0) as i32);
        }
        id if id == fnv("System.currentTimeMillis") => {
            let ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as i64).unwrap_or(0);
            Some(Ok(RVal::Int(ms)))
        }
        id if id == fnv("System.nanoTime") => {
            let ns = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as i64).unwrap_or(0);
            Some(Ok(RVal::Int(ns)))
        }
        id if id == fnv("System.lineSeparator") => Some(Ok(RVal::Str("\n".into()))),
        id if id == fnv("System.getenv") => {
            let key = args.first().map(|v| v.to_display()).unwrap_or_default();
            let val = std::env::var(&key).unwrap_or_default();
            Some(Ok(RVal::Str(val)))
        }
        id if id == fnv("System.getProperty") => {
            // Stub common properties
            let key = args.first().map(|v| v.to_display()).unwrap_or_default();
            let val = match key.as_str() {
                "os.name"    => std::env::consts::OS.to_string(),
                "user.home"  => std::env::var("HOME").unwrap_or_default(),
                "user.dir"   => std::env::current_dir().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default(),
                "java.version" => "17.0.0".into(),
                "line.separator" => "\n".into(),
                "file.separator" => "/".into(),
                "path.separator" => ":".into(),
                _ => String::new(),
            };
            Some(Ok(RVal::Str(val)))
        }
        id if id == fnv("System.arraycopy") => {
            // arraycopy(src, srcPos, dest, destPos, length)
            if let (Some(RVal::Array(src)), Some(RVal::Array(dst))) = (args.first(), args.get(2)) {
                let src_pos = args.get(1).map(|v| v.as_int()).unwrap_or(0).max(0) as usize;
                let dst_pos = args.get(3).map(|v| v.as_int()).unwrap_or(0).max(0) as usize;
                let length  = args.get(4).map(|v| v.as_int()).unwrap_or(0).max(0) as usize;
                let src_v = src.borrow();
                let mut dst_v = dst.borrow_mut();
                for i in 0..length {
                    let val = src_v.get(src_pos + i).cloned().unwrap_or(RVal::Null);
                    while dst_v.len() <= dst_pos + i { dst_v.push(RVal::Null); }
                    dst_v[dst_pos + i] = val;
                }
            }
            Some(Ok(RVal::Void))
        }
        id if id == fnv("System.identityHashCode") => {
            Some(Ok(RVal::Int(0))) // stub
        }
        _ => None,
    }
}
