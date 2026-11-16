//! Heroic adapter. Heroic exports PROTON_BATTLEYE_RUNTIME=<tools>/runtimes/battleye_runtime
//! when the per-game `battlEyeRuntime` setting is on (src/backend/launcher.ts).
use super::{proton_tools, Detected, Runner};
use crate::ctx::{self, Ctx};

pub fn detect(c: &Ctx) -> Detected {
    let mut d = Detected {
        launcher: "Heroic",
        ..Default::default()
    };
    for base in [
        c.xdg("XDG_CONFIG_HOME", ".config").join("heroic"),
        c.home_path(".var/app/com.heroicgameslauncher.hgl/config/heroic"),
    ] {
        if !base.is_dir() {
            continue;
        }
        d.present = true;
        d.runners.extend(proton_tools(&base.join("tools/proton")));
        for w in ctx::subdirs(&base.join("tools/wine")) {
            d.runners.push(Runner {
                name: w.file_name().unwrap().to_string_lossy().into_owned(),
                path: w,
                version: None,
            });
        }
        if d.be_runtime.is_none() {
            d.be_runtime = Some(base.join("tools/runtimes/battleye_runtime"));
        }
        let on = ctx::files(&base.join("GamesConfig"))
            .iter()
            .filter_map(|f| ctx::read(f))
            .filter(|s| {
                s.split_whitespace()
                    .collect::<String>()
                    .contains("\"battlEyeRuntime\":true")
            })
            .count();
        d.notes
            .push(format!("{on} game config(s) with battlEyeRuntime enabled"));
        d.search_roots.push(base.join("Prefixes"));
        d.config_dirs.push(base);
    }
    let games = c.home_path("Games/Heroic");
    if games.is_dir() {
        d.search_roots.push(games);
    }
    d
}
