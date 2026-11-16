//! Detection and reversible removal of BattlEye hostname overrides in a hosts file.
//!
//! Any hosts-file entry for a `*.battleye.com` name redirects the anti-cheat
//! client away from its real service. That is the opposite of "BattlEye working
//! as intended", so this module only ever *removes* such entries.
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Override {
    pub line_no: usize,
    pub address: String,
    pub host: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub overrides: Vec<Override>,
    pub cleaned: String,
}

pub fn is_battleye_host(h: &str) -> bool {
    let h = h.trim_end_matches('.').to_ascii_lowercase();
    h == "battleye.com" || h.ends_with(".battleye.com")
}

/// Compute the cleaned file. Lines that carry no BattlEye host are preserved
/// byte-for-byte; a line mixing BattlEye and unrelated hosts keeps the unrelated ones.
pub fn plan(content: &str) -> Plan {
    let mut overrides = vec![];
    let mut cleaned = String::with_capacity(content.len());
    for (i, raw) in content.split_inclusive('\n').enumerate() {
        let (body, eol) = match raw.strip_suffix('\n') {
            Some(b) => (b, "\n"),
            None => (raw, ""),
        };
        let (active, comment) = match body.find('#') {
            Some(p) => (&body[..p], Some(&body[p..])),
            None => (body, None),
        };
        let mut toks = active.split_whitespace();
        let addr = toks.next();
        let hosts: Vec<&str> = toks.collect();
        let hit: Vec<&str> = hosts
            .iter()
            .copied()
            .filter(|h| is_battleye_host(h))
            .collect();
        match addr {
            Some(addr) if !hit.is_empty() => {
                for h in &hit {
                    overrides.push(Override {
                        line_no: i + 1,
                        address: addr.to_string(),
                        host: h.to_string(),
                    });
                }
                let keep: Vec<&str> = hosts
                    .iter()
                    .copied()
                    .filter(|h| !is_battleye_host(h))
                    .collect();
                if !keep.is_empty() {
                    cleaned.push_str(addr);
                    for k in keep {
                        cleaned.push(' ');
                        cleaned.push_str(k);
                    }
                    if let Some(c) = comment {
                        cleaned.push(' ');
                        cleaned.push_str(c);
                    }
                    cleaned.push_str(eol);
                }
            }
            _ => cleaned.push_str(raw),
        }
    }
    Plan { overrides, cleaned }
}

/// Unified-ish diff of removed/changed lines, for dry-run output.
pub fn diff(before: &str, after: &str) -> String {
    let a: Vec<&str> = before.lines().collect();
    let b: Vec<&str> = after.lines().collect();
    let mut out = String::new();
    let (mut i, mut j) = (0, 0);
    while i < a.len() {
        if j < b.len() && a[i] == b[j] {
            i += 1;
            j += 1;
        } else if j < b.len() && plan(&format!("{}\n", a[i])).cleaned.trim_end() == b[j] {
            out.push_str(&format!("-{}\n+{}\n", a[i], b[j]));
            i += 1;
            j += 1;
        } else {
            out.push_str(&format!("-{}\n", a[i]));
            i += 1;
        }
    }
    out
}

fn backup_prefix(file: &Path) -> String {
    format!(
        "{}.gtaol-doctor.",
        file.file_name().and_then(|s| s.to_str()).unwrap_or("hosts")
    )
}

/// Write the cleaned file atomically after saving a sibling backup. Returns the backup path.
pub fn apply(file: &Path, ts: u64) -> Result<Option<PathBuf>, String> {
    let before = std::fs::read_to_string(file)
        .map_err(|e| format!("cannot read {}: {e}", file.display()))?;
    let p = plan(&before);
    if p.overrides.is_empty() {
        return Ok(None);
    }
    let dir = file.parent().unwrap_or(Path::new("."));
    let backup = dir.join(format!("{}{}.bak", backup_prefix(file), ts));
    std::fs::copy(file, &backup).map_err(|e| {
        format!(
            "cannot write backup {}: {e} (root needed for /etc?)",
            backup.display()
        )
    })?;
    replace_atomically(file, &p.cleaned)?;
    Ok(Some(backup))
}

fn replace_atomically(file: &Path, content: &str) -> Result<(), String> {
    let dir = file.parent().unwrap_or(Path::new("."));
    let tmp = dir.join(format!(".{}tmp", backup_prefix(file)));
    std::fs::write(&tmp, content).map_err(|e| format!("cannot write {}: {e}", tmp.display()))?;
    if let Ok(meta) = std::fs::metadata(file) {
        let _ = std::fs::set_permissions(&tmp, meta.permissions());
    }
    std::fs::rename(&tmp, file).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("cannot replace {}: {e}", file.display())
    })
}

/// Newest backup created by `apply` next to `file`.
pub fn latest_backup(file: &Path) -> Option<PathBuf> {
    let dir = file.parent()?;
    let prefix = backup_prefix(file);
    let mut best: Option<(u64, PathBuf)> = None;
    for f in crate::ctx::files(dir) {
        let name = f.file_name()?.to_str()?.to_string();
        if let Some(ts) = name
            .strip_prefix(&prefix)
            .and_then(|r| r.strip_suffix(".bak"))
            .and_then(|t| t.parse::<u64>().ok())
        {
            if best.as_ref().map_or(true, |(b, _)| ts > *b) {
                best = Some((ts, f));
            }
        }
    }
    best.map(|(_, p)| p)
}

pub fn rollback(file: &Path) -> Result<PathBuf, String> {
    let b = latest_backup(file)
        .ok_or_else(|| format!("no gtaol-doctor backup found next to {}", file.display()))?;
    let content =
        std::fs::read_to_string(&b).map_err(|e| format!("cannot read {}: {e}", b.display()))?;
    replace_atomically(file, &content)?;
    Ok(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "# Static table lookup for hostnames.\n127.0.0.1 localhost\n::1 localhost\n127.0.1.1 AaravsArchlinux.localdomain AaravsArchlinux\n0.0.0.0 test-s1.battleye.com\n0.0.0.0 paradiseenhanced-s1.battleye.com\n";

    #[test]
    fn detects_the_two_workaround_entries() {
        let p = plan(SAMPLE);
        let hosts: Vec<&str> = p.overrides.iter().map(|o| o.host.as_str()).collect();
        assert_eq!(
            hosts,
            ["test-s1.battleye.com", "paradiseenhanced-s1.battleye.com"]
        );
        assert_eq!(p.overrides[0].line_no, 5);
    }

    #[test]
    fn preserves_unrelated_lines_byte_for_byte() {
        let p = plan(SAMPLE);
        assert_eq!(p.cleaned, "# Static table lookup for hostnames.\n127.0.0.1 localhost\n::1 localhost\n127.0.1.1 AaravsArchlinux.localdomain AaravsArchlinux\n");
    }

    #[test]
    fn mixed_line_keeps_other_hosts_and_comment() {
        let p = plan("0.0.0.0 ads.example.com TEST-S1.BattlEye.com # blocklist\n");
        assert_eq!(p.cleaned, "0.0.0.0 ads.example.com # blocklist\n");
        assert_eq!(p.overrides.len(), 1);
    }

    #[test]
    fn commented_entries_and_lookalikes_are_untouched() {
        let s = "#0.0.0.0 test-s1.battleye.com\n0.0.0.0 notbattleye.com\n0.0.0.0 battleye.com.evil.example\n";
        let p = plan(s);
        assert!(p.overrides.is_empty());
        assert_eq!(p.cleaned, s);
    }

    #[test]
    fn no_trailing_newline_and_crlf_free_roundtrip() {
        let s = "127.0.0.1 localhost";
        assert_eq!(plan(s).cleaned, s);
    }

    #[test]
    fn clean_is_idempotent() {
        let once = plan(SAMPLE).cleaned;
        let twice = plan(&once);
        assert!(twice.overrides.is_empty());
        assert_eq!(twice.cleaned, once);
    }

    #[test]
    fn diff_shows_only_removed_lines() {
        let p = plan(SAMPLE);
        assert_eq!(
            diff(SAMPLE, &p.cleaned),
            "-0.0.0.0 test-s1.battleye.com\n-0.0.0.0 paradiseenhanced-s1.battleye.com\n"
        );
    }
}
