//! Package manager — §28.
//!
//! Pipeline:
//!   ShortNameResolver → VersionResolver → DependencyGraph →
//!   LockfileGen → ArtifactDownloader
//!
//! Extension points:
//!   - [`ClassResolver`] — where to fetch packages (default: Maven Central)

pub mod config;
pub mod lockfile;
pub mod registry;
pub mod resolver;
pub mod shortname;

pub use config::ProjectConfig;
pub use lockfile::{LockedPackage, Lockfile};
pub use registry::ClassResolver;
pub use resolver::DependencyGraph;
pub use shortname::ShortNameRegistry;
