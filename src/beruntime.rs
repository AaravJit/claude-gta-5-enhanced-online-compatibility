//! Validation of a "Proton BattlEye Runtime" directory.
//!
//! Layout is taken from Valve's Wine fork, dlls/ntdll/unix/loader.c
//! set_dll_path(): when PROTON_BATTLEYE_RUNTIME is set, `<dir>/v1/lib/wine/`
//! and `<dir>/v1/lib64/wine/` are appended to the builtin DLL search path, and
//! the `proton` script forces `beclient.dll` / `beclient_x64.dll` to "builtin,native".
//! The runtime is therefore a *Linux-side userspace* BEClient replacement,
//! not a Windows kernel driver.
use crate::ctx;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Runtime {
    pub has_lib32: bool,
    pub has_lib64: bool,
    pub modules: Vec<String>,
}

impl Runtime {
    pub fn usable(&self) -> bool {
        self.has_lib64
            && self
                .modules
                .iter()
                .any(|m| m.to_ascii_lowercase().contains("beclient"))
    }
}

fn collect(dir: &Path, depth: u8, out: &mut Vec<String>, base: &Path) {
    for f in ctx::files(dir) {
        if let Ok(rel) = f.strip_prefix(base) {
            out.push(rel.to_string_lossy().into_owned());
        }
    }
    if depth > 0 {
        for d in ctx::subdirs(dir) {
            collect(&d, depth - 1, out, base);
        }
    }
}

pub fn inspect(dir: &Path) -> Option<Runtime> {
    if !dir.is_dir() {
        return None;
    }
    let lib32 = dir.join("v1/lib/wine");
    let lib64 = dir.join("v1/lib64/wine");
    let mut modules = vec![];
    collect(&lib32, 2, &mut modules, dir);
    collect(&lib64, 2, &mut modules, dir);
    Some(Runtime {
        has_lib32: lib32.is_dir(),
        has_lib64: lib64.is_dir(),
        modules,
    })
}
