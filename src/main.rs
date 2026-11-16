use gtaol_doctor::ctx::{self, Ctx};
use gtaol_doctor::{diagnose, game, hosts, report::Status};
use std::path::PathBuf;
use std::process::ExitCode;

const HELP: &str = "\
gtaol-doctor - GTA V Enhanced / BattlEye on Linux diagnostics (not an anti-cheat bypass)

USAGE:
  gtaol-doctor diagnose [--json] [--no-sanitize] [--no-commands] [--game-dir DIR]...
  gtaol-doctor hosts check|clean|rollback [--file FILE] [--apply]
  gtaol-doctor startup-meta check|quarantine|restore --file FILE [--apply]
  gtaol-doctor --version | --help

GLOBAL OPTIONS (mainly for testing against a fixture tree):
  --root DIR   treat DIR as / for /etc, /sys, /proc, /usr lookups
  --home DIR   treat DIR as the home directory

Every modifying subcommand is a dry run unless --apply is given, and every
applied change is backed up and reversible (hosts rollback / startup-meta restore).

EXIT CODES: 0 no local conflicts, 2 local conflicts found, 1 error.
A vendor block is reported in the output, not through the exit code.
";

struct Args {
    pos: Vec<String>,
    json: bool,
    sanitize: bool,
    apply: bool,
    file: Option<PathBuf>,
}

fn parse(c: &mut Ctx) -> Result<Args, String> {
    let mut a = Args {
        pos: vec![],
        json: false,
        sanitize: true,
        apply: false,
        file: None,
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        let mut val = |name: &str| {
            it.next()
                .map(PathBuf::from)
                .ok_or_else(|| format!("{name} needs a value"))
        };
        match arg.as_str() {
            "--help" | "-h" => a.pos = vec!["help".into()],
            "--version" | "-V" => a.pos = vec!["version".into()],
            "--json" => a.json = true,
            "--no-sanitize" => a.sanitize = false,
            "--no-commands" => c.run_commands = false,
            "--apply" => a.apply = true,
            "--file" => a.file = Some(val("--file")?),
            "--root" => c.root = val("--root")?,
            "--home" => c.home = val("--home")?,
            "--game-dir" => c.game_dirs.push(val("--game-dir")?),
            s if s.starts_with("--") => return Err(format!("unknown option {s}")),
            _ => a.pos.push(arg),
        }
    }
    Ok(a)
}

fn run() -> Result<u8, String> {
    let mut c = Ctx::from_process();
    let a = parse(&mut c)?;
    let pos: Vec<&str> = a.pos.iter().map(|s| s.as_str()).collect();
    match pos.as_slice() {
        [] | ["help", ..] => {
            print!("{HELP}");
            Ok(0)
        }
        ["version", ..] => {
            println!("gtaol-doctor {}", env!("CARGO_PKG_VERSION"));
            Ok(0)
        }
        ["diagnose"] => {
            let mut r = diagnose::run(&c);
            if a.sanitize {
                r = r.sanitized(&c.home.to_string_lossy());
            }
            print!("{}", if a.json { r.to_json() } else { r.to_text() });
            Ok(if r.has(Status::Conflict) { 2 } else { 0 })
        }
        ["hosts", action] => {
            let file = a.file.clone().unwrap_or_else(|| c.sys("/etc/hosts"));
            match *action {
                "check" | "clean" => {
                    let before = std::fs::read_to_string(&file)
                        .map_err(|e| format!("cannot read {}: {e}", file.display()))?;
                    let p = hosts::plan(&before);
                    if p.overrides.is_empty() {
                        println!(
                            "{}: no BattlEye hostname overrides. Nothing to do.",
                            file.display()
                        );
                        return Ok(0);
                    }
                    println!(
                        "{}: {} BattlEye hostname override(s):",
                        file.display(),
                        p.overrides.len()
                    );
                    print!("{}", hosts::diff(&before, &p.cleaned));
                    if *action == "check" {
                        return Ok(2);
                    }
                    if !a.apply {
                        println!("dry run: nothing written. Re-run with --apply (needs write access; for /etc/hosts use sudo).");
                        return Ok(2);
                    }
                    let backup =
                        hosts::apply(&file, ctx::now_secs())?.expect("overrides were present");
                    println!(
                        "applied. backup: {}\nundo with: gtaol-doctor hosts rollback --file {}",
                        backup.display(),
                        file.display()
                    );
                    Ok(0)
                }
                "rollback" => {
                    let b = hosts::latest_backup(&file).ok_or_else(|| {
                        format!("no gtaol-doctor backup found next to {}", file.display())
                    })?;
                    if !a.apply {
                        println!(
                            "dry run: would restore {} from {}. Re-run with --apply.",
                            file.display(),
                            b.display()
                        );
                        return Ok(0);
                    }
                    let b = hosts::rollback(&file)?;
                    println!("restored {} from {}", file.display(), b.display());
                    Ok(0)
                }
                other => Err(format!("unknown hosts action {other}")),
            }
        }
        ["startup-meta", action] => {
            let file = a
                .file
                .clone()
                .ok_or("startup-meta needs --file <path to startup.meta> (diagnose prints it)")?;
            match *action {
                "check" => {
                    let st = game::meta_state(&file);
                    println!("{}: {:?}", file.display(), st);
                    Ok(if st == game::MetaState::Workaround {
                        2
                    } else {
                        0
                    })
                }
                "quarantine" => {
                    if !a.apply {
                        println!("dry run: state is {:?}; with --apply a workaround file would be moved to {}", game::meta_state(&file), c.state_dir().join("quarantine").display());
                        return Ok(0);
                    }
                    let dest = game::quarantine_startup_meta(&c, &file, ctx::now_secs())?;
                    println!("moved to {}\nundo with: gtaol-doctor startup-meta restore --file \"{}\" --apply", dest.display(), file.display());
                    Ok(0)
                }
                "restore" => {
                    if !a.apply {
                        println!("dry run: would restore the most recent quarantined copy to {}. Re-run with --apply.", file.display());
                        return Ok(0);
                    }
                    let src = game::restore_startup_meta(&c, &file)?;
                    println!("restored {} from {}", file.display(), src.display());
                    Ok(0)
                }
                other => Err(format!("unknown startup-meta action {other}")),
            }
        }
        _ => Err(format!("unknown command: {}\n\n{HELP}", pos.join(" "))),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}
