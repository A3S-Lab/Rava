//! `rava run` — run a Java source file directly.

use anyhow::{Context, Result};
use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct RunArgs {
    /// Java source file to run (omit to use main class from rava.hcl)
    pub file: Option<PathBuf>,

    /// Watch for file changes and restart automatically
    #[arg(long)]
    pub watch: bool,

    /// Dependency JARs to load alongside a `.class`/`.jar` (comma-separated or repeated)
    #[arg(long, short = 'c', value_delimiter = ',')]
    pub classpath: Vec<PathBuf>,

    /// Arguments passed to the Java program
    #[arg(last = true)]
    pub program_args: Vec<String>,
}

pub fn run(args: RunArgs) -> Result<()> {
    if args.watch {
        return super::watch::run_watch_loop(
            "java sources",
            || run_once(&args),
            || watch_paths(&args),
        );
    }

    run_once(&args)
}

fn run_once(args: &RunArgs) -> Result<()> {
    // Run a pre-compiled `.class` file directly (bytecode → RIR → interpreter).
    if let Some(ref file) = args.file {
        let ext = file.extension().and_then(|e| e.to_str());
        if ext == Some("class") || ext == Some("jar") {
            let bytes = std::fs::read(file)
                .with_context(|| format!("cannot read {}", file.display()))?;
            let module = if ext == Some("jar") {
                // Load the main JAR + resolved deps from rava.lock + explicit -c jars.
                let mut jars: Vec<Vec<u8>> = vec![bytes];
                jars.extend(lockfile_dep_jars());
                for cp in &args.classpath {
                    jars.push(
                        std::fs::read(cp)
                            .with_context(|| format!("cannot read classpath jar {}", cp.display()))?,
                    );
                }
                let refs: Vec<&[u8]> = jars.iter().map(|v| v.as_slice()).collect();
                rava_micrort::bytecode::load_jars(&refs)
            } else {
                rava_micrort::bytecode::load_class_module(&bytes)
            }
            .map_err(|e| anyhow::anyhow!("class load error: {}", e))?;
            return rava_micrort::RirInterpreter::new(module)
                .run_main()
                .map_err(|e| anyhow::anyhow!("runtime error: {}", e));
        }
    }

    let compiler = rava_frontend::Compiler::new();

    let module = if let Some(ref file) = args.file {
        let source = std::fs::read_to_string(file)
            .with_context(|| format!("cannot read {}", file.display()))?;
        compiler
            .compile(&source, file)
            .map_err(|e| anyhow::anyhow!("compile error: {}", e))?
    } else {
        // Project mode
        #[cfg(all(feature = "aot", feature = "pkg"))]
        {
            let _main = super::build::resolve_main_from_hcl_pub()?;
            let files = super::build::collect_source_files()?;
            if files.is_empty() {
                anyhow::bail!("no .java files found in src/");
            }
            compiler
                .compile_project(&files)
                .map_err(|e| anyhow::anyhow!("compile error: {}", e))?
        }
        #[cfg(not(all(feature = "aot", feature = "pkg")))]
        anyhow::bail!("no file specified — pass a .java file")
    };

    let interp = rava_micrort::RirInterpreter::new(module);
    interp
        .run_main()
        .map_err(|e| anyhow::anyhow!("runtime error: {}", e))?;

    Ok(())
}

/// Auto-load dependency JARs listed in `rava.lock` (downloading any not yet cached).
/// Best-effort: a dependency that fails to resolve/read is skipped rather than aborting.
#[cfg(feature = "pkg")]
fn lockfile_dep_jars() -> Vec<Vec<u8>> {
    let Ok(cache_dir) = rava_pkg::resolver::default_cache_dir() else {
        return Vec::new();
    };
    collect_lock_jars(std::path::Path::new("rava.lock"), &cache_dir)
}

/// Read every dependency JAR named in `lock_path` from `cache_dir` (downloading misses).
/// Pure-ish helper split out for testing: cached deps need no network.
#[cfg(feature = "pkg")]
fn collect_lock_jars(lock_path: &std::path::Path, cache_dir: &std::path::Path) -> Vec<Vec<u8>> {
    let Ok(lock) = rava_pkg::Lockfile::from_file(lock_path) else {
        return Vec::new();
    };
    let mut jars = Vec::new();
    for pkg in &lock.packages {
        if let Ok(path) = rava_pkg::resolver::download_jar(
            &pkg.group_id,
            &pkg.artifact_id,
            &pkg.version,
            cache_dir,
        ) {
            if let Ok(bytes) = std::fs::read(&path) {
                jars.push(bytes);
            }
        }
    }
    jars
}

#[cfg(not(feature = "pkg"))]
fn lockfile_dep_jars() -> Vec<Vec<u8>> {
    Vec::new()
}

#[cfg(all(test, feature = "pkg"))]
mod tests {
    use super::*;
    use std::fs;

    // RAII temp dir (repo convention — see codegen-cranelift/tests/aot_e2e.rs).
    struct TmpDir(PathBuf);
    impl Drop for TmpDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn tmp(id: &str) -> TmpDir {
        let d = TmpDir(std::env::temp_dir().join(format!("rava_lock_{}_{}", std::process::id(), id)));
        let _ = fs::remove_dir_all(&d.0);
        fs::create_dir_all(&d.0).unwrap();
        d
    }

    #[test]
    fn collect_lock_jars_reads_cached_dep() {
        let t = tmp("cached");
        // Pre-place a "cached" jar so download_jar returns it without any network.
        let jar_dir = t.0.join("local").join("mathlib").join("1.0");
        fs::create_dir_all(&jar_dir).unwrap();
        fs::write(jar_dir.join("mathlib-1.0.jar"), b"jarbytes").unwrap();

        let lock_path = t.0.join("rava.lock");
        fs::write(
            &lock_path,
            r#"packages = [{
  group_id = "local"
  artifact_id = "mathlib"
  version = "1.0"
  sha256 = "x"
  url = "file"
  dependencies = []
}]"#,
        )
        .unwrap();

        let jars = collect_lock_jars(&lock_path, &t.0);
        assert_eq!(jars, vec![b"jarbytes".to_vec()]);
    }

    #[test]
    fn collect_lock_jars_missing_lock_is_empty() {
        let t = tmp("missing");
        let jars = collect_lock_jars(&t.0.join("nope.lock"), &t.0);
        assert!(jars.is_empty());
    }
}

fn watch_paths(args: &RunArgs) -> Result<Vec<PathBuf>> {
    if let Some(file) = &args.file {
        return Ok(vec![file.clone()]);
    }

    #[cfg(all(feature = "aot", feature = "pkg"))]
    {
        let mut files = super::build::collect_source_files()?;
        files.push(PathBuf::from("rava.hcl"));
        files.sort();
        files.dedup();
        Ok(files)
    }

    #[cfg(not(all(feature = "aot", feature = "pkg")))]
    {
        anyhow::bail!("watch mode requires a file argument");
    }
}
