//! Java concurrency stubs: Thread, Atomic*, Lock, etc.
//! Single-threaded interpreter — most ops are no-ops or simplified.

use super::format::fnv;
use crate::rir_interp::RVal;
use rava_common::error::Result;

pub fn dispatch(func_id: u32, args: &[RVal]) -> Option<Result<RVal>> {
    match func_id {
        id if id == fnv("Thread.sleep") => {
            let ms = args.first().map(|v| v.as_int()).unwrap_or(0).max(0) as u64;
            std::thread::sleep(std::time::Duration::from_millis(ms));
            Some(Ok(RVal::Void))
        }
        id if id == fnv("Thread.currentThread") => Some(Ok(RVal::Str("main".into()))),
        id if id == fnv("Thread.yield") => Some(Ok(RVal::Void)),
        id if id == fnv("Thread.interrupted") => Some(Ok(RVal::Bool(false))),

        id if id == fnv("AtomicInteger")
            || id == fnv("AtomicInteger.<init>")
            || id == fnv("AtomicLong")
            || id == fnv("AtomicLong.<init>") =>
        {
            Some(Ok(RVal::Int(args.first().map(|v| v.as_int()).unwrap_or(0))))
        }
        id if id == fnv("AtomicBoolean") || id == fnv("AtomicBoolean.<init>") => Some(Ok(
            RVal::Bool(args.first().map(|v| v.is_truthy()).unwrap_or(false)),
        )),
        id if id == fnv("AtomicReference") || id == fnv("AtomicReference.<init>") => {
            Some(Ok(args.first().cloned().unwrap_or(RVal::Null)))
        }

        // Lock / synchronization stubs
        id if id == fnv("ReentrantLock")
            || id == fnv("ReentrantLock.<init>")
            || id == fnv("ReentrantReadWriteLock")
            || id == fnv("ReentrantReadWriteLock.<init>")
            || id == fnv("CountDownLatch")
            || id == fnv("CountDownLatch.<init>")
            || id == fnv("Semaphore")
            || id == fnv("Semaphore.<init>")
            || id == fnv("CyclicBarrier")
            || id == fnv("CyclicBarrier.<init>") =>
        {
            Some(Ok(RVal::Null))
        }

        // Executor stubs
        id if id == fnv("Executors.newFixedThreadPool")
            || id == fnv("Executors.newSingleThreadExecutor")
            || id == fnv("Executors.newCachedThreadPool")
            || id == fnv("Executors.newScheduledThreadPool") =>
        {
            Some(Ok(RVal::Null))
        }

        _ => None,
    }
}

/// Instance methods on concurrency objects (AtomicInteger, Lock, etc.)
pub fn dispatch_named(method: &str, args: &[RVal]) -> Option<Result<RVal>> {
    match method {
        // AtomicInteger / AtomicLong methods — only match when receiver looks like an atomic value
        // Note: "get" is intentionally NOT matched here to avoid intercepting HashMap.get()
        "getAndSet" => Some(Ok(args.first().cloned().unwrap_or(RVal::Int(0)))),
        "set" => Some(Ok(RVal::Void)),
        "getAndIncrement" => Some(Ok(args.first().cloned().unwrap_or(RVal::Int(0)))),
        "getAndDecrement" => Some(Ok(args.first().cloned().unwrap_or(RVal::Int(0)))),
        "incrementAndGet" => Some(Ok(RVal::Int(
            args.first().map(|v| v.as_int()).unwrap_or(0) + 1,
        ))),
        "decrementAndGet" => Some(Ok(RVal::Int(
            args.first().map(|v| v.as_int()).unwrap_or(0) - 1,
        ))),
        "addAndGet" => Some(Ok(RVal::Int(
            args.first().map(|v| v.as_int()).unwrap_or(0)
                + args.get(1).map(|v| v.as_int()).unwrap_or(0),
        ))),
        "compareAndSet" | "compareAndExchange" => Some(Ok(RVal::Bool(true))),
        // Lock methods — no-op
        "lock" | "unlock" | "tryLock" | "lockInterruptibly" => Some(Ok(RVal::Void)),
        "await" | "signal" | "signalAll" => Some(Ok(RVal::Void)),
        "countDown" | "release" | "acquire" => Some(Ok(RVal::Void)),
        "await_count" => Some(Ok(RVal::Void)),
        "submit" | "execute" | "shutdown" | "shutdownNow" => Some(Ok(RVal::Null)),
        _ => None,
    }
}
