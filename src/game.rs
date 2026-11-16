//! Locating a game install and inspecting the files the community workaround touches.
use crate::ctx::{self, Ctx};
use crate::titles::Title;
use std::path::{Path, PathBuf};

/// Marker the GTAOnlineLinux workaround writes at the top and bottom of its startup.meta.
pub const WORKAROUND_MARKER: &str = "<!--GTAOnlineLinux-->";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetaState {
    Absent,
    Workaround,
    Other,
}

#[derive(Debug, Clone)]
pub struct Install {
    pub path: PathBuf,
    pub exes: Vec<String>,
    pub battleye_files: Vec<String>,
    /// (relative path, state) for x64/data/startup.meta and update/x64/data/startup.meta
    pub startup_meta: Vec<(String, MetaState)>,
}

pub const META_LOCATIONS: [&str; 2] = ["x64/data/startup.meta", "update/x64/data/startup.meta"];

pub fn meta_state(p: &Path) -> MetaState {
    match ctx::read(p) {
        None => MetaState::Absent,
        Some(s) if s.contains(WORKAROUND_MARKER) => MetaState::Workaround,
        Some(_) => MetaState::Other,
    }
}

pub fn inspect(path: &Path, t: &Title) -> Install {
    let exes = t
        .exe_names
        .iter()
        .filter(|e| path.join(e).is_file())
        .cloned()
        .collect();
    let battleye_files = ctx::files(&path.join("BattlEye"))
        .iter()
        .filter_map(|f| f.file_name().map(|n| n.to_string_lossy().into_owned()))
        .collect();
    let startup_meta = META_LOCATIONS
        .iter()
        .map(|rel| (rel.to_string(), meta_state(&path.join(rel))))
        .collect();
    Install {
        path: path.to_path_buf(),
        exes,
        battleye_files,
        startup_meta,
    }
}

/// Bounded, symlink-free search for a directory with one of `names`.
pub fn find_named_dirs(root: &Path, names: &[String], depth: u8, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(root) else {
        return;
    };
    for e in rd.flatten() {
        let Ok(ft) = e.file_type() else { continue };
        if !ft.is_dir() {
            continue; // also skips symlinks such as dosdevices/z: -> /
        }
        let p = e.path();
        if names
            .iter()
            .any(|n| e.file_name().to_string_lossy() == n.as_str())
        {
            out.push(p);
        } else if depth > 0 {
            find_named_dirs(&p, names, depth - 1, out);
        }
    }
}

pub fn find_installs(c: &Ctx, roots: &[PathBuf], t: &Title) -> Vec<Install> {
    let mut dirs = vec![];
    for r in roots.iter().chain(c.game_dirs.iter()) {
        if r.file_name().is_some_and(|n| {
            t.dir_names
                .iter()
                .any(|d| n.to_string_lossy() == d.as_str())
        }) {
            dirs.push(r.clone());
        } else {
            find_named_dirs(r, &t.dir_names, 5, &mut dirs);
        }
    }
    dirs.sort();
    dirs.dedup();
    dirs.iter().map(|d| inspect(d, t)).collect()
}

/// Move a workaround startup.meta out of the game tree into the state dir.
/// Refuses to touch a file that does not carry the workaround marker.
pub fn quarantine_startup_meta(c: &Ctx, file: &Path, ts: u64) -> Result<PathBuf, String> {
    if meta_state(file) != MetaState::Workaround {
        return Err(format!(
            "{} is absent or not the GTAOnlineLinux workaround file; refusing to touch it",
            file.display()
        ));
    }
    let qdir = c.state_dir().join("quarantine").join(ts.to_string());
    std::fs::create_dir_all(&qdir).map_err(|e| format!("cannot create {}: {e}", qdir.display()))?;
    let dest = qdir.join("startup.meta");
    std::fs::copy(file, &dest).map_err(|e| format!("cannot copy to {}: {e}", dest.display()))?;
    std::fs::write(qdir.join("origin.txt"), file.to_string_lossy().as_bytes())
        .map_err(|e| e.to_string())?;
    std::fs::remove_file(file).map_err(|e| {
        format!(
            "copied to quarantine but cannot remove {}: {e}",
            file.display()
        )
    })?;
    Ok(dest)
}

/// Put the most recently quarantined file for `file` back.
pub fn restore_startup_meta(c: &Ctx, file: &Path) -> Result<PathBuf, String> {
    if file.exists() {
        return Err(format!(
            "{} already exists; not overwriting",
            file.display()
        ));
    }
    let mut entries = ctx::subdirs(&c.state_dir().join("quarantine"));
    entries.sort_by_key(|p| {
        p.file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.parse::<u64>().ok())
            .unwrap_or(0)
    });
    for q in entries.iter().rev() {
        if ctx::read(&q.join("origin.txt")).as_deref() == Some(&*file.to_string_lossy()) {
            let src = q.join("startup.meta");
            std::fs::copy(&src, file)
                .map_err(|e| format!("cannot restore {}: {e}", file.display()))?;
            return Ok(src);
        }
    }
    Err(format!(
        "no quarantined copy recorded for {}",
        file.display()
    ))
}
