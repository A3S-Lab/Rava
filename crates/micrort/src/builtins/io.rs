//! Java file I/O builtins: java.io, java.nio.file.

use rava_common::error::{RavaError, Result};
use crate::rir_interp::RVal;
use std::cell::RefCell;
use std::rc::Rc;
use super::format::fnv;

pub fn dispatch(func_id: u32, args: &[RVal]) -> Option<Result<RVal>> {
    match func_id {
        id if id == fnv("Files.readString") || id == fnv("Files.readAllBytes") => {
            let path = args.first().map(|v| v.to_display()).unwrap_or_default();
            Some(std::fs::read_to_string(&path).map(RVal::Str).map_err(|e| RavaError::JavaException {
                exception_type: "IOException".into(), message: e.to_string(),
            }))
        }
        id if id == fnv("Files.readAllLines") => {
            let path = args.first().map(|v| v.to_display()).unwrap_or_default();
            match std::fs::read_to_string(&path) {
                Ok(s) => {
                    let lines: Vec<RVal> = s.lines().map(|l| RVal::Str(l.to_string())).collect();
                    Some(Ok(RVal::Array(Rc::new(RefCell::new(lines)))))
                }
                Err(e) => Some(Err(RavaError::JavaException { exception_type: "IOException".into(), message: e.to_string() })),
            }
        }
        id if id == fnv("Files.writeString") || id == fnv("Files.write") => {
            let path    = args.first().map(|v| v.to_display()).unwrap_or_default();
            let content = args.get(1).map(|v| v.to_display()).unwrap_or_default();
            Some(std::fs::write(&path, content).map(|_| RVal::Void).map_err(|e| RavaError::JavaException {
                exception_type: "IOException".into(), message: e.to_string(),
            }))
        }
        id if id == fnv("Files.exists")      => Some(Ok(RVal::Bool(std::path::Path::new(&args.first().map(|v| v.to_display()).unwrap_or_default()).exists()))),
        id if id == fnv("Files.notExists")   => Some(Ok(RVal::Bool(!std::path::Path::new(&args.first().map(|v| v.to_display()).unwrap_or_default()).exists()))),
        id if id == fnv("Files.isDirectory") => Some(Ok(RVal::Bool(std::path::Path::new(&args.first().map(|v| v.to_display()).unwrap_or_default()).is_dir()))),
        id if id == fnv("Files.isRegularFile") => Some(Ok(RVal::Bool(std::path::Path::new(&args.first().map(|v| v.to_display()).unwrap_or_default()).is_file()))),
        id if id == fnv("Files.delete") || id == fnv("Files.deleteIfExists") => {
            let _ = std::fs::remove_file(args.first().map(|v| v.to_display()).unwrap_or_default());
            Some(Ok(RVal::Void))
        }
        id if id == fnv("Files.createDirectory") || id == fnv("Files.createDirectories") => {
            let _ = std::fs::create_dir_all(args.first().map(|v| v.to_display()).unwrap_or_default());
            Some(Ok(RVal::Void))
        }
        id if id == fnv("Files.copy") => {
            let src = args.first().map(|v| v.to_display()).unwrap_or_default();
            let dst = args.get(1).map(|v| v.to_display()).unwrap_or_default();
            Some(std::fs::copy(&src, &dst).map(|_| RVal::Void).map_err(|e| RavaError::JavaException {
                exception_type: "IOException".into(), message: e.to_string(),
            }))
        }
        id if id == fnv("Files.move") => {
            let src = args.first().map(|v| v.to_display()).unwrap_or_default();
            let dst = args.get(1).map(|v| v.to_display()).unwrap_or_default();
            Some(std::fs::rename(&src, &dst).map(|_| RVal::Void).map_err(|e| RavaError::JavaException {
                exception_type: "IOException".into(), message: e.to_string(),
            }))
        }
        id if id == fnv("Files.size") => {
            let path = args.first().map(|v| v.to_display()).unwrap_or_default();
            Some(std::fs::metadata(&path).map(|m| RVal::Int(m.len() as i64)).map_err(|e| RavaError::JavaException {
                exception_type: "IOException".into(), message: e.to_string(),
            }))
        }
        id if id == fnv("Files.list") || id == fnv("Files.walk") => {
            let path = args.first().map(|v| v.to_display()).unwrap_or_default();
            let entries: Vec<RVal> = std::fs::read_dir(&path)
                .map(|rd| rd.filter_map(|e| e.ok()).map(|e| RVal::Str(e.path().to_string_lossy().into_owned())).collect())
                .unwrap_or_default();
            Some(Ok(RVal::Array(Rc::new(RefCell::new(entries)))))
        }
        id if id == fnv("Paths.get") || id == fnv("Path.of") => {
            let path = args.iter().map(|v| v.to_display()).collect::<Vec<_>>().join("/");
            Some(Ok(RVal::Str(path)))
        }
        id if id == fnv("File") || id == fnv("File.<init>") => {
            Some(Ok(RVal::Str(args.first().map(|v| v.to_display()).unwrap_or_default())))
        }
        _ => None,
    }
}
