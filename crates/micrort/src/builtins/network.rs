//! Java network API stubs: java.net.

use super::format::fnv;
use crate::rir_interp::RVal;
use rava_common::error::Result;

pub fn dispatch(func_id: u32, args: &[RVal]) -> Option<Result<RVal>> {
    match func_id {
        id if id == fnv("InetAddress.getLocalHost") => Some(Ok(RVal::Str("localhost".into()))),
        id if id == fnv("InetAddress.getByName") => Some(Ok(RVal::Str(
            args.first()
                .map(|v| v.to_display())
                .unwrap_or_else(|| "127.0.0.1".into()),
        ))),
        id if id == fnv("InetAddress.getHostName") || id == fnv("InetAddress.getHostAddress") => {
            Some(Ok(RVal::Str("127.0.0.1".into())))
        }
        id if id == fnv("URL") || id == fnv("URL.<init>") => Some(Ok(RVal::Str(
            args.first().map(|v| v.to_display()).unwrap_or_default(),
        ))),
        id if id == fnv("URI.create") || id == fnv("URI") || id == fnv("URI.<init>") => Some(Ok(
            RVal::Str(args.first().map(|v| v.to_display()).unwrap_or_default()),
        )),
        id if id == fnv("URI.toString")
            || id == fnv("URL.toString")
            || id == fnv("URL.toExternalForm") =>
        {
            Some(Ok(RVal::Str(
                args.first().map(|v| v.to_display()).unwrap_or_default(),
            )))
        }
        _ => None,
    }
}
