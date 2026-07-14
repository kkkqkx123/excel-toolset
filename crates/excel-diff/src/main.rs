//! Standalone binary for git diff driver integration.
//!
//! This is a minimal binary that only handles diff operations needed by git.
//! The install/uninstall of the git diff driver is handled by shell scripts.
//!
//! Usage (called by git):
//!   excel-diff git-driver
//!
//! Git sets GIT_DIFF_PATH_OLD and GIT_DIFF_PATH_NEW environment variables
//! pointing to temporary copies of the old and new versions of the file.

use excel_diff::{diff_files, get_git_diff_file_paths, semantic};
use semantic::Verbosity;
use std::process;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Git diff driver invocation: excel-diff git-driver
    // Also support being called with file paths directly (for testing)
    if args.len() >= 2 && args[1] == "git-driver" {
        run_git_driver();
    } else {
        eprintln!("Usage: excel-diff git-driver");
        eprintln!();
        eprintln!("This binary is designed to be used as a git diff driver.");
        eprintln!("Run 'install-global.sh' to configure git globally.");
        process::exit(1);
    }
}

fn run_git_driver() {
    let (old_path, new_path) = match get_git_diff_file_paths() {
        Ok(paths) => paths,
        Err(e) => {
            eprintln!("Error: failed to get file paths: {}", e);
            process::exit(1);
        }
    };

    let diff = match diff_files(&old_path, &new_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error: failed to diff files: {}", e);
            process::exit(1);
        }
    };

    let text = semantic::to_natural_text(&diff, None, Verbosity::Detail);
    println!("{}", text);
}
