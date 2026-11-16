//! Embedded per-title vendor-enablement database (data/titles.conf).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Title {
    pub key: String,
    pub name: String,
    pub steam_appid: String,
    pub dir_names: Vec<String>,
    pub exe_names: Vec<String>,
    pub anticheat: String,
    /// "enabled" | "not-enabled" | "unknown"
    pub proton_enablement: String,
    pub as_of: String,
    pub evidence: String,
    pub sources: Vec<String>,
}

const EMBEDDED: &str = include_str!("../data/titles.conf");

fn list(v: &str) -> Vec<String> {
    v.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

pub fn parse(src: &str) -> Vec<Title> {
    let mut out: Vec<Title> = vec![];
    for line in src.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(k) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            out.push(Title {
                key: k.to_string(),
                proton_enablement: "unknown".into(),
                ..Default::default()
            });
            continue;
        }
        let (Some(t), Some((k, v))) = (out.last_mut(), line.split_once('=')) else {
            continue;
        };
        let v = v.trim();
        match k.trim() {
            "name" => t.name = v.into(),
            "steam_appid" => t.steam_appid = v.into(),
            "dir_names" => t.dir_names = list(v),
            "exe_names" => t.exe_names = list(v),
            "anticheat" => t.anticheat = v.into(),
            "proton_enablement" => t.proton_enablement = v.into(),
            "as_of" => t.as_of = v.into(),
            "evidence" => t.evidence = v.into(),
            "sources" => t.sources = list(v),
            _ => {}
        }
    }
    out
}

pub fn all() -> Vec<Title> {
    parse(EMBEDDED)
}

pub fn gta5_enhanced() -> Title {
    all()
        .into_iter()
        .find(|t| t.key == "gta5-enhanced")
        .expect("embedded database contains gta5-enhanced")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_db_is_wellformed() {
        let t = gta5_enhanced();
        assert_eq!(t.steam_appid, "3240220");
        assert!(["enabled", "not-enabled", "unknown"].contains(&t.proton_enablement.as_str()));
        assert!(!t.sources.is_empty() && !t.as_of.is_empty());
        assert!(t
            .dir_names
            .contains(&"Grand Theft Auto V Enhanced".to_string()));
    }

    #[test]
    fn missing_enablement_defaults_to_unknown_not_enabled() {
        let t = parse("[x]\nname = X\n");
        assert_eq!(t[0].proton_enablement, "unknown");
    }
}
