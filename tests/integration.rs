//! End-to-end tests: run the real binary against fixture trees that mirror
//! real launcher layouts. No network, no real system files, no proprietary bits.
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static N: AtomicU32 = AtomicU32::new(0);

struct Fx {
    base: PathBuf,
    root: PathBuf,
    home: PathBuf,
}

impl Drop for Fx {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base);
    }
}

fn w(p: &Path, s: &str) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, s).unwrap();
}

const HOSTS_STOCK: &str =
    "# Static table lookup for hostnames.\n127.0.0.1 localhost\n::1 localhost\n";
const HOSTS_WORKAROUND: &str = "# Static table lookup for hostnames.\n127.0.0.1 localhost\n::1 localhost\n0.0.0.0 test-s1.battleye.com\n0.0.0.0 paradiseenhanced-s1.battleye.com\n";
const META: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!--GTAOnlineLinux-->\n<CDataFileMgr__ContentsOfDataFileXml/>\n<!--GTAOnlineLinux-->\n";
const GAME_REL: &str = "Games/rockstar-games-launcher/drive_c/Program Files/Rockstar Games/Grand Theft Auto V Enhanced";

fn base_system() -> Fx {
    let base = std::env::temp_dir().join(format!(
        "gtaol-it-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    let root = base.join("root");
    let home = base.join("home/aarav");
    fs::create_dir_all(&home).unwrap();
    w(
        &root.join("etc/os-release"),
        "NAME=\"Arch Linux\"\nPRETTY_NAME=\"Arch Linux\"\nID=arch\n",
    );
    w(
        &root.join("proc/sys/kernel/osrelease"),
        "6.17.7-zen1-1-zen\n",
    );
    w(&root.join("etc/hosts"), HOSTS_STOCK);
    let gpu = root.join("sys/bus/pci/devices/0000:2b:00.0");
    w(&gpu.join("class"), "0x030000\n");
    w(&gpu.join("vendor"), "0x1002\n");
    w(&gpu.join("device"), "0x7550\n");
    w(&gpu.join("subsystem_vendor"), "0x1849\n");
    w(&gpu.join("subsystem_device"), "0x5417\n");
    fs::create_dir_all(root.join("drivers/amdgpu")).unwrap();
    std::os::unix::fs::symlink(root.join("drivers/amdgpu"), gpu.join("driver")).unwrap();
    w(
        &root.join("usr/share/vulkan/icd.d/radeon_icd.x86_64.json"),
        "{}",
    );
    Fx { base, root, home }
}

/// Lutris + Rockstar Games Launcher prefix + Steam-installed BattlEye runtime.
fn lutris_setup(fx: &Fx, workaround: bool) {
    let h = &fx.home;
    let game = h.join(GAME_REL);
    w(&game.join("GTA5_Enhanced.exe"), "MZ");
    w(&game.join("BattlEye/BEClient_x64.dll"), "MZ");
    fs::create_dir_all(game.join("x64/data/errorcodes")).unwrap();
    fs::create_dir_all(game.join("update/x64/data")).unwrap();
    let steam = h.join(".local/share/Steam");
    w(
        &steam.join("steamapps/libraryfolders.vdf"),
        &format!(
            "\"libraryfolders\"\n{{\n\t\"0\"\n\t{{\n\t\t\"path\"\t\t\"{}\"\n\t}}\n}}\n",
            steam.display()
        ),
    );
    w(
        &steam.join(
            "steamapps/common/Proton BattlEye Runtime/v1/lib64/wine/x86_64-unix/beclient_x64.so",
        ),
        "ELF",
    );
    w(
        &steam.join("steamapps/common/Proton BattlEye Runtime/v1/lib/wine/i386-unix/beclient.so"),
        "ELF",
    );
    w(
        &steam.join("compatibilitytools.d/GE-Proton10-25/proton"),
        "#!/usr/bin/env python3\n",
    );
    w(
        &steam.join("compatibilitytools.d/GE-Proton10-25/version"),
        "1762000000 GE-Proton10-25\n",
    );
    let env = if workaround {
        format!("system:\n  env:\n    PROTON_BATTLEYE_RUNTIME: {}/steamapps/common/Proton BattlEye Runtime\n", steam.display())
    } else {
        String::new()
    };
    w(
        &h.join(".local/share/lutris/games/rockstar-games-launcher-1700000000.yml"),
        &format!("game:\n  exe: drive_c/Program Files/Rockstar Games/Launcher/Launcher.exe\n  prefix: {}/Games/rockstar-games-launcher\n{}wine:\n  battleye: true\n  eac: true\n  version: ge-proton\n", h.display(), env),
    );
    if workaround {
        w(&game.join("x64/data/startup.meta"), META);
        w(&fx.root.join("etc/hosts"), HOSTS_WORKAROUND);
    }
}

fn run(fx: &Fx, args: &[&str]) -> (i32, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_gtaol-doctor"))
        .args(args)
        .arg("--root")
        .arg(&fx.root)
        .arg("--home")
        .arg(&fx.home)
        .arg("--no-commands")
        .env_clear()
        .env("XDG_SESSION_TYPE", "wayland")
        .env("XDG_CURRENT_DESKTOP", "KDE")
        .output()
        .unwrap();
    (
        out.status.code().unwrap(),
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr),
    )
}

fn has(out: &str, id: &str, status: &str) -> bool {
    out.lines().any(|l| {
        l.contains(&format!("\"id\": \"{id}\"")) && l.contains(&format!("\"status\": \"{status}\""))
    })
}

#[test]
fn workaround_setup_is_reported_as_conflicts_and_vendor_block() {
    let fx = base_system();
    lutris_setup(&fx, true);
    let (code, out) = run(&fx, &["diagnose", "--json"]);
    assert_eq!(code, 2, "{out}");
    assert!(has(&out, "HOSTS", "CONFLICT"), "{out}");
    assert!(has(&out, "GAME-STARTUP-META", "CONFLICT"), "{out}");
    assert!(has(&out, "LUTRIS-ENV", "WARN"), "{out}");
    assert!(has(&out, "TITLE-SUPPORT", "BLOCKED"), "{out}");
    assert!(has(&out, "BE-RUNTIME", "OK"), "{out}");
    assert!(
        has(&out, "SYS-GPU", "OK") && out.contains("1002:7550") && out.contains("driver=amdgpu"),
        "{out}"
    );
    assert!(
        out.contains(
            "\"level\": 1, \"name\": \"Local environment validated\", \"state\": \"VERIFIED\""
        ),
        "{out}"
    );
    for n in [4, 6, 7, 8, 9] {
        assert!(
            out.lines()
                .any(|l| l.contains(&format!("\"level\": {n},")) && l.contains("BLOCKED (vendor)")),
            "level {n}: {out}"
        );
    }
}

#[test]
fn report_is_sanitized_by_default() {
    let fx = base_system();
    lutris_setup(&fx, true);
    let (_, out) = run(&fx, &["diagnose"]);
    assert!(!out.contains("aarav"), "user name leaked:\n{out}");
    assert!(out.contains("$HOME/Games/rockstar-games-launcher"));
    let (_, raw) = run(&fx, &["diagnose", "--no-sanitize"]);
    assert!(raw.contains("aarav"));
}

#[test]
fn clean_setup_has_no_conflicts_but_stays_blocked() {
    let fx = base_system();
    lutris_setup(&fx, false);
    let (code, out) = run(&fx, &["diagnose", "--json"]);
    assert_eq!(code, 0, "{out}");
    assert!(!out.contains("\"status\": \"CONFLICT\""), "{out}");
    assert!(
        has(&out, "TITLE-SUPPORT", "BLOCKED"),
        "a clean machine must still not be reported as supported:\n{out}"
    );
    assert!(has(&out, "HOSTS", "OK") && has(&out, "GAME-STARTUP-META", "OK"));
}

#[test]
fn empty_machine_reports_unknown_not_ok() {
    let fx = base_system();
    let (code, out) = run(&fx, &["diagnose", "--json"]);
    assert_eq!(code, 0);
    assert!(has(&out, "GAME", "UNKNOWN"), "{out}");
    assert!(
        out.contains(
            "\"level\": 1, \"name\": \"Local environment validated\", \"state\": \"NOT VERIFIED\""
        ),
        "{out}"
    );
}

#[test]
fn steam_library_install_is_found_and_labelled() {
    let fx = base_system();
    let steam = fx.home.join(".local/share/Steam");
    let lib = fx.base.join("mnt/games/SteamLibrary");
    w(
        &steam.join("steamapps/libraryfolders.vdf"),
        &format!(
            "\"libraryfolders\"\n{{\n\t\"1\"\n\t{{\n\t\t\"path\"\t\t\"{}\"\n\t}}\n}}\n",
            lib.display()
        ),
    );
    w(
        &lib.join("steamapps/common/Grand Theft Auto V Enhanced/PlayGTAV.exe"),
        "MZ",
    );
    w(
        &lib.join("steamapps/common/Proton 10.0/proton"),
        "#!/usr/bin/env python3\n",
    );
    let (_, out) = run(&fx, &["diagnose"]);
    assert!(out.contains("distribution: Steam library"), "{out}");
    assert!(out.contains("runner Proton 10.0"), "{out}");
}

#[test]
fn explicit_game_dir_is_honoured() {
    let fx = base_system();
    let dir = fx.base.join("elsewhere/Grand Theft Auto V Enhanced");
    w(&dir.join("GTA5_Enhanced.exe"), "MZ");
    let (_, out) = run(
        &fx,
        &["diagnose", "--json", "--game-dir", dir.to_str().unwrap()],
    );
    assert!(has(&out, "GAME", "OK"), "{out}");
}

#[test]
fn hosts_clean_is_dry_run_by_default_then_applies_and_rolls_back() {
    let fx = base_system();
    lutris_setup(&fx, true);
    let hosts = fx.root.join("etc/hosts");

    let (code, out) = run(&fx, &["hosts", "clean"]);
    assert_eq!(code, 2);
    assert!(
        out.contains("-0.0.0.0 test-s1.battleye.com") && out.contains("dry run"),
        "{out}"
    );
    assert_eq!(
        fs::read_to_string(&hosts).unwrap(),
        HOSTS_WORKAROUND,
        "dry run must not write"
    );

    let (code, out) = run(&fx, &["hosts", "clean", "--apply"]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(fs::read_to_string(&hosts).unwrap(), HOSTS_STOCK);

    let (code, _) = run(&fx, &["hosts", "check"]);
    assert_eq!(code, 0);
    let (code, out) = run(&fx, &["hosts", "clean", "--apply"]);
    assert_eq!(code, 0);
    assert!(
        out.contains("Nothing to do"),
        "second apply must be a no-op: {out}"
    );

    let (code, _) = run(&fx, &["hosts", "rollback"]);
    assert_eq!(code, 0);
    assert_eq!(
        fs::read_to_string(&hosts).unwrap(),
        HOSTS_STOCK,
        "rollback is a dry run without --apply"
    );
    let (code, out) = run(&fx, &["hosts", "rollback", "--apply"]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(
        fs::read_to_string(&hosts).unwrap(),
        HOSTS_WORKAROUND,
        "rollback must restore byte-for-byte"
    );
}

#[test]
fn hosts_rollback_without_backup_is_an_error() {
    let fx = base_system();
    let (code, out) = run(&fx, &["hosts", "rollback", "--apply"]);
    assert_eq!(code, 1);
    assert!(out.contains("no gtaol-doctor backup"), "{out}");
}

#[test]
fn startup_meta_quarantine_and_restore_roundtrip() {
    let fx = base_system();
    lutris_setup(&fx, true);
    let meta = fx.home.join(GAME_REL).join("x64/data/startup.meta");
    let m = meta.to_str().unwrap();

    let (code, _) = run(&fx, &["startup-meta", "check", "--file", m]);
    assert_eq!(code, 2);
    let (_, out) = run(&fx, &["startup-meta", "quarantine", "--file", m]);
    assert!(out.contains("dry run") && meta.exists());

    let (code, out) = run(&fx, &["startup-meta", "quarantine", "--file", m, "--apply"]);
    assert_eq!(code, 0, "{out}");
    assert!(!meta.exists());
    assert!(
        fx.home.join(GAME_REL).join("x64/data/errorcodes").is_dir(),
        "siblings untouched"
    );
    let (code, out) = run(&fx, &["diagnose", "--json"]);
    assert_eq!(code, 2, "hosts conflict remains");
    assert!(
        has(&out, "GAME-STARTUP-META", "OK") && !has(&out, "GAME-STARTUP-META", "CONFLICT"),
        "{out}"
    );

    let (code, out) = run(&fx, &["startup-meta", "restore", "--file", m, "--apply"]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(fs::read_to_string(&meta).unwrap(), META);
}

#[test]
fn startup_meta_quarantine_refuses_files_it_does_not_recognise() {
    let fx = base_system();
    let f = fx.base.join("startup.meta");
    w(&f, "<?xml version=\"1.0\"?><stock/>");
    let (code, out) = run(
        &fx,
        &[
            "startup-meta",
            "quarantine",
            "--file",
            f.to_str().unwrap(),
            "--apply",
        ],
    );
    assert_eq!(code, 1);
    assert!(out.contains("refusing") && f.exists(), "{out}");
}

#[test]
fn unknown_option_fails_loudly() {
    let fx = base_system();
    let (code, out) = run(&fx, &["diagnose", "--fix-everything"]);
    assert_eq!(code, 1);
    assert!(out.contains("unknown option"));
}
