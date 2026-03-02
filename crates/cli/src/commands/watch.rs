use anyhow::Result;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

type Snapshot = BTreeMap<PathBuf, Option<(u64, SystemTime)>>;

pub fn run_watch_loop<Run, Collect>(
    label: &str,
    mut run_once: Run,
    collect_paths: Collect,
) -> Result<()>
where
    Run: FnMut() -> Result<()>,
    Collect: Fn() -> Result<Vec<PathBuf>>,
{
    run_once()?;
    let mut snapshot = snapshot_paths(&collect_paths()?);

    eprintln!("watching {label} (Ctrl+C to stop)");
    loop {
        std::thread::sleep(Duration::from_millis(400));

        let paths = collect_paths()?;
        let next = snapshot_paths(&paths);
        if next != snapshot {
            snapshot = next;
            eprintln!("change detected, rerunning...");
            if let Err(err) = run_once() {
                eprintln!("run failed: {err}");
            }
        }
    }
}

fn snapshot_paths(paths: &[PathBuf]) -> Snapshot {
    let mut snapshot = BTreeMap::new();
    for path in paths {
        snapshot.insert(path.clone(), file_stamp(path));
    }
    snapshot
}

fn file_stamp(path: &Path) -> Option<(u64, SystemTime)> {
    let meta = std::fs::metadata(path).ok()?;
    let modified = meta.modified().ok()?;
    Some((meta.len(), modified))
}
