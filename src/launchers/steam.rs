use super::{proton_tools, Detected};
use crate::ctx::{self, Ctx};
use std::path::PathBuf;

pub const BATTLEYE_RUNTIME_APPID: &str = "1161040";
pub const BATTLEYE_RUNTIME_DIR: &str = "Proton BattlEye Runtime";

pub fn roots(c: &Ctx) -> Vec<PathBuf> {
    [
        ".local/share/Steam",
        ".steam/steam",
        ".var/app/com.valvesoftware.Steam/.local/share/Steam",
    ]
    .iter()
    .map(|r| c.home_path(r))
    .filter(|p| p.join("steamapps").is_dir())
    .collect()
}

/// Extract `"path"  "<dir>"` values from libraryfolders.vdf.
pub fn library_paths(vdf: &str) -> Vec<PathBuf> {
    vdf.lines()
        .filter_map(|l| {
            let mut q = l.trim().split('"').filter(|s| !s.trim().is_empty());
            if q.next()? != "path" {
                return None;
            }
            Some(PathBuf::from(q.next()?.replace("\\\\", "\\")))
        })
        .collect()
}

pub fn detect(c: &Ctx) -> Detected {
    let mut d = Detected {
        launcher: "Steam",
        ..Default::default()
    };
    let roots = roots(c);
    d.present = !roots.is_empty();
    let mut libs: Vec<PathBuf> = vec![];
    for r in &roots {
        libs.push(r.clone());
        if let Some(vdf) = ctx::read(&r.join("steamapps/libraryfolders.vdf")) {
            libs.extend(library_paths(&vdf));
        }
        d.runners
            .extend(proton_tools(&r.join("compatibilitytools.d")));
    }
    libs.retain(|l| l.join("steamapps").is_dir());
    libs.sort();
    libs.dedup();
    for l in &libs {
        d.runners.extend(proton_tools(&l.join("steamapps/common")));
        let rt = l.join("steamapps/common").join(BATTLEYE_RUNTIME_DIR);
        if d.be_runtime.is_none() && rt.is_dir() {
            d.be_runtime = Some(rt);
        }
        d.search_roots.push(l.join("steamapps/common"));
    }
    d.config_dirs = roots;
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_library_paths() {
        let vdf = "\"libraryfolders\"\n{\n\t\"0\"\n\t{\n\t\t\"path\"\t\t\"/home/u/.local/share/Steam\"\n\t\t\"label\"\t\t\"\"\n\t}\n\t\"1\"\n\t{\n\t\t\"path\"\t\t\"/mnt/games/SteamLibrary\"\n\t}\n}\n";
        assert_eq!(
            library_paths(vdf),
            [
                PathBuf::from("/home/u/.local/share/Steam"),
                PathBuf::from("/mnt/games/SteamLibrary")
            ]
        );
    }
}
