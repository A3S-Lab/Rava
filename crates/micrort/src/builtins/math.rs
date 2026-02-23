//! Java Math class builtins.

use rava_common::error::Result;
use crate::rir_interp::RVal;
use super::format::{fnv, rand_f64};

pub fn dispatch(func_id: u32, args: &[RVal]) -> Option<Result<RVal>> {
    let f1 = || args.first().map(|v| v.as_float()).unwrap_or(0.0);
    let i1 = || args.first().map(|v| v.as_int()).unwrap_or(0);

    match func_id {
        id if id == fnv("Math.max") => {
            let a = i1(); let b = args.get(1).map(|v| v.as_int()).unwrap_or(0);
            Some(Ok(RVal::Int(a.max(b))))
        }
        id if id == fnv("Math.min") => {
            let a = i1(); let b = args.get(1).map(|v| v.as_int()).unwrap_or(0);
            Some(Ok(RVal::Int(a.min(b))))
        }
        id if id == fnv("Math.abs") => Some(Ok(if matches!(args.first(), Some(RVal::Float(_))) {
            RVal::Float(f1().abs())
        } else {
            RVal::Int(i1().abs())
        })),
        id if id == fnv("Math.pow")   => { let b = args.get(1).map(|v| v.as_float()).unwrap_or(0.0); Some(Ok(RVal::Float(f1().powf(b)))) }
        id if id == fnv("Math.sqrt")  => Some(Ok(RVal::Float(f1().sqrt()))),
        id if id == fnv("Math.cbrt")  => Some(Ok(RVal::Float(f1().cbrt()))),
        id if id == fnv("Math.floor") => Some(Ok(RVal::Float(f1().floor()))),
        id if id == fnv("Math.ceil")  => Some(Ok(RVal::Float(f1().ceil()))),
        id if id == fnv("Math.round") => Some(Ok(RVal::Int(f1().round() as i64))),
        id if id == fnv("Math.log")   => Some(Ok(RVal::Float(f1().ln()))),
        id if id == fnv("Math.log10") => Some(Ok(RVal::Float(f1().log10()))),
        id if id == fnv("Math.log1p") => Some(Ok(RVal::Float(f1().ln_1p()))),
        id if id == fnv("Math.exp")   => Some(Ok(RVal::Float(f1().exp()))),
        id if id == fnv("Math.sin")   => Some(Ok(RVal::Float(f1().sin()))),
        id if id == fnv("Math.cos")   => Some(Ok(RVal::Float(f1().cos()))),
        id if id == fnv("Math.tan")   => Some(Ok(RVal::Float(f1().tan()))),
        id if id == fnv("Math.asin")  => Some(Ok(RVal::Float(f1().asin()))),
        id if id == fnv("Math.acos")  => Some(Ok(RVal::Float(f1().acos()))),
        id if id == fnv("Math.atan")  => Some(Ok(RVal::Float(f1().atan()))),
        id if id == fnv("Math.atan2") => { let x = args.get(1).map(|v| v.as_float()).unwrap_or(0.0); Some(Ok(RVal::Float(f1().atan2(x)))) }
        id if id == fnv("Math.hypot") => { let b = args.get(1).map(|v| v.as_float()).unwrap_or(0.0); Some(Ok(RVal::Float(f1().hypot(b)))) }
        id if id == fnv("Math.signum")    => Some(Ok(RVal::Float(f1().signum()))),
        id if id == fnv("Math.toRadians") => Some(Ok(RVal::Float(f1().to_radians()))),
        id if id == fnv("Math.toDegrees") => Some(Ok(RVal::Float(f1().to_degrees()))),
        id if id == fnv("Math.random")    => Some(Ok(RVal::Float(rand_f64()))),
        id if id == fnv("Math.PI") => Some(Ok(RVal::Float(std::f64::consts::PI))),
        id if id == fnv("Math.E")  => Some(Ok(RVal::Float(std::f64::consts::E))),
        id if id == fnv("Math.floorDiv") => {
            let a = i1(); let b = args.get(1).map(|v| v.as_int()).unwrap_or(1);
            Some(Ok(RVal::Int(if b == 0 { 0 } else { a.div_euclid(b) })))
        }
        id if id == fnv("Math.floorMod") => {
            let a = i1(); let b = args.get(1).map(|v| v.as_int()).unwrap_or(1);
            Some(Ok(RVal::Int(if b == 0 { 0 } else { a.rem_euclid(b) })))
        }
        id if id == fnv("Math.ceilDiv") => {
            let a = i1(); let b = args.get(1).map(|v| v.as_int()).unwrap_or(1);
            Some(Ok(RVal::Int(if b == 0 { 0 } else { (a + b - 1) / b })))
        }
        _ => None,
    }
}
