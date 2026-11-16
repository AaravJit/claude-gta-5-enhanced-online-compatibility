# Architecture

## Principles

1. **One engine, thin adapters.** `diagnose` works on a launcher-neutral model (`launchers::Detected`: runners,
   BattlEye runtime path, search roots). Each launcher is ~50 lines that read its documented on-disk config.
2. **Read-only by default.** Only `hosts::apply` and `game::quarantine_startup_meta` write. Both are dry-run unless
   `--apply`, both back up first, both have an inverse command, both only *remove* workaround state.
3. **Unknown is never OK.** Six statuses: `OK, INFO, WARN, CONFLICT, BLOCKED, UNKNOWN`. Vendor enablement comes
   from dated, sourced data (`data/titles.conf`), never from local observation.
4. **Testable without the real system.** All paths go through `Ctx { root, home, env }`; tests run the real binary
   against fixture trees. No proprietary file is needed or mocked as "working".
5. **Auditable.** Zero third-party crates; std only. Rust was chosen for a single static-ish binary, memory safety
   in a tool that may run under sudo, and trivial packaging on every distro; Python would add a runtime dependency
   and Bash would not be testable to this standard.

## What a vendor-authorised integration looks like (Component D)

```
game.exe (Windows PE, unmodified)
  └─ loads BEClient_x64.dll ──► Wine resolves it as BUILTIN (override "b,n")
                                 └─ <PROTON_BATTLEYE_RUNTIME>/v1/lib64/wine/…  (BattlEye's Linux module)
                                      └─ talks to BattlEye's backend as a *Proton* client, honestly identified
BattlEye backend: is Proton allowed for this game ID?  ── yes ─► client validated ─► game servers accept
                                                         └ no ─► kick / "BattlEye is required"
```

| Piece | Owner |
|---|---|
| Linux BEClient module, its checks, its protocol, its updates | BattlEye (closed, must stay vendor-controlled) |
| Per-title Proton allow flag | BattlEye, at the publisher's request |
| Decision to request it; server/session policy | Rockstar |
| DLL search path + override plumbing | Valve Wine/Proton, GE-Proton (already upstream) |
| Exporting `PROTON_BATTLEYE_RUNTIME`, downloading the runtime | Steam, Lutris, Heroic (done); Bottles (missing) |
| Diagnostics, launcher parity, clean-state verification, enablement database | this project |

Launcher portability adds one real requirement for the vendor: enablement must not depend on Steam being the
launcher. For other opt-in titles the runtime works from Lutris/Heroic, so there is precedent, but Rockstar would be
supporting RGL-under-Wine, which it does not officially support today.

## Roadmap for the code

- `titles.conf` grows to other BattlEye/EAC titles; the engine is not GTA-specific.
- `--json` schema freeze, then Lutris/Heroic can call the tool and surface "not enabled by publisher" in their UI.
- `winprobe` grows into a conformance suite for Wine service-control behaviour (self-owned programs only).
