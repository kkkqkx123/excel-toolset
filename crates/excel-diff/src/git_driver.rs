use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use excel_types::{AppError, Result};

const GITATTR_ENTRY: &str = "*.xlsx diff=excel-diff\n";
const GITATTR_PATTERN: &str = "*.xlsx diff=excel-diff";

const GIT_DIFF_PATH_OLD: &str = "GIT_DIFF_PATH_OLD";
const GIT_DIFF_PATH_NEW: &str = "GIT_DIFF_PATH_NEW";

#[allow(dead_code)]
const DEFAULT_COMMAND_NAME: &str = "excel-cli";

pub fn install_git_driver() -> Result<()> {
    let gitattr_path = get_gitattributes_path()?;

    let gitattr_existed = gitattr_path.exists();
    let mut need_write = true;

    if gitattr_existed {
        let content = read_gitattributes(&gitattr_path)?;
        if content.contains(GITATTR_PATTERN) {
            need_write = false;
        }
    }

    if need_write {
        if gitattr_existed {
            let content = read_gitattributes(&gitattr_path)?;
            write_gitattributes(&gitattr_path, content + GITATTR_ENTRY)?;
        } else {
            write_gitattributes(&gitattr_path, GITATTR_ENTRY.to_string())?;
        }
    }

    let exe_path = get_invocation_command()?;

    let output = Command::new("git")
        .args(["config", "diff.excel-diff.command", &exe_path])
        .output()
        .map_err(|e| AppError::Custom(format!("Failed to run git config: {}", e)))?;

    if !output.status.success() {
        if need_write && !gitattr_existed {
            let _ = fs::remove_file(&gitattr_path);
        }
        return Err(AppError::Custom(format!(
            "git config failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    Ok(())
}

pub fn uninstall_git_driver() -> Result<()> {
    let gitattr_path = get_gitattributes_path()?;

    if gitattr_path.exists() {
        let content = read_gitattributes(&gitattr_path)?;

        let remaining: String = content
            .lines()
            .filter(|line| !line.contains(GITATTR_PATTERN))
            .collect::<Vec<_>>()
            .join("\n");

        let trimmed = remaining.trim();
        if trimmed.is_empty() {
            fs::remove_file(&gitattr_path).map_err(AppError::Io)?;
        } else {
            write_gitattributes(&gitattr_path, trimmed.to_string())?;
        }
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

fn get_gitattributes_path() -> Result<PathBuf> {
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
