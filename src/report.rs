//! Findings and their text / JSON rendering.
use std::fmt::Write;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// Observed and as expected.
    Ok,
    /// Observed fact, no judgement.
    Info,
    /// Worth attention, not a blocker by itself.
    Warn,
    /// Actively incompatible with legitimate BattlEye operation.
    Conflict,
    /// Cannot be fixed on this machine; needs a vendor.
    Blocked,
    /// The tool could not determine this. Never rendered as a pass.
    Unknown,
}

impl Status {
    pub fn tag(self) -> &'static str {
        match self {
            Status::Ok => "OK",
            Status::Info => "INFO",
            Status::Warn => "WARN",
            Status::Conflict => "CONFLICT",
            Status::Blocked => "BLOCKED",
            Status::Unknown => "UNKNOWN",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Finding {
    pub id: &'static str,
    pub status: Status,
    pub title: String,
    pub detail: Vec<String>,
    pub fix: Option<String>,
}

impl Finding {
    pub fn new(id: &'static str, status: Status, title: impl Into<String>) -> Finding {
        Finding {
            id,
            status,
            title: title.into(),
            detail: vec![],
            fix: None,
        }
    }
    pub fn detail(mut self, d: impl Into<String>) -> Finding {
        self.detail.push(d.into());
        self
    }
    pub fn fix(mut self, f: impl Into<String>) -> Finding {
        self.fix = Some(f.into());
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LevelState {
    Verified,
    NotVerified,
    NeedsThisMachine,
    Blocked,
}

impl LevelState {
    pub fn tag(self) -> &'static str {
        match self {
            LevelState::Verified => "VERIFIED",
            LevelState::NotVerified => "NOT VERIFIED",
            LevelState::NeedsThisMachine => "NEEDS MANUAL TEST",
            LevelState::Blocked => "BLOCKED (vendor)",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Level {
    pub n: u8,
    pub name: &'static str,
    pub state: LevelState,
    pub why: String,
}

#[derive(Clone, Debug, Default)]
pub struct Report {
    pub findings: Vec<Finding>,
    pub levels: Vec<Level>,
    pub verdict: String,
}

/// Replace the home directory (and therefore the user name) with `$HOME`, which,
/// unlike `~`, still expands when a printed command quotes a path with spaces.
pub fn sanitize(s: &str, home: &str) -> String {
    if home.len() > 1 {
        s.replace(home, "$HOME")
    } else {
        s.to_string()
    }
}

pub fn json_str(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(o, "\\u{:04x}", c as u32);
            }
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

impl Report {
    pub fn has(&self, st: Status) -> bool {
        self.findings.iter().any(|f| f.status == st)
    }

    pub fn find(&self, id: &str) -> Option<&Finding> {
        self.findings.iter().find(|f| f.id == id)
    }

    pub fn sanitized(mut self, home: &str) -> Report {
        for f in &mut self.findings {
            f.title = sanitize(&f.title, home);
            for d in &mut f.detail {
                *d = sanitize(d, home);
            }
            if let Some(x) = &mut f.fix {
                *x = sanitize(x, home);
            }
        }
        for l in &mut self.levels {
            l.why = sanitize(&l.why, home);
        }
        self
    }

    pub fn to_text(&self) -> String {
        let mut o = String::new();
        let _ = writeln!(
            o,
            "gtaol-doctor {} - GTA V Enhanced / BattlEye on Linux diagnostics",
            env!("CARGO_PKG_VERSION")
        );
        let _ = writeln!(o, "(read-only; nothing on this system was modified)\n");
        for f in &self.findings {
            let _ = writeln!(o, "[{:<8}] {:<22} {}", f.status.tag(), f.id, f.title);
            for d in &f.detail {
                let _ = writeln!(o, "             {}", d);
            }
            if let Some(x) = &f.fix {
                let _ = writeln!(o, "             fix: {}", x);
            }
        }
        let _ = writeln!(o, "\nAcceptance levels");
        for l in &self.levels {
            let _ = writeln!(
                o,
                "  L{} {:<42} {:<18} {}",
                l.n,
                l.name,
                l.state.tag(),
                l.why
            );
        }
        let _ = writeln!(o, "\nVerdict: {}", self.verdict);
        o
    }

    pub fn to_json(&self) -> String {
        let mut o = String::from("{\n  \"tool\": \"gtaol-doctor\",\n");
        let _ = writeln!(o, "  \"version\": {},", json_str(env!("CARGO_PKG_VERSION")));
        let _ = writeln!(o, "  \"verdict\": {},", json_str(&self.verdict));
        o.push_str("  \"findings\": [\n");
        for (i, f) in self.findings.iter().enumerate() {
            let det: Vec<String> = f.detail.iter().map(|d| json_str(d)).collect();
            let _ =
                write!(
                o,
                "    {{\"id\": {}, \"status\": {}, \"title\": {}, \"detail\": [{}], \"fix\": {}}}",
                json_str(f.id),
                json_str(f.status.tag()),
                json_str(&f.title),
                det.join(", "),
                f.fix.as_deref().map(json_str).unwrap_or_else(|| "null".into())
            );
            o.push_str(if i + 1 < self.findings.len() {
                ",\n"
            } else {
                "\n"
            });
        }
        o.push_str("  ],\n  \"levels\": [\n");
        for (i, l) in self.levels.iter().enumerate() {
            let _ = write!(
                o,
                "    {{\"level\": {}, \"name\": {}, \"state\": {}, \"why\": {}}}",
                l.n,
                json_str(l.name),
                json_str(l.state.tag()),
                json_str(&l.why)
            );
            o.push_str(if i + 1 < self.levels.len() {
                ",\n"
            } else {
                "\n"
            });
        }
        o.push_str("  ]\n}\n");
        o
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_escapes_control_and_quotes() {
        assert_eq!(json_str("a\"b\\c\n\u{1}"), "\"a\\\"b\\\\c\\n\\u0001\"");
    }

    #[test]
    fn sanitize_strips_home_and_username() {
        let s = sanitize("/home/aarav/Games/x and /home/aarav", "/home/aarav");
        assert_eq!(s, "$HOME/Games/x and $HOME");
        assert!(!s.contains("aarav"));
    }

    #[test]
    fn sanitize_ignores_root_home() {
        assert_eq!(sanitize("/etc/hosts", "/"), "/etc/hosts");
    }
}
