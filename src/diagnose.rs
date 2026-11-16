//! Orchestration: gather facts, derive findings, evaluate acceptance levels.
use crate::ctx::{self, Ctx};
use crate::game::{self, MetaState};
use crate::launchers::{self, lutris};
use crate::report::{Finding, Level, LevelState, Report, Status};
use crate::{beruntime, hosts, system, titles};
use std::path::PathBuf;

fn runtime_finding(label: &str, dir: &std::path::Path) -> Finding {
    match beruntime::inspect(dir) {
        None => Finding::new("BE-RUNTIME", Status::Info, format!("{label}: not installed at {}", dir.display())),
        Some(rt) if rt.usable() => Finding::new("BE-RUNTIME", Status::Ok, format!("{label}: present at {}", dir.display()))
            .detail(format!("layout v1/lib/wine={} v1/lib64/wine={} modules={}", rt.has_lib32, rt.has_lib64, rt.modules.len()))
            .detail("this is Valve's Linux-side userspace BEClient (loaded as a Wine builtin), not a Windows kernel driver")
            .detail("presence only makes the runtime *available*; BattlEye's service decides per title whether it is accepted"),
        Some(rt) => Finding::new("BE-RUNTIME", Status::Warn, format!("{label}: directory exists but layout is unexpected: {}", dir.display()))
            .detail(format!("v1/lib/wine={} v1/lib64/wine={} modules={:?}", rt.has_lib32, rt.has_lib64, rt.modules))
            .fix("verify/reinstall \"Proton BattlEye Runtime\" (Steam app 1161040) from Steam > Library > Tools"),
    }
}

pub fn run(c: &Ctx) -> Report {
    let title = titles::gta5_enhanced();
    let mut f: Vec<Finding> = system::findings(c);

    // ---- launchers and runners -------------------------------------------------
    let (lutris_det, lutris_games) = lutris::detect(c);
    let dets = vec![
        launchers::steam::detect(c),
        lutris_det,
        launchers::heroic::detect(c),
        launchers::bottles::detect(c),
    ];
    let mut roots: Vec<PathBuf> = vec![];
    let mut runner_count = 0;
    for d in &dets {
        if !d.present {
            f.push(Finding::new(
                "LAUNCHER",
                Status::Info,
                format!("{}: not detected", d.launcher),
            ));
            continue;
        }
        let mut x = Finding::new(
            "LAUNCHER",
            Status::Ok,
            format!(
                "{}: detected ({} runner(s) found)",
                d.launcher,
                d.runners.len()
            ),
        );
        for r in &d.runners {
            runner_count += 1;
            x = x.detail(format!(
                "runner {} {} at {}",
                r.name,
                r.version
                    .as_deref()
                    .map(|v| format!("[{v}]"))
                    .unwrap_or_default(),
                r.path.display()
            ));
        }
        for n in &d.notes {
            x = x.detail(n.clone());
        }
        f.push(x);
        roots.extend(d.search_roots.iter().cloned());
        if let Some(rt) = &d.be_runtime {
            f.push(runtime_finding(
                &format!("{} BattlEye runtime", d.launcher),
                rt,
            ));
        }
    }
    if let Some(v) = c.env.get("PROTON_BATTLEYE_RUNTIME") {
        f.push(runtime_finding(
            "PROTON_BATTLEYE_RUNTIME from current environment",
            std::path::Path::new(v),
        ));
    }

    // ---- game installs ---------------------------------------------------------
    let installs = game::find_installs(c, &roots, &title);
    if installs.is_empty() {
        f.push(Finding::new("GAME", Status::Unknown, format!("{}: no install found in launcher prefixes/libraries", title.name))
            .fix("pass the install or prefix directory explicitly: gtaol-doctor diagnose --game-dir <DIR>"));
    }
    for inst in &installs {
        let via_steam = inst.path.to_string_lossy().contains("/steamapps/common/");
        let mut x = Finding::new(
            "GAME",
            Status::Ok,
            format!("{} at {}", title.name, inst.path.display()),
        )
        .detail(format!(
            "distribution: {}",
            if via_steam {
                "Steam library"
            } else {
                "non-Steam (Rockstar Games Launcher / Epic) inside a Wine prefix"
            }
        ))
        .detail(format!(
            "executables present: {}",
            if inst.exes.is_empty() {
                "none of the expected names".into()
            } else {
                inst.exes.join(", ")
            }
        ))
        .detail(format!(
            "BattlEye/ directory files: {}",
            if inst.battleye_files.is_empty() {
                "none found".into()
            } else {
                inst.battleye_files.join(", ")
            }
        ));
        if inst.exes.is_empty() {
            x.status = Status::Warn;
        }
        f.push(x);
        for (rel, st) in &inst.startup_meta {
            let p = inst.path.join(rel);
            match st {
                MetaState::Absent => f.push(Finding::new("GAME-STARTUP-META", Status::Ok, format!("{rel}: not present (stock state)"))),
                MetaState::Workaround => f.push(
                    Finding::new("GAME-STARTUP-META", Status::Conflict, format!("{rel}: GTAOnlineLinux workaround file present"))
                        .detail(p.display().to_string())
                        .detail("this overrides the game's shipped startup data; a vendor-supported configuration runs unmodified game data")
                        .fix(format!("gtaol-doctor startup-meta quarantine --file \"{}\" --apply   (reversible: startup-meta restore)", p.display())),
                ),
                MetaState::Other => f.push(
                    Finding::new("GAME-STARTUP-META", Status::Warn, format!("{rel}: present, not recognised as the workaround file"))
                        .detail(p.display().to_string())
                        .detail("left alone; if you did not create it, verify game files from the Rockstar Games Launcher"),
                ),
            }
        }
    }

    // ---- Lutris per-game configuration ------------------------------------------
    let lutris_rt = dets
        .iter()
        .find(|d| d.launcher == "Lutris")
        .and_then(|d| d.be_runtime.clone());
    for g in &lutris_games {
        let owns_game = g
            .prefix
            .as_ref()
            .is_some_and(|p| installs.iter().any(|i| i.path.starts_with(p)));
        if !owns_game {
            continue;
        }
        f.push(
            Finding::new(
                "LUTRIS-GAME",
                Status::Info,
                format!("config {}", g.file.display()),
            )
            .detail(format!(
                "wine version={} battleye={} eac={}",
                g.wine_version.as_deref().unwrap_or("(runner default)"),
                g.battleye,
                g.eac
            ))
            .detail(format!(
                "prefix={}",
                g.prefix
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default()
            )),
        );
        if let Some(user_rt) = g.env.get("PROTON_BATTLEYE_RUNTIME") {
            let mut x = Finding::new(
                "LUTRIS-ENV",
                Status::Warn,
                "PROTON_BATTLEYE_RUNTIME is set by hand in Lutris environment variables",
            )
            .detail(format!("value: {user_rt}"));
            if g.battleye {
                x = x.detail(format!(
                    "with \"Enable BattlEye Anti-Cheat\" on, Lutris sets this variable itself to {} after merging user variables, so the manual value is overridden (per lutris/runners/wine.py on master; confirm in the Lutris launch log of your version)",
                    lutris_rt.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "<lutris runtime>/battleye_runtime".into())
                ));
            }
            f.push(x.fix(
                "remove the manual variable; let the Lutris toggle own it (one source of truth)",
            ));
        }
        if g.eac {
            f.push(
                Finding::new(
                    "LUTRIS-EAC",
                    Status::Info,
                    "Easy Anti-Cheat runtime toggle is on; GTA V does not use EAC",
                )
                .detail("harmless (only adds a DLL search path); may be turned off for clarity"),
            );
        }
        for k in ["SteamDeck", "SteamOS"] {
            if g.env.get(k).is_some_and(|v| v == "1") {
                f.push(Finding::new("LUTRIS-ENV", Status::Conflict, format!("{k}=1 is set for this game"))
                    .detail("community reports in ValveSoftware/Proton#8495: when told it runs on Steam Deck the game starts with BattlEye disabled (Story Mode only)")
                    .fix(format!("remove {k} from the Lutris environment variables")));
            }
        }
    }

    // ---- hosts -----------------------------------------------------------------
    let hosts_path = c.sys("/etc/hosts");
    match ctx::read(&hosts_path) {
        None => f.push(Finding::new(
            "HOSTS",
            Status::Unknown,
            "could not read /etc/hosts",
        )),
        Some(s) => {
            let p = hosts::plan(&s);
            if p.overrides.is_empty() {
                f.push(Finding::new(
                    "HOSTS",
                    Status::Ok,
                    "no BattlEye hostname overrides in /etc/hosts",
                ));
            } else {
                let mut x = Finding::new(
                    "HOSTS",
                    Status::Conflict,
                    format!(
                        "{} BattlEye hostname override(s) in /etc/hosts",
                        p.overrides.len()
                    ),
                );
                for o in &p.overrides {
                    x = x.detail(format!("line {}: {} {}", o.line_no, o.address, o.host));
                }
                f.push(
                    x.detail("these stop the BattlEye client from reaching its service: the client cannot be validated, which is the opposite of the goal")
                        .fix("gtaol-doctor hosts clean   (dry run), then: sudo gtaol-doctor hosts clean --apply   (reversible: hosts rollback)"),
                );
            }
        }
    }

    // ---- vendor enablement -----------------------------------------------------
    let enabled = title.proton_enablement == "enabled";
    let mut x = if enabled {
        Finding::new(
            "TITLE-SUPPORT",
            Status::Ok,
            format!(
                "{}: vendor has enabled {} for Proton (database as of {})",
                title.name, title.anticheat, title.as_of
            ),
        )
    } else if title.proton_enablement == "unknown" {
        Finding::new(
            "TITLE-SUPPORT",
            Status::Unknown,
            format!("{}: vendor enablement unknown", title.name),
        )
    } else {
        Finding::new("TITLE-SUPPORT", Status::Blocked, format!("{}: {} is NOT enabled for Linux/Proton by the publisher (database as of {})", title.name, title.anticheat, title.as_of))
            .detail("no local configuration, runner, runtime or Wine patch can change this; it is a server-side, per-title decision")
    };
    x = x.detail(format!("evidence: {}", title.evidence));
    for s in &title.sources {
        x = x.detail(format!("source: {s}"));
    }
    f.push(x);

    // ---- levels ----------------------------------------------------------------
    let conflicts = f.iter().filter(|x| x.status == Status::Conflict).count();
    let unknown_sys: Vec<&str> = f
        .iter()
        .filter(|x| x.status == Status::Unknown && (x.id.starts_with("SYS") || x.id == "GAME"))
        .map(|x| x.id)
        .collect();
    let l1 = if !installs.is_empty() && runner_count > 0 && unknown_sys.is_empty() {
        (
            LevelState::Verified,
            format!(
                "{} install(s), {} runner(s), system facts read",
                installs.len(),
                runner_count
            ),
        )
    } else {
        (
            LevelState::NotVerified,
            format!(
                "missing: {}{}{}",
                if installs.is_empty() {
                    "game install; "
                } else {
                    ""
                },
                if runner_count == 0 { "runner; " } else { "" },
                unknown_sys.join(", ")
            ),
        )
    };
    let vendor = |n: u8, name: &'static str| Level {
        n,
        name,
        state: if enabled {
            LevelState::NeedsThisMachine
        } else {
            LevelState::Blocked
        },
        why: if enabled {
            "vendor enablement recorded; verify by playing".into()
        } else {
            "requires publisher/BattlEye per-title enablement".into()
        },
    };
    let levels = vec![
        Level { n: 0, name: "Research verified", state: LevelState::Verified, why: format!("see docs/RESEARCH.md; title database as of {}", title.as_of) },
        Level { n: 1, name: "Local environment validated", state: l1.0, why: l1.1 },
        Level { n: 2, name: "Compatibility tooling compiled", state: LevelState::Verified, why: "this binary is running".into() },
        Level { n: 3, name: "Game launch verified", state: LevelState::NeedsThisMachine, why: "launch Story Mode; this tool does not start the game".into() },
        vendor(4, "Authorized BattlEye initialization"),
        Level { n: 5, name: "Rockstar authentication (supported config)", state: if enabled { LevelState::NeedsThisMachine } else { LevelState::Blocked }, why: "sign-in itself works under Proton, but no supported online configuration exists to verify it in".into() },
        vendor(6, "Standard private multiplayer"),
        vendor(7, "Standard public multiplayer"),
        vendor(8, "Cross-launcher compatibility"),
        vendor(9, "Cross-distro portability"),
    ];

    let verdict = if enabled {
        format!("Vendor enablement is recorded for this title. {conflicts} local conflict(s) to resolve before testing.")
    } else {
        format!(
            "GTA Online with genuine BattlEye protection is not available on Linux today: the publisher has not enabled it. {} local conflict(s) found{}. Story Mode is unaffected.",
            conflicts,
            if conflicts > 0 { " (leftovers of the unofficial workaround; revert them, see the fix lines)" } else { "" }
        )
    };
    Report {
        findings: f,
        levels,
        verdict,
    }
}
