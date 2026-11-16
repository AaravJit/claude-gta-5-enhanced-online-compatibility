#!/usr/bin/env bash
# Read-only baseline collection for a bug report. Changes nothing. Needs no root.
# Does NOT collect: hostnames, MAC/IP addresses, serial numbers, machine-id,
# account names, tokens, cookies, or launcher login state.
# Usage: bash scripts/collect-baseline.sh > baseline.txt   (review before sharing)
set -u
section() { printf '\n===== %s =====\n' "$1"; }
have() { command -v "$1" >/dev/null 2>&1; }
scrub() { sed -e "s#${HOME}#~#g" -e "s#\b${USER:-nouser}\b#USER#g"; }

{
section "kernel";            uname -srmo
section "os-release";        grep -E '^(NAME|PRETTY_NAME|ID|ID_LIKE|BUILD_ID|VERSION_ID)=' /etc/os-release
section "cpu";               have lscpu && lscpu | grep -E '^(Architecture|Model name|CPU\(s\)|Thread|Core|Socket)'
section "gpu (lspci -nnk)";  have lspci && lspci -nnk | grep -A3 -E 'VGA|3D|Display'
section "block devices";     have lsblk && lsblk -o NAME,TYPE,FSTYPE,SIZE,MOUNTPOINTS | grep -v loop   # no UUIDs/serials
section "session";           echo "XDG_SESSION_TYPE=${XDG_SESSION_TYPE:-unset} XDG_CURRENT_DESKTOP=${XDG_CURRENT_DESKTOP:-unset}"
section "kernel modules";    have lsmod && lsmod | grep -E '^(amdgpu|nvidia|i915|xe|vboxdrv|kvm)' || echo "(none of the listed modules)"
section "packages (pacman)"; have pacman && pacman -Q linux linux-zen linux-lts mesa vulkan-radeon lib32-vulkan-radeon vulkan-icd-loader lib32-vulkan-icd-loader steam lutris umu-launcher wine wine-staging vulkan-tools 2>&1 | sed 's/^error: package /missing: /'
section "vulkan";            have vulkaninfo && vulkaninfo --summary 2>/dev/null | grep -E 'deviceName|driverName|driverInfo|apiVersion|deviceType' || echo "vulkaninfo not installed (pacman -S vulkan-tools)"
section "secure boot";       have bootctl && bootctl status 2>/dev/null | grep -E 'Secure Boot|Current Boot Loader|Product' || echo "bootctl unavailable"
section "lutris";            have lutris && lutris --version 2>/dev/null | tail -1
section "proton builds";     ls -1 "$HOME/.local/share/Steam/compatibilitytools.d" "$HOME/.local/share/lutris/runners/wine" "$HOME/.local/share/lutris/runners/proton" 2>/dev/null
section "battleye runtimes"; for d in "$HOME/.local/share/Steam/steamapps/common/Proton BattlEye Runtime" "$HOME/.local/share/lutris/runtime/battleye_runtime"; do [ -d "$d" ] && { echo "$d"; find "$d" -maxdepth 4 -type f | head -20; } || echo "absent: $d"; done
section "lutris game config (anti-cheat + env keys only)"
grep -l -i 'rockstar' "$HOME"/.local/share/lutris/games/*.yml "$HOME"/.config/lutris/games/*.yml 2>/dev/null | while read -r f; do echo "$f"; grep -E '^\s*(battleye|eac|version|prefix|esync|fsync|dxvk|PROTON_[A-Z_]+|SteamDeck|WINE[A-Z_]*):' "$f"; done
section "hosts (battleye lines only)"; grep -n -i 'battleye' /etc/hosts || echo "no battleye entries"
section "gtaol-doctor";      have gtaol-doctor && gtaol-doctor diagnose || echo "gtaol-doctor not on PATH"
} 2>&1 | scrub
