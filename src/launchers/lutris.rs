//! Lutris adapter.
//!
//! Behaviour modelled on lutris/runners/wine.py (master, read 2026-11-16):
//!   if runner_config.get("battleye"): env["PROTON_BATTLEYE_RUNTIME"] = RUNTIME_DIR/battleye_runtime
//! That assignment happens *after* the user's "Environment variables" are merged
//! (runner.py: env.update(system_config.get("env"))), so with the BattlEye toggle
//! on, a user-supplied PROTON_BATTLEYE_RUNTIME is overwritten. Both options default to True.
use super::{proton_tools, Detected, Runner};
use crate::ctx::{self, Ctx};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Minimal reader for the subset of YAML Lutris writes: nested mappings of
/// scalars by indentation. Returns dotted keys, e.g. `system.env.FOO`.
pub fn flatten_yaml(src: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let mut stack: Vec<(usize, String)> = vec![];
    for line in src.lines() {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("- ") {
            continue;
        }
        let indent = line.len() - trimmed.len();
        let Some((k, v)) = trimmed.split_once(':') else {
            continue;
        };
        while stack.last().is_some_and(|(i, _)| *i >= indent) {
            stack.pop();
        }
        let key = k.trim().trim_matches(|c| c == '\'' || c == '"').to_string();
        let v = v.trim();
        if v.is_empty() {
            stack.push((indent, key));
        } else {
            let mut full: Vec<&str> = stack.iter().map(|(_, k)| k.as_str()).collect();
            full.push(&key);
            let v = if v.len() >= 2
                && (v.starts_with('\'') && v.ends_with('\'')
                    || v.starts_with('"') && v.ends_with('"'))
            {
                &v[1..v.len() - 1]
            } else {
                v
            };
            out.insert(full.join("."), v.to_string());
        }
    }
    out
}

fn truthy(v: Option<&String>, default: bool) -> bool {
    match v.map(|s| s.to_ascii_lowercase()) {
        Some(s) => matches!(s.as_str(), "true" | "yes" | "on" | "1"),
        None => default,
    }
}

#[derive(Debug, Clone, Default)]
pub struct GameConfig {
    pub file: PathBuf,
    pub prefix: Option<PathBuf>,
    pub exe: Option<String>,
    pub wine_version: Option<String>,
    pub battleye: bool,
    pub eac: bool,
    pub env: BTreeMap<String, String>,
}

pub fn parse_game(
    file: &Path,
    src: &str,
    runner_defaults: &BTreeMap<String, String>,
) -> GameConfig {
    let m = flatten_yaml(src);
    let get = |k: &str| m.get(k).or_else(|| runner_defaults.get(k));
    GameConfig {
        file: file.to_path_buf(),
        prefix: m.get("game.prefix").map(PathBuf::from),
        exe: m.get("game.exe").cloned(),
        wine_version: get("wine.version").cloned(),
        battleye: truthy(get("wine.battleye"), true),
        eac: truthy(get("wine.eac"), true),
        env: m
            .iter()
            .filter_map(|(k, v)| {
                k.strip_prefix("system.env.")
                    .map(|n| (n.to_string(), v.clone()))
            })
            .collect(),
    }
}

fn bases(c: &Ctx) -> Vec<(PathBuf, PathBuf)> {
    // (config dir, data dir) for native and Flatpak installs
    let flat = c.home_path(".var/app/net.lutris.Lutris");
    vec![
        (
            c.xdg("XDG_CONFIG_HOME", ".config").join("lutris"),
            c.xdg("XDG_DATA_HOME", ".local/share").join("lutris"),
        ),
        (flat.join("config/lutris"), flat.join("data/lutris")),
    ]
}

pub fn detect(c: &Ctx) -> (Detected, Vec<GameConfig>) {
    let mut d = Detected {
        launcher: "Lutris",
        ..Default::default()
    };
    let mut games = vec![];
    for (cfg, data) in bases(c) {
        if !cfg.is_dir() && !data.is_dir() {
            continue;
        }
        d.present = true;
        d.config_dirs.push(cfg.clone());
        let defaults = ctx::read(&cfg.join("runners/wine.yml"))
            .map(|s| flatten_yaml(&s))
            .unwrap_or_default();
        // Game configs live in <config>/games (older) or <data>/games (newer releases).
        for gd in [cfg.join("games"), data.join("games")] {
            for f in ctx::files(&gd) {
                if f.extension().is_some_and(|e| e == "yml") {
                    if let Some(src) = ctx::read(&f) {
                        let g = parse_game(&f, &src, &defaults);
                        if let Some(p) = &g.prefix {
                            d.search_roots.push(p.clone());
                        }
                        games.push(g);
                    }
                }
            }
        }
        for w in ctx::subdirs(&data.join("runners/wine")) {
            d.runners.push(Runner {
                name: w.file_name().unwrap().to_string_lossy().into_owned(),
                path: w,
                version: None,
            });
        }
        d.runners.extend(proton_tools(&data.join("runners/proton")));
        let rt = data.join("runtime/battleye_runtime");
        if d.be_runtime.is_none() {
            d.be_runtime = Some(rt);
        }
    }
    if d.present {
        d.notes.push("Lutris runs GE-Proton through umu; \"GE-Proton (Latest)\" resolves to the newest build in Steam's compatibilitytools.d".into());
    }
    d.search_roots.sort();
    d.search_roots.dedup();
    (d, games)
}

#[cfg(test)]
mod tests {
    use super::*;

    const YML: &str = "game:\n  exe: /home/u/Games/rockstar-games-launcher/drive_c/Program Files/Rockstar Games/Launcher/Launcher.exe\n  prefix: /home/u/Games/rockstar-games-launcher\nsystem:\n  env:\n    PROTON_BATTLEYE_RUNTIME: '/home/u/.local/share/Steam/steamapps/common/Proton BattlEye Runtime'\nwine:\n  battleye: true\n  eac: false\n  version: ge-proton\n";

    #[test]
    fn parses_lutris_game_yaml() {
        let g = parse_game(Path::new("x.yml"), YML, &BTreeMap::new());
        assert_eq!(
            g.prefix.as_deref(),
            Some(Path::new("/home/u/Games/rockstar-games-launcher"))
        );
        assert!(g.battleye && !g.eac);
        assert_eq!(g.wine_version.as_deref(), Some("ge-proton"));
        assert_eq!(
            g.env["PROTON_BATTLEYE_RUNTIME"],
            "/home/u/.local/share/Steam/steamapps/common/Proton BattlEye Runtime"
        );
    }

    #[test]
    fn anticheat_toggles_default_on_like_lutris() {
        let g = parse_game(
            Path::new("x.yml"),
            "game:\n  prefix: /p\n",
            &BTreeMap::new(),
        );
        assert!(g.battleye && g.eac);
    }

    #[test]
    fn runner_level_config_is_a_fallback_only() {
        let defaults = flatten_yaml("wine:\n  battleye: false\n  version: wine-ge-8-26\n");
        let g = parse_game(
            Path::new("x.yml"),
            "wine:\n  version: ge-proton\n",
            &defaults,
        );
        assert!(!g.battleye);
        assert_eq!(g.wine_version.as_deref(), Some("ge-proton"));
    }
}
