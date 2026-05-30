use std::process::Command;

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap_or_else(|_| "tests/git_e2e/fixtures".to_string());
    let out_dir = std::path::Path::new(&out_dir);

    let exe = std::env::current_exe().unwrap();
    let manifest = std::fs::read_to_string("Cargo.toml").unwrap();
    let test_exe = std::env::var("CARGO_BIN_EXE_excel_diff").is_ok();

    let fixtures_dir = if test_exe {
        std::path::PathBuf::from("tests/git_e2e/fixtures")
    } else {
        std::path::PathBuf::from(&out_dir)
    };

    std::fs::create_dir_all(&fixtures_dir).unwrap();

    let exe_path = exe.parent().unwrap().join("excel-cli");
    let need_build = !exe_path.with_extension("exe").exists();

    if need_build {
        eprintln!("cargo build --package excel-cli ...");
    }

    let script = if cfg!(windows) {
        let bat_path = fixtures_dir.join("diff_driver.bat");
        let bat_content = if need_build {
            let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
            format!(
                "@echo off\n\
                 \"{}\" --manifest-path \"{}\" run --package excel-cli -- diff file %1 %2\n",
                std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string()),
                manifest_path.display()
            )
        } else {
            format!(
                "@echo off\n\
                 \"{}\" diff file %1 %2\n",
                exe_path.display()
            )
        };
        std::fs::write(&bat_path, &bat_content).unwrap();
        bat_path
    } else {
        let sh_path = fixtures_dir.join("diff_driver.sh");
        let sh_content = if need_build {
            let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
            format!(
                "#!/bin/sh\n\
                 cargo run --manifest-path '{}' --package excel-cli -- diff file \"$1\" \"$2\"\n",
                manifest_path.display()
            )
        } else {
            format!(
                "#!/bin/sh\n\
                 '{}' diff file \"$1\" \"$2\"\n",
                exe_path.display()
            )
        };
        std::fs::write(&sh_path, &sh_content).unwrap();
        #[cfg(unix)]
        std::fs::set_permissions(&sh_path, std::os::unix::fs::PermissionsExt::from_mode(0o755)).ok();
        sh_path
    };

    println!("cargo:warning=Diff driver script: {:?}", script);
}