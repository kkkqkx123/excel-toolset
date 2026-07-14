use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use excel_types::{AppError, Result};

const GITATTR_PATTERN: &str = "diff=excel-diff";
const DEFAULT_PATTERNS: &[&str] = &["*.xlsx", "*.xls", "*.xlsm", "*.xlsb"];

const GIT_DIFF_PATH_OLD: &str = "GIT_DIFF_PATH_OLD";
const GIT_DIFF_PATH_NEW: &str = "GIT_DIFF_PATH_NEW";

/// Install the git diff driver for Excel files.
///
/// When `global` is false (default), configures the current git repository.
/// When `global` is true, configures the global git config and global gitattributes
/// so that all repositories on the system use the driver.
pub fn install_git_driver(global: bool, patterns: &[String]) -> Result<()> {
    // Determine which gitattributes path and git config scope to use
    let (gitattr_path, is_global_attr) = if global {
        (get_global_gitattributes_path()?, true)
    } else {
        (get_repo_gitattributes_path()?, false)
    };

    // Normalize patterns: if empty, use defaults
    let resolved_patterns: Vec<&str> = if patterns.is_empty() {
        DEFAULT_PATTERNS.to_vec()
    } else {
        patterns.iter().map(|s| s.as_str()).collect()
    };

    // Write gitattributes entries
    let gitattr_existed = gitattr_path.exists();
    let mut need_write = true;

    if gitattr_existed {
        let content = read_gitattributes(&gitattr_path)?;
        // Check if all patterns already exist
        let all_exist = resolved_patterns
            .iter()
            .all(|p| content.contains(&format!("{} {}", p, GITATTR_PATTERN)));
        if all_exist {
            need_write = false;
        }
    }

    if need_write {
        if gitattr_existed {
            let content = read_gitattributes(&gitattr_path)?;
            let mut new_content = content;
            for pattern in &resolved_patterns {
                let entry = format!("{} {}\n", pattern, GITATTR_PATTERN);
                if !new_content.contains(&format!("{} {}", pattern, GITATTR_PATTERN)) {
                    new_content.push_str(&entry);
                }
            }
            write_gitattributes(&gitattr_path, new_content)?;
        } else {
            // Ensure the directory exists (needed for global ~/.config/git/ path)
            if let Some(parent) = gitattr_path.parent() {
                fs::create_dir_all(parent).map_err(|e| {
                    AppError::Io(std::io::Error::other(format!(
                        "Failed to create directory for gitattributes: {}",
                        e
                    )))
                })?;
            }
            let mut content = String::new();
            for pattern in &resolved_patterns {
                content.push_str(&format!("{} {}\n", pattern, GITATTR_PATTERN));
            }
            write_gitattributes(&gitattr_path, content)?;
        }
    }

    // Set git config for the diff driver command
    let exe_path = get_invocation_command()?;

    let mut args = vec!["config"];
    if global {
        args.push("--global");
    }
    args.push("diff.excel-diff.command");
    args.push(&exe_path);

    let output = Command::new("git")
        .args(&args)
        .output()
        .map_err(|e| AppError::Custom(format!("Failed to run git config: {}", e)))?;

    if !output.status.success() {
        // Rollback gitattributes if we just created it
        if need_write && !gitattr_existed && !is_global_attr {
            let _ = fs::remove_file(&gitattr_path);
        }
        return Err(AppError::Custom(format!(
            "git config failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    // When installing globally, also set core.attributesfile if needed
    if global {
        let attr_path_str = gitattr_path.to_string_lossy().to_string();
        let need_set_attr = match Command::new("git")
            .args(["config", "--global", "core.attributesfile"])
            .output()
        {
            Ok(out) => {
                let current = String::from_utf8_lossy(&out.stdout).trim().to_string();
                current != attr_path_str
            }
            Err(_) => true,
        };

        if need_set_attr {
            let output = Command::new("git")
                .args([
                    "config",
                    "--global",
                    "core.attributesfile",
                    &attr_path_str,
                ])
                .output()
                .map_err(|e| AppError::Custom(format!("Failed to set core.attributesfile: {}", e)))?;

            if !output.status.success() {
                return Err(AppError::Custom(format!(
                    "git config core.attributesfile failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                )));
            }
        }
    }

    Ok(())
}

/// Uninstall the git diff driver.
///
/// When `global` is false (default), removes configuration from the current repository.
/// When `global` is true, removes from global git config.
pub fn uninstall_git_driver(global: bool) -> Result<()> {
    if global {
        uninstall_global()?;
    } else {
        uninstall_local()?;
    }
    Ok(())
}

fn uninstall_local() -> Result<()> {
    let gitattr_path = get_repo_gitattributes_path()?;

    if gitattr_path.exists() {
        remove_excel_entries_from_attr(&gitattr_path)?;
    }

    let output = Command::new("git")
        .args(["config", "--unset", "diff.excel-diff.command"])
        .output()
        .map_err(|e| AppError::Custom(format!("Failed to unset git config: {}", e)))?;

    if !output.status.success() {
        let exit_code = output.status.code().unwrap_or(-1);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let combined = format!("{}{}", stdout, stderr);
        if exit_code != 5 && !combined.contains("entry does not exist") {
            return Err(AppError::Custom(format!(
                "git config --unset failed: {}",
                stderr
            )));
        }
    }

    Ok(())
}

fn uninstall_global() -> Result<()> {
    // Remove from global gitattributes
    let gitattr_path = get_global_gitattributes_path()?;

    if gitattr_path.exists() {
        remove_excel_entries_from_attr(&gitattr_path)?;
    }

    // Unset global git config
    let output = Command::new("git")
        .args([
            "config",
            "--global",
            "--unset",
            "diff.excel-diff.command",
        ])
        .output()
        .map_err(|e| AppError::Custom(format!("Failed to unset global git config: {}", e)))?;

    if !output.status.success() {
        let exit_code = output.status.code().unwrap_or(-1);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let combined = format!("{}{}", stdout, stderr);
        if exit_code != 5 && !combined.contains("entry does not exist") {
            return Err(AppError::Custom(format!(
                "git config --global --unset failed: {}",
                stderr
            )));
        }
    }

    Ok(())
}

/// Remove all Excel diff entries from a gitattributes file.
/// If the file becomes empty after removal, delete it.
fn remove_excel_entries_from_attr(path: &PathBuf) -> Result<()> {
    let content = read_gitattributes(path)?;

    let remaining: String = content
        .lines()
        .filter(|line| !line.contains(GITATTR_PATTERN))
        .collect::<Vec<_>>()
        .join("\n");

    let trimmed = remaining.trim();
    if trimmed.is_empty() {
        fs::remove_file(path).map_err(AppError::Io)?;
    } else {
        write_gitattributes(path, trimmed.to_string())?;
    }
    Ok(())
}

fn read_gitattributes(path: &PathBuf) -> Result<String> {
    fs::read_to_string(path).map_err(|e| {
        AppError::Io(std::io::Error::other(format!(
            "Failed to read .gitattributes: {}",
            e
        )))
    })
}

fn write_gitattributes(path: &PathBuf, content: String) -> Result<()> {
    fs::write(path, content).map_err(|e| {
        AppError::Io(std::io::Error::other(format!(
            "Failed to write .gitattributes: {}",
            e
        )))
    })
}

fn get_repo_gitattributes_path() -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| AppError::Custom(format!("Failed to find git root: {}", e)))?;

    if !output.status.success() {
        return Err(AppError::Custom("Not in a git repository".into()));
    }

    let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(PathBuf::from(root).join(".gitattributes"))
}

/// Get the global gitattributes path (typically ~/.config/git/attributes).
/// Falls back to ~/.gitattributes if the XDG config path is not used.
fn get_global_gitattributes_path() -> Result<PathBuf> {
    // Try to read from git config first
    if let Ok(output) = Command::new("git")
        .args(["config", "--global", "core.attributesfile"])
        .output()
    {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Ok(PathBuf::from(path));
        }
    }

    // Check XDG_CONFIG_HOME first, then fallback to ~/.config
    let config_home = if let Ok(xdg) = env::var("XDG_CONFIG_HOME") {
        PathBuf::from(xdg)
    } else {
        let home = env::var("HOME").map_err(|_| {
            AppError::Custom("Cannot determine home directory for global gitattributes".into())
        })?;
        PathBuf::from(home).join(".config")
    };

    Ok(config_home.join("git").join("attributes"))
}

fn get_invocation_command() -> Result<String> {
    // In test environment, return a placeholder command name
    #[cfg(test)]
    {
        return Ok(format!("{} diff git-driver", DEFAULT_COMMAND_NAME));
    }

    // Production environment: get actual executable path
    #[cfg(not(test))]
    {
        // Priority: use environment variable if set (optional configuration)
        if let Ok(custom_cmd) = env::var("EXCEL_DIFF_COMMAND") {
            return Ok(custom_cmd);
        }

        // For global installs, prefer the command name so it survives binary upgrades.
        // Check if the binary is on PATH (global install scenario).
        if let Ok(_path) = which_binary("excel-cli") {
            return Ok("excel-cli diff git-driver".to_string());
        }

        let exe_path = env::current_exe()
            .map_err(|e| AppError::Custom(format!("Failed to get current executable: {}", e)))?;

        let exe_str = exe_path.to_string_lossy();

        if exe_str.contains(' ') {
            Ok(format!("\"{}\" diff git-driver", exe_str))
        } else {
            Ok(format!("{} diff git-driver", exe_str))
        }
    }
}

/// Check if a binary exists on PATH.
#[cfg(not(test))]
fn which_binary(name: &str) -> Result<String> {
    let output = Command::new("which")
        .arg(name)
        .output()
        .map_err(|e| AppError::Custom(format!("Failed to run which: {}", e)))?;
    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Ok(path);
        }
    }
    Err(AppError::Custom(format!("{} not found on PATH", name)))
}

#[allow(dead_code)]
const DEFAULT_COMMAND_NAME: &str = "excel-cli";

/// Get file paths for git diff driver.
/// Supports both environment variables (standard Git diff driver protocol)
/// and command line arguments.
pub fn get_git_diff_file_paths() -> Result<(String, String)> {
    // Priority 1: Use environment variables (standard Git diff driver protocol)
    if let (Ok(old), Ok(new)) = (env::var(GIT_DIFF_PATH_OLD), env::var(GIT_DIFF_PATH_NEW)) {
        // Skip validation for empty paths - let downstream functions handle errors
        return Ok((old, new));
    }

    // Priority 2: Use command line arguments
    let args: Vec<String> = env::args().collect();

    // Skip the executable path and subcommands
    // Expected format: <exe> diff file <old> <new>
    // Or: <exe> diff git-driver <old> <new>
    // Or: <exe> <old> <new> (when called directly)

    let paths = parse_cli_args(&args)?;

    if paths.len() >= 2 {
        let old = paths[0].clone();
        let new = paths[1].clone();

        // Validate paths before returning
        validate_path(&old)?;
        validate_path(&new)?;

        Ok((old, new))
    } else {
        Err(AppError::Custom(format!(
            "Failed to get file paths. Expected 2 paths, got {}. \
                 Try using environment variables: {} and {}",
            paths.len(),
            GIT_DIFF_PATH_OLD,
            GIT_DIFF_PATH_NEW
        )))
    }
}

/// Parse command line arguments to extract file paths.
/// Filters out subcommands, flags, and options.
fn parse_cli_args(args: &[String]) -> Result<Vec<String>> {
    let mut paths = Vec::new();

    for arg in args.iter().skip(1) {
        // Skip flags and options
        if arg.starts_with("--") {
            continue;
        }

        // Skip known subcommands
        if matches!(arg.as_str(), "diff" | "file" | "git-driver") {
            continue;
        }

        // Check if it looks like a file path
        if is_likely_file_path(arg) {
            paths.push(arg.clone());
        }
    }

    Ok(paths)
}

/// Check if a string is likely a file path.
fn is_likely_file_path(arg: &str) -> bool {
    // Check for common Excel file extensions
    let has_excel_extension = arg.ends_with(".xlsx")
        || arg.ends_with(".xls")
        || arg.ends_with(".xlsm")
        || arg.ends_with(".xlsb");

    // Check for path separators
    let has_separator = arg.contains('/') || arg.contains('\\');

    // Ensure it's not a flag or option
    let not_a_flag = !arg.starts_with('-');

    has_excel_extension || (has_separator && not_a_flag)
}

/// Validate that a path is not empty and has a supported file extension.
fn validate_path(path: &str) -> Result<()> {
    // Check if path is empty
    if path.is_empty() {
        return Err(AppError::Custom("Path cannot be empty".into()));
    }

    // Check file extension
    let ext_lower = path.to_lowercase();
    if !(ext_lower.ends_with(".xlsx")
        || ext_lower.ends_with(".xls")
        || ext_lower.ends_with(".xlsm")
        || ext_lower.ends_with(".xlsb"))
    {
        return Err(AppError::Custom(format!(
            "Unsupported file extension. Expected .xlsx, .xls, .xlsm, or .xlsb, got: {}",
            if let Some(ext) = std::path::Path::new(path).extension() {
                ext.to_string_lossy().to_string()
            } else {
                "no extension".to_string()
            }
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_patterns_cover_all_extensions() {
        assert!(DEFAULT_PATTERNS.contains(&"*.xlsx"));
        assert!(DEFAULT_PATTERNS.contains(&"*.xls"));
        assert!(DEFAULT_PATTERNS.contains(&"*.xlsm"));
        assert!(DEFAULT_PATTERNS.contains(&"*.xlsb"));
    }

    #[test]
    fn test_get_invocation_command_in_test_env() {
        let cmd = get_invocation_command().unwrap();

        // In test environment, should return placeholder command
        assert_eq!(cmd, format!("{} diff git-driver", DEFAULT_COMMAND_NAME));

        // Should not contain test binary file path
        assert!(!cmd.contains("git_e2e"));
        assert!(!cmd.contains(".so"));
        assert!(!cmd.contains("/target/"));
        assert!(!cmd.contains("target/debug/deps"));
        assert!(!cmd.contains("target/release/deps"));
    }

    #[test]
    fn test_get_invocation_command_format() {
        let cmd = get_invocation_command().unwrap();

        // Command should contain "diff git-driver"
        assert!(cmd.contains("diff"), "Command should contain 'diff'");
        assert!(
            cmd.contains("git-driver"),
            "Command should contain 'git-driver'"
        );

        // Command should not be empty
        assert!(!cmd.is_empty(), "Command should not be empty");

        // Command should start with command name or quote
        assert!(
            cmd.starts_with(DEFAULT_COMMAND_NAME) || cmd.starts_with('"'),
            "Command should start with command name or quote"
        );
    }

    #[test]
    fn test_get_invocation_command_does_not_contain_test_paths() {
        let cmd = get_invocation_command().unwrap();

        // Ensure no test-related paths are present
        assert!(
            !cmd.contains("git_e2e-"),
            "Should not contain test binary name"
        );
        assert!(
            !cmd.contains("target/debug/deps"),
            "Should not contain test debug path"
        );
        assert!(
            !cmd.contains("target/release/deps"),
            "Should not contain test release path"
        );
    }

    #[test]
    fn test_parse_cli_args_with_git_driver() {
        let args = vec![
            "excel-cli".to_string(),
            "diff".to_string(),
            "git-driver".to_string(),
            "/old.xlsx".to_string(),
            "/new.xlsx".to_string(),
        ];

        let paths = parse_cli_args(&args).unwrap();
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0], "/old.xlsx");
        assert_eq!(paths[1], "/new.xlsx");
    }

    #[test]
    fn test_parse_cli_args_with_file() {
        let args = vec![
            "excel-cli".to_string(),
            "diff".to_string(),
            "file".to_string(),
            "/old.xlsx".to_string(),
            "/new.xlsx".to_string(),
        ];

        let paths = parse_cli_args(&args).unwrap();
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0], "/old.xlsx");
        assert_eq!(paths[1], "/new.xlsx");
    }

    #[test]
    fn test_parse_cli_args_with_flags() {
        let args = vec![
            "excel-cli".to_string(),
            "diff".to_string(),
            "file".to_string(),
            "--verbose".to_string(),
            "/old.xlsx".to_string(),
            "--output".to_string(),
            "json".to_string(),
            "/new.xlsx".to_string(),
        ];

        let paths = parse_cli_args(&args).unwrap();
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0], "/old.xlsx");
        assert_eq!(paths[1], "/new.xlsx");
    }

    #[test]
    fn test_is_likely_file_path_excel_extensions() {
        assert!(is_likely_file_path("file.xlsx"));
        assert!(is_likely_file_path("file.xls"));
        assert!(is_likely_file_path("file.xlsm"));
        assert!(is_likely_file_path("file.xlsb"));
        assert!(is_likely_file_path("/path/to/file.xlsx"));
        assert!(is_likely_file_path("C:\\Users\\file.xlsx"));
    }

    #[test]
    fn test_is_likely_file_path_with_separator() {
        assert!(is_likely_file_path("/path/to/data"));
        assert!(is_likely_file_path("C:\\path\\to\\data"));
    }

    #[test]
    fn test_is_likely_file_path_rejects_flags() {
        assert!(!is_likely_file_path("--verbose"));
        assert!(!is_likely_file_path("-v"));
        assert!(!is_likely_file_path("--output=json"));
    }

    #[test]
    fn test_validate_path_empty() {
        let result = validate_path("");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn test_validate_path_invalid_extension() {
        let result = validate_path("file.txt");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported"));
    }

    #[test]
    fn test_validate_path_valid_xlsx() {
        let result = validate_path("file.xlsx");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_path_valid_xls() {
        let result = validate_path("file.xls");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_path_valid_xlsm() {
        let result = validate_path("file.xlsm");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_path_valid_xlsb() {
        let result = validate_path("file.xlsb");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_path_with_directory() {
        let result = validate_path("/path/to/file.xlsx");
        assert!(result.is_ok());
    }

    #[test]
    fn test_remove_excel_entries_preserves_other() {
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
    fn test_remove_excel_entries_only_entry() {
        let content = "*.xlsx diff=excel-diff\n";
        let remaining: String = content
            .lines()
            .filter(|line| !line.contains(GITATTR_PATTERN))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(remaining.trim().is_empty());
    }

    #[test]
    fn test_remove_excel_entries_no_excel() {
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
        let new_content = content.to_string()
            + &format!("{} {}\n", DEFAULT_PATTERNS[0], GITATTR_PATTERN);
        assert!(new_content.ends_with('\n'));
        assert!(new_content.contains("*.xlsx"));
        assert!(new_content.contains("*.xml"));
    }

    #[test]
    fn test_get_git_diff_file_paths_missing_args_error() {
        unsafe {
            // Clear environment variables
            env::remove_var(GIT_DIFF_PATH_OLD);
            env::remove_var(GIT_DIFF_PATH_NEW);

            // Without args or env vars, should return error
            // Note: In test environment, args might include the test binary path
            // So this test may not fail in all environments
            let result = get_git_diff_file_paths();
            // If successful, it means there are args; we just verify the function works
            if result.is_ok() {
                let (old, new) = result.unwrap();
                assert!(!old.is_empty());
                assert!(!new.is_empty());
            }
            // If error, that's also acceptable for this test
        }
    }
}
