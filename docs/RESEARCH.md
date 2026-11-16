# Research findings (2026-11-16)

Each claim is tagged: **[read]** primary source read in full during this work, **[summary]** page read through a
summarising fetch, **[recalled]** well-known public statement not re-fetched (host unreachable from the build
environment: `support.rockstargames.com`, `battleye.com`, `partner.steamgames.com`, `gamingonlinux.com`), **[inference]**.

## 1. How BattlEye works under Proton

- **[read]** Valve's Wine fork, `dlls/ntdll/unix/loader.c`, `set_dll_path()` (branches `proton_10.0`, `bleeding-edge`):
  if `PROTON_BATTLEYE_RUNTIME` is set, `<dir>/v1/lib/wine/` and `<dir>/v1/lib64/wine/` are appended to the builtin
  DLL search path (EAC: `<dir>/v2/lib32/`, `/v2/lib64/`).
- **[read]** `proton` script (`ValveSoftware/Proton@proton_10.0`, `Session.__init__`): `"beclient.dll": "b,n"`,
  `"beclient_x64.dll": "b,n"`. GE-Proton's `proton` has the same two overrides.
- **Consequence:** the game's Windows `BEClient_x64.dll` is replaced by a *Wine builtin* shipped in the runtime,
  i.e. a Linux userspace module written by BattlEye. **No Windows kernel driver is involved**, and none could be:
  Wine hosts "drivers" in `winedevice.exe`, an unprivileged Linux process with an emulated `ntoskrnl.exe`. A real
  anti-cheat driver's value comes from ring-0 facilities (object/process callbacks, image-load notification,
  kernel memory visibility). Emulated there, they protect nothing, and reporting them as real would be a false
  attestation.
- **[recalled]** BattlEye, 2021-09-24: Proton/Steam Deck support is "on an opt-in basis with game developers choosing
  whether they want to allow it or not". Valve (Steamworks, Dec 2021): all a developer needs to do is contact
  BattlEye to enable it for their title.
- **Consequence:** enablement is a per-title switch on BattlEye's backend. The runtime is identical for every game;
  what differs is whether BattlEye's service accepts the Proton client for that game ID.

## 2. GTA V Enhanced status

- **[recalled, quoted in search results]** Rockstar BattlEye FAQ (published 2024-09-17): "Steam Deck does not support
  BattlEye for GTA Online. You will be able to play GTAV Story Mode but unable to play GTA Online."
- **[summary]** ValveSoftware/Proton#8495 (GTA V Enhanced, 3240220), labelled "Game compatibility - Unofficial":
  2025-03-04 "kicked after a few minutes due to BattlEye support not being enabled by Rockstar"; the game disables
  BattlEye itself when told it runs on Steam Deck; #9501 (2026-02-15) "Multiplayer kicks Linux/Steam Deck users via
  BattlEye when running under Proton"; latest reports 2026-10-27 and 2026-11-13: removed from Freemode within about
  five minutes on current Proton builds. No Valve or Rockstar statement of enablement exists in the thread.
- I found no 2026 announcement from Rockstar changing this.

## 3. How launchers pass the runtime

- **[read]** Lutris `lutris/runners/wine.py` (master): options `battleye` and `eac` default to `True`;
  `if runner_config.get("battleye"): env["PROTON_BATTLEYE_RUNTIME"] = RUNTIME_DIR/battleye_runtime`. User
  "Environment variables" are merged earlier (`runner.py`: `env.update(system_config.get("env"))`), so a hand-set
  `PROTON_BATTLEYE_RUNTIME` is overwritten while the toggle is on. (Read on master; Lutris 0.5.22 not separately checked.)
- **[read]** Heroic `src/backend/launcher.ts`: `PROTON_BATTLEYE_RUNTIME = <runtimePath>/battleye_runtime` when the
  per-game `battlEyeRuntime` setting is on; downloads the runtime if missing.
- Steam sets the variable itself when the "Proton BattlEye Runtime" tool (app 1161040) is installed. **[recalled]**
- Bottles: no integration found.

## 4. The GTAOnlineLinux workaround

- **[read]** `manual.md`: (a) hosts entries `0.0.0.0 test-s1.battleye.com` and `0.0.0.0 paradiseenhanced-s1.battleye.com`,
  (b) `PROTON_BATTLEYE_RUNTIME` launch option, (c) a replacement `x64/data/startup.meta`.
- (a) prevents the BattlEye client from reaching BattlEye. (c) replaces data the game ships. I did not determine
  what (c) changes inside the engine and make no claim about it.
- Whatever sessions it reaches, it does so with BattlEye *not validating the client*. That is the opposite of this
  project's goal, is outside Rockstar's terms, and puts the account used at risk.

## 5. Dependency graph and blocker classification

```
Linux kernel / Mesa / Vulkan ............ works (Story Mode runs)            not a blocker
Wine / Proton Windows API ............... works for the game                 not a blocker
Proton BattlEye Runtime (userspace) ..... exists, installable                not a blocker
Launcher passes PROTON_BATTLEYE_RUNTIME . Steam/Lutris/Heroic do; Bottles no fixable by us / upstream
BattlEye backend: Proton client allowed
  for GTA V's game ID ................... NOT ENABLED                         requires vendor (Rockstar asks BattlEye)
Rockstar services accept the session .... follows from the line above        requires vendor
Windows BattlEye kernel driver on Linux . impossible by design, and unneeded fundamentally incompatible
```

| Blocker | Class |
|---|---|
| Per-title BattlEye Proton enablement | Requires vendor cooperation |
| Rockstar policy (FAQ) | Requires vendor cooperation |
| Running the Windows kernel driver under Wine | Fundamentally incompatible; also not how supported titles work |
| Consistent runtime wiring across launchers | Fixable by this project / upstream patches |
| Leftover workaround state on user machines | Fixable by this project (done) |
| Rockstar's VM policy for GTA Online | Unknown; not verified |

## 6. Why "more Windows APIs" does not help

Supported BattlEye titles (opt-in ones) run on the same Wine, with the same API surface, on the same machines.
The API layer is therefore demonstrably sufficient. Adding security APIs whose only purpose is to make the
environment look like genuine Windows to an anti-cheat would be misrepresentation, and BattlEye's Proton client
does not ask for it: it is a native Linux module that knows it is on Linux.
