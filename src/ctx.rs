//! Execution context. All filesystem and environment access goes through this
//! so that tests can point the tool at a fixture tree instead of the real system.
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Ctx {
    /// Prefix for absolute system paths (`/etc`, `/sys`, `/proc`, `/usr`). `/` in production.
    pub root: PathBuf,
    /// The invoking user's home directory.
    pub home: PathBuf,
    pub env: HashMap<String, String>,
    /// Extra directories the user explicitly allowed us to search for the game.
    pub game_dirs: Vec<PathBuf>,
    /// Allow spawning read-only helper commands (`vulkaninfo --summary`).
    pub run_commands: bool,
}

impl Ctx {
    pub fn from_process() -> Ctx {
        let env: HashMap<String, String> = std::env::vars().collect();
        let home = env
            .get("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/"));
        Ctx {
            root: PathBuf::from("/"),
            home,
            env,
            game_dirs: vec![],
            run_commands: true,
        }
    }

    /// Map an absolute system path into the (possibly fake) root.
    pub fn sys(&self, abs: &str) -> PathBuf {
        self.root.join(abs.trim_start_matches('/'))
    }

    pub fn home_path(&self, rel: &str) -> PathBuf {
        self.home.join(rel)
    }

    pub fn xdg(&self, var: &str, default_rel: &str) -> PathBuf {
        match self.env.get(var) {
            Some(v) if !v.is_empty() => PathBuf::from(v),
            _ => self.home.join(default_rel),
        }
    }

    pub fn state_dir(&self) -> PathBuf {
        self.xdg("XDG_STATE_HOME", ".local/state")
            .join("gtaol-doctor")
    }
}

pub fn read(p: &Path) -> Option<String> {
    std::fs::read(p)
        .ok()
        .map(|b| String::from_utf8_lossy(&b).into_owned())
}

pub fn subdirs(p: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = match std::fs::read_dir(p) {
        Ok(rd) => rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect(),
        Err(_) => vec![],
    };
    v.sort();
    v
}

pub fn files(p: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = match std::fs::read_dir(p) {
        Ok(rd) => rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_file())
            .collect(),
        Err(_) => vec![],
    };
    v.sort();
    v
}

pub fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
