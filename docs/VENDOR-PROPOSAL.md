# Requests to vendors and upstreams

## Rockstar Games

**Ask:** request BattlEye to enable Proton support for GTA V Legacy and Enhanced (BattlEye states this is an opt-in
flag on their side and Valve's Steamworks documentation says no further developer work is required).

**What makes it easier to say yes**

- The Proton BattlEye client is BattlEye's own code, already trusted for other large titles; Rockstar is not asked
  to trust community software or weaken any check.
- A staged rollout is possible: Steam Deck (a fixed, Valve-controlled image) first, desktop Linux later.
- Demand evidence: ValveSoftware/Proton#8495 and its duplicates, ProtonDB reports for 3240220.
- This project can supply support-load reducers: a diagnostic that tells users precisely when a problem is local
  (and removes unofficial workaround state) versus on Rockstar's side.

**What only Rockstar can answer:** whether the userspace Proton client meets their threat model for GTA Online's
peer-to-peer sessions. That, not engineering effort, is the likely reason for the current position, and it is a
legitimate one. Nothing in this repository tries to route around it.

## BattlEye

- Publish (even briefly) what the Proton client covers, so publishers can assess risk without private briefings.
- Document the runtime directory contract (`v1/lib/wine`, `v1/lib64/wine`) so non-Steam launchers have a spec
  rather than reading Wine source.

## Valve / Proton

- A public, machine-readable list of BattlEye/EAC titles with Proton enablement would let every launcher show
  "not enabled by the publisher" instead of users debugging their own machines.
- Document `PROTON_BATTLEYE_RUNTIME` / `PROTON_EAC_RUNTIME` as a stable interface.

## Lutris

- When the BattlEye toggle is on, a user-set `PROTON_BATTLEYE_RUNTIME` is silently overwritten
  (`runners/wine.py` assigns after `system_config["env"]` is merged). Either respect the user value or log it.
- Default `eac`/`battleye` to on is fine; showing per-title enablement would prevent false expectations.

## Heroic, Bottles

- Heroic: same per-title enablement hint. Bottles: add the two runtime variables as a per-bottle option.
