//! CLI subcommands.

#[cfg(feature = "pkg")]
pub mod add;
#[cfg(feature = "aot")]
pub mod build;
#[cfg(feature = "pkg")]
pub mod deps;
pub mod fmt;
#[cfg(feature = "pkg")]
pub mod init;
#[cfg(feature = "pkg")]
pub mod remove;
pub mod run;
pub mod test;
#[cfg(feature = "pkg")]
pub mod update;
mod watch;
