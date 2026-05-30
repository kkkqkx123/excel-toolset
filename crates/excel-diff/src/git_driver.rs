use std::process::Command;

pub fn install_git_driver() -> Result<(), String> {
    let git_attr = "*.xlsx diff=excel-diff\n";

    let gitattr_path = get_gitattributes_path()?;
    std::fs::write(&gitattr_path, git_attr)
        .map_err(|e| format!("Failed to write .gitattributes: {}", e))?;

    let output = Command::new("git")
        .args(["config", "diff.excel-diff.command", "excel-cli diff file"])
        .output()
        .map_err(|e| format!("Failed to run git config: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "git config failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}

pub fn uninstall_git_driver() -> Result<(), String> {
    let gitattr_path = get_gitattributes_path()?;
    if gitattr_path.exists() {
        std::fs::remove_file(&gitattr_path)
            .map_err(|e| format!("Failed to remove .gitattributes: {}", e))?;
    }

    let output = Command::new("git")
        .args(["config", "--unset", "diff.excel-diff.command"])
        .output()
        .map_err(|e| format!("Failed to unset git config: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "git config --unset failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}

fn get_gitattributes_path() -> Result<std::path::PathBuf, String> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| format!("Failed to find git root: {}", e))?;

    if !output.status.success() {
        return Err("Not in a git repository".to_string());
    }

    let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(std::path::PathBuf::from(root).join(".gitattributes"))
}
