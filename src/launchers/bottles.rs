//! Bottles adapter: detection only. Bottles has no BattlEye runtime integration of its own.
use super::{Detected, Runner};
use crate::ctx::{self, Ctx};

pub fn detect(c: &Ctx) -> Detected {
    let mut d = Detected {
        launcher: "Bottles",
        ..Default::default()
    };
    for base in [
        c.xdg("XDG_DATA_HOME", ".local/share").join("bottles"),
        c.home_path(".var/app/com.usebottles.bottles/data/bottles"),
    ] {
        if !base.is_dir() {
            continue;
        }
        d.present = true;
        for r in ctx::subdirs(&base.join("runners")) {
            d.runners.push(Runner {
                name: r.file_name().unwrap().to_string_lossy().into_owned(),
                path: r,
                version: None,
            });
        }
        d.search_roots.extend(ctx::subdirs(&base.join("bottles")));
        d.config_dirs.push(base);
    }
    if d.present {
        d.notes.push("no PROTON_BATTLEYE_RUNTIME integration; a vendor-enabled title would need it set per bottle".into());
    }
    d
}
