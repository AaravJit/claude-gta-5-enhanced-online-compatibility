//! Host facts: distribution, kernel, session, GPUs, Vulkan ICDs.
use crate::ctx::{self, Ctx};
use crate::report::{Finding, Status};

pub fn os_release_field(content: &str, key: &str) -> Option<String> {
    content.lines().find_map(|l| {
        let (k, v) = l.split_once('=')?;
        (k.trim() == key).then(|| v.trim().trim_matches('"').to_string())
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gpu {
    pub slot: String,
    pub vendor: String,
    pub device: String,
    pub subsystem: String,
    pub driver: Option<String>,
}

fn hex(p: std::path::PathBuf) -> String {
    ctx::read(&p)
        .map(|s| s.trim().trim_start_matches("0x").to_string())
        .unwrap_or_else(|| "????".into())
}

pub fn gpus(c: &Ctx) -> Vec<Gpu> {
    let mut v = vec![];
    for dev in ctx::subdirs(&c.sys("/sys/bus/pci/devices")) {
        let class = hex(dev.join("class"));
        if !class.starts_with("03") {
            continue;
        }
        let driver = std::fs::read_link(dev.join("driver"))
            .ok()
            .and_then(|l| l.file_name().map(|n| n.to_string_lossy().into_owned()));
        v.push(Gpu {
            slot: dev.file_name().unwrap().to_string_lossy().into_owned(),
            vendor: hex(dev.join("vendor")),
            device: hex(dev.join("device")),
            subsystem: format!(
                "{}:{}",
                hex(dev.join("subsystem_vendor")),
                hex(dev.join("subsystem_device"))
            ),
            driver,
        });
    }
    v
}

pub fn vendor_name(id: &str) -> &'static str {
    match id {
        "1002" => "AMD",
        "10de" => "NVIDIA",
        "8086" => "Intel",
        _ => "unknown vendor",
    }
}

pub fn vulkan_icds(c: &Ctx) -> Vec<String> {
    let mut v = vec![];
    for d in ["/usr/share/vulkan/icd.d", "/etc/vulkan/icd.d"] {
        for f in ctx::files(&c.sys(d)) {
            if f.extension().is_some_and(|e| e == "json") {
                v.push(f.file_name().unwrap().to_string_lossy().into_owned());
            }
        }
    }
    v
}

fn vulkaninfo_summary(c: &Ctx) -> Option<Vec<String>> {
    if !c.run_commands {
        return None;
    }
    let out = std::process::Command::new("vulkaninfo")
        .arg("--summary")
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    let keep = [
        "deviceName",
        "driverName",
        "driverInfo",
        "apiVersion",
        "deviceType",
    ];
    Some(
        text.lines()
            .filter(|l| keep.iter().any(|k| l.contains(k)))
            .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
            .collect(),
    )
}

pub fn findings(c: &Ctx) -> Vec<Finding> {
    let mut out = vec![];

    match ctx::read(&c.sys("/etc/os-release")) {
        Some(s) => {
            let name = os_release_field(&s, "PRETTY_NAME")
                .or_else(|| os_release_field(&s, "NAME"))
                .unwrap_or_else(|| "unknown".into());
            let id = os_release_field(&s, "ID").unwrap_or_default();
            let like = os_release_field(&s, "ID_LIKE").unwrap_or_default();
            out.push(Finding::new(
                "SYS-DISTRO",
                Status::Info,
                format!(
                    "{name} (id={id} like={like}) arch={}",
                    std::env::consts::ARCH
                ),
            ));
        }
        None => out.push(Finding::new(
            "SYS-DISTRO",
            Status::Unknown,
            "could not read /etc/os-release",
        )),
    }

    match ctx::read(&c.sys("/proc/sys/kernel/osrelease")) {
        Some(k) => out.push(Finding::new(
            "SYS-KERNEL",
            Status::Info,
            format!("Linux {}", k.trim()),
        )),
        None => out.push(Finding::new(
            "SYS-KERNEL",
            Status::Unknown,
            "could not read kernel release",
        )),
    }

    let session = c
        .env
        .get("XDG_SESSION_TYPE")
        .cloned()
        .filter(|s| !s.is_empty());
    let desktop = c
        .env
        .get("XDG_CURRENT_DESKTOP")
        .cloned()
        .unwrap_or_default();
    out.push(match session {
        Some(s) => Finding::new(
            "SYS-SESSION",
            Status::Info,
            format!("session={s} desktop={desktop}"),
        ),
        None => Finding::new(
            "SYS-SESSION",
            Status::Unknown,
            "XDG_SESSION_TYPE not set (run from inside the desktop session)",
        ),
    });

    let g = gpus(c);
    if g.is_empty() {
        out.push(Finding::new(
            "SYS-GPU",
            Status::Unknown,
            "no PCI display device found under /sys/bus/pci/devices",
        ));
    }
    for gpu in &g {
        let st = if gpu.driver.is_some() {
            Status::Ok
        } else {
            Status::Warn
        };
        out.push(Finding::new(
            "SYS-GPU",
            st,
            format!(
                "{} [{}:{}] subsystem {} at {} driver={}",
                vendor_name(&gpu.vendor),
                gpu.vendor,
                gpu.device,
                gpu.subsystem,
                gpu.slot,
                gpu.driver.as_deref().unwrap_or("NONE BOUND")
            ),
        ));
    }

    let icds = vulkan_icds(c);
    let mut f = if icds.is_empty() {
        Finding::new("SYS-VULKAN", Status::Warn, "no Vulkan ICD manifests found")
            .fix("install the Vulkan driver for your GPU (Arch/AMD: vulkan-radeon)")
    } else {
        Finding::new(
            "SYS-VULKAN",
            Status::Ok,
            format!("ICD manifests: {}", icds.join(", ")),
        )
    };
    match vulkaninfo_summary(c) {
        Some(lines) => {
            for l in lines {
                f = f.detail(l);
            }
        }
        None => f = f.detail("vulkaninfo not run or unavailable; driver version unverified (Arch: pacman -S vulkan-tools)"),
    }
    out.push(f);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_os_release() {
        let s = "NAME=\"Arch Linux\"\nPRETTY_NAME=\"Arch Linux\"\nID=arch\nBUILD_ID=rolling\n";
        assert_eq!(os_release_field(s, "ID").as_deref(), Some("arch"));
        assert_eq!(
            os_release_field(s, "PRETTY_NAME").as_deref(),
            Some("Arch Linux")
        );
        assert_eq!(os_release_field(s, "ID_LIKE"), None);
    }
}
