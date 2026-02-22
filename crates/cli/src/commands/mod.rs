//! CLI subcommands.

#[cfg(feature = "pkg")]
pub mod add;
#[cfg(feature = "aot")]
pub mod build;
pub mod fmt;
#[cfg(feature = "pkg")]
pub mod init;
pub mod run;
pub mod test;
