//! Thin launcher adapters over one shared model. Each adapter only *reads*
//! the launcher's documented on-disk configuration.
use crate::ctx::Ctx;
use std::path::PathBuf;

pub mod bottles;
pub mod heroic;
pub mod lutris;
pub mod steam;

#[derive(Debug, Clone, Default)]
pub struct Runner {
    pub name: String,
    pub path: PathBuf,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Detected {
    pub launcher: &'static str,
    pub present: bool,
    pub config_dirs: Vec<PathBuf>,
    pub runners: Vec<Runner>,
    /// Directory this launcher exports as PROTON_BATTLEYE_RUNTIME, if any.
    pub be_runtime: Option<PathBuf>,
    /// Roots worth searching for game installs (prefixes, libraries).
    pub search_roots: Vec<PathBuf>,
    pub notes: Vec<String>,
}

pub fn detect_all(c: &Ctx) -> Vec<Detected> {
    vec![
        steam::detect(c),
        lutris::detect(c).0,
        heroic::detect(c),
        bottles::detect(c),
    ]
}

/// Proton-style tool directories (contain a `proton` script); version from the `version` file.
pub fn proton_tools(dir: &std::path::Path) -> Vec<Runner> {
    crate::ctx::subdirs(dir)
        .into_iter()
        .filter(|d| d.join("proton").is_file())
        .map(|d| Runner {
            name: d.file_name().unwrap().to_string_lossy().into_owned(),
            version: crate::ctx::read(&d.join("version")).map(|v| v.trim().to_string()),
            path: d,
        })
        .collect()
}
