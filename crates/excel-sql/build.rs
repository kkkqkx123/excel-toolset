fn main() {
    // DuckDB on Windows uses the Restart Manager API (RmStartSession, RmEndSession, etc.)
    // which requires linking against Rstrtmgr.lib from the Windows SDK.
    // rust-lld.exe does not automatically search Windows SDK paths, so we
    // locate the library and add the search path manually.
    //
    // This issue has been addressed by upstream and can be removed when update dependency.
    #[cfg(target_os = "windows")]
    {
        if let Some(path) = find_rstrtmgr_lib()
            && let Some(parent) = path.parent()
        {
            println!("cargo:rustc-link-search={}", parent.display());
        }
        println!("cargo:rustc-link-lib=dylib=Rstrtmgr");
    }
}

#[cfg(target_os = "windows")]
fn find_rstrtmgr_lib() -> Option<std::path::PathBuf> {
    // Common Windows SDK install locations
    let kit_roots = [
        r"C:\Program Files (x86)\Windows Kits\10\Lib",
        r"C:\Program Files (x86)\Windows Kits\11\Lib",
        r"C:\Program Files\Windows Kits\10\Lib",
        r"C:\Program Files\Windows Kits\11\Lib",
    ];

    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let arch_dir = match arch.as_str() {
        "x86_64" => "x64",
        "x86" => "x86",
        "aarch64" => "arm64",
        _ => "x64",
    };

    for root in &kit_roots {
        let root_path = std::path::Path::new(root);
        if !root_path.exists() {
            continue;
        }

        // Iterate SDK versions (e.g. 10.0.22621.0, 10.0.20348.0, ...)
        if let Ok(entries) = std::fs::read_dir(root_path) {
            for entry in entries.flatten() {
                let path = entry.path().join("um").join(arch_dir).join("Rstrtmgr.lib");
                if path.exists() {
                    return Some(path);
                }
            }
        }
    }

    None
}