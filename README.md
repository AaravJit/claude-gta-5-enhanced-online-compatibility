# gtaol-doctor

Diagnostics and reversible cleanup for **GTA V Enhanced + BattlEye on Linux** (Steam/Proton, Lutris, Heroic, Bottles).

**This is not an anti-cheat bypass and does not make GTA Online work.** As of 2026-11-16 Rockstar has not enabled
BattlEye's Proton support for GTA V, and nothing that runs on your machine can change that
([docs/RESEARCH.md](docs/RESEARCH.md)). What this project does instead:

- tells you, truthfully, what state your machine is in (`diagnose`), with no fabricated green ticks:
  a perfectly configured machine is still reported as `BLOCKED (vendor)`;
- detects and reversibly removes leftovers of the unofficial "block BattlEye" workaround
  (`hosts clean`, `startup-meta quarantine`), so the machine is ready for the day the title is enabled;
- provides one launcher-neutral engine with thin adapters, a title-enablement database
  ([data/titles.conf](data/titles.conf)) and a vendor-facing proposal ([docs/VENDOR-PROPOSAL.md](docs/VENDOR-PROPOSAL.md)).

## Build (Arch Linux)

```bash
sudo pacman -S --needed rust git
```

```bash
cargo build --release --locked
```

```bash
cargo test --locked
```

The binary is `target/release/gtaol-doctor`. It has zero third-party crate dependencies and links only glibc.
Other distros: `dnf install cargo` / `apt install cargo` (Rust 1.74+). An Arch `PKGBUILD` is in `packaging/arch/`.

## Use

```bash
./target/release/gtaol-doctor diagnose
```

`diagnose` only reads. Output has home directory and user name replaced by `$HOME`; add `--json` for machine output,
`--game-dir DIR` to point at an install outside the detected launchers.

```bash
./target/release/gtaol-doctor hosts clean
```

Dry run: prints the lines that would be removed from `/etc/hosts`. To apply (creates `/etc/hosts.gtaol-doctor.<time>.bak` first):

```bash
sudo ./target/release/gtaol-doctor hosts clean --apply
```

Undo: `sudo ./target/release/gtaol-doctor hosts rollback --apply`.
`startup-meta quarantine --file <path> --apply` moves a workaround `startup.meta` (only one carrying the
`<!--GTAOnlineLinux-->` marker) into `~/.local/state/gtaol-doctor/quarantine/`; `startup-meta restore` puts it back.

Exit codes: `0` no local conflicts, `2` local conflicts found, `1` error.

## Layout

| Path | What |
|---|---|
| `src/diagnose.rs` | orchestration, acceptance levels L0–L9 |
| `src/launchers/` | Steam, Lutris, Heroic, Bottles adapters (read-only) |
| `src/hosts.rs`, `src/game.rs` | the only code that writes; dry-run by default, backed up, reversible |
| `src/beruntime.rs` | Proton BattlEye Runtime layout check |
| `data/titles.conf` | per-title vendor enablement (data, sourced, dated) |
| `tests/integration.rs` | runs the real binary against fixture trees |
| `winprobe/` | self-owned read-only Windows API probe (built and run under Wine in CI) |
| `scripts/collect-baseline.sh` | read-only system baseline for bug reports |
| `docs/` | research, architecture, vendor proposal |

## Uninstall

Delete the binary and `~/.local/state/gtaol-doctor/`. Backups it created are named `hosts.gtaol-doctor.*.bak`.

License: MIT.
