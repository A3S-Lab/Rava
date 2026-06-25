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
pub mod pom;
pub mod registry;
pub mod resolver;
pub mod shortname;

pub use config::{BuildConfig, ProjectConfig, ProjectMeta};
pub use pom::parse_pom_dependencies;
pub use lockfile::{LockedPackage, Lockfile};
pub use registry::{resolve_closure, ClassResolver, Dependency, MavenCentralResolver, ResolvedArtifact};
pub use resolver::{download_pom, latest_version, parse_coordinate, DependencyGraph};
pub use shortname::ShortNameRegistry;
