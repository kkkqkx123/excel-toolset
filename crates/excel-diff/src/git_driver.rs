use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const GITATTR_ENTRY: &str = "*.xlsx diff=excel-diff\n";
const GITATTR_PATTERN: &str = "*.xlsx diff=excel-diff";

pub fn install_git_driver() -> Result<(), String> {
    let gitattr_path = get_gitattributes_path()?;

    let gitattr_existed = gitattr_path.exists();
    let mut need_write = true;

    if gitattr_existed {
        let content = fs::read_to_string(&gitattr_path)
            .map_err(|e| format!("Failed to read .gitattributes: {}", e))?;
        if content.contains(GITATTR_PATTERN) {
            need_write = false;
        }
    }

    if need_write {
        if gitattr_existed {
            let content = fs::read_to_string(&gitattr_path)
                .map_err(|e| format!("Failed to read .gitattributes: {}", e))?;
            fs::write(&gitattr_path, content + GITATTR_ENTRY)
                .map_err(|e| format!("Failed to write .gitattributes: {}", e))?;
        } else {
            fs::write(&gitattr_path, GITATTR_ENTRY)
                .map_err(|e| format!("Failed to write .gitattributes: {}", e))?;
        }
    }

    let exe_path = get_invocation_command()?;

    let output = Command::new("git")
        .args(["config", "diff.excel-diff.command", &exe_path])
        .output()
        .map_err(|e| format!("Failed to run git config: {}", e))?;

    if !output.status.success() {
        if need_write && !gitattr_existed {
            let _ = fs::remove_file(&gitattr_path);
        }
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
        let content = fs::read_to_string(&gitattr_path)
            .map_err(|e| format!("Failed to read .gitattributes: {}", e))?;

        let remaining: String = content
            .lines()
            .filter(|line| !line.contains(GITATTR_PATTERN))
            .collect::<Vec<_>>()
            .join("\n");

        let trimmed = remaining.trim();
        if trimmed.is_empty() {
            fs::remove_file(&gitattr_path)
                .map_err(|e| format!("Failed to remove .gitattributes: {}", e))?;
        } else {
            fs::write(&gitattr_path, trimmed)
                .map_err(|e| format!("Failed to update .gitattributes: {}", e))?;
        }
    }

    let output = Command::new("git")
        .args(["config", "--unset", "diff.excel-diff.command"])
        .output()
        .map_err(|e| format!("Failed to unset git config: {}", e))?;

    if !output.status.success() {
        let exit_code = output.status.code().unwrap_or(-1);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let combined = format!("{}{}", stdout, stderr);
        if exit_code != 5 && !combined.contains("entry does not exist") {
            return Err(format!("git config --unset failed: {}", stderr));
        }
    }

    Ok(())
}

fn get_gitattributes_path() -> Result<PathBuf, String> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| format!("Failed to find git root: {}", e))?;

    if !output.status.success() {
        return Err("Not in a git repository".to_string());
    }

    let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(PathBuf::from(root).join(".gitattributes"))
}

fn get_invocation_command() -> Result<String, String> {
    let exe_path =
        env::current_exe().map_err(|e| format!("Failed to get current executable: {}", e))?;

    let exe_str = exe_path.to_string_lossy();

    if exe_str.contains(' ') {
        Ok(format!("\"{}\" diff file", exe_str))
    } else {
        Ok(format!("{} diff file", exe_str))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_gitattributes_line_pattern() {
        assert!(GITATTR_PATTERN.contains("*.xlsx"));
        assert!(GITATTR_PATTERN.contains("diff=excel-diff"));
    }

    #[test]
    fn test_gitattributes_format() {
        let entry = GITATTR_ENTRY.trim();
        assert!(entry.starts_with("*.xlsx"));
        assert!(entry.contains("diff=excel-diff"));
    }

    #[test]
    fn test_get_invocation_command_contains_diff_file_suffix() {
        let cmd = get_invocation_command().unwrap();
        assert!(
            cmd.contains("diff file"),
            "command should end with 'diff file', got: {}",
            cmd
        );
    }

    #[test]
    fn test_filter_line_removes_excel_diff() {
        let lines = vec![
            "*.xml diff=xml-diff",
            "*.xlsx diff=excel-diff",
            "*.json diff=json-diff",
        ];
        let remaining: Vec<&str> = lines
            .iter()
            .filter(|line| !line.contains(GITATTR_PATTERN))
            .copied()
            .collect();
        assert_eq!(remaining.len(), 2);
        assert!(remaining[0].contains("xml"));
        assert!(remaining[1].contains("json"));
    }

    #[test]
    fn test_gitattributes_excel_only_is_empty_after_filter() {
        let content = "*.xlsx diff=excel-diff\n";
        let remaining: String = content
            .lines()
            .filter(|line| !line.contains(GITATTR_PATTERN))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(remaining.trim().is_empty());
    }

    #[test]
    fn test_gitattributes_no_excel_entry_stays_unchanged() {
        let content = "*.xml diff=xml-diff\n*.json diff=json-diff\n";
        let remaining: String = content
            .lines()
            .filter(|line| !line.contains(GITATTR_PATTERN))
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(remaining, "*.xml diff=xml-diff\n*.json diff=json-diff");
    }

    #[test]
    fn test_gitattributes_trailing_newline_is_preserved() {
        let content = "*.xml diff=xml-diff\n";
        let mut need_write = true;
        if content.contains(GITATTR_PATTERN) {
            need_write = false;
        }
        assert!(need_write);

        let new_content = content.to_string() + GITATTR_ENTRY;
        assert!(new_content.ends_with('\n'));
        assert!(new_content.contains("*.xlsx"));
        assert!(new_content.contains("*.xml"));
    }
}
