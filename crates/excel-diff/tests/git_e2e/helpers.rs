use std::path::Path;
use std::process::{Command, Output};
use std::sync::Mutex;

/// Global lock to serialise tests that change process CWD.
///
/// This mutex is required because `git_driver::install_git_driver()` and
/// `git_driver::uninstall_git_driver()` locate the git repository root by
/// walking up from `std::env::current_dir()`. Changing CWD is currently the
/// only way to test these functions without modifying the library API.
///
/// Tests that do not invoke install/uninstall should use `tempfile::tempdir()`
/// directly and avoid this lock.
static GIT_LOCK: Mutex<()> = Mutex::new(());

/// Run a closure with CWD set to a newly initialised temp git repo.
///
/// The global `GIT_LOCK` mutex serialises all callers so that CWD changes
/// never collide. `catch_unwind` guarantees CWD and the mutex are restored
/// even if the closure panics.
///
/// Prefer `tempfile::tempdir()` + `Command::current_dir()` for tests that
/// do not strictly require a CWD change.
pub fn with_git_repo<F, T>(f: F) -> T
where
    F: FnOnce() -> T,
{
    let lock = GIT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    let original = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();

    let output = Command::new("git").args(["init"]).output().unwrap();
    assert!(
        output.status.success(),
        "git init failed in {}:\n{}",
        dir.path().display(),
        String::from_utf8_lossy(&output.stderr)
    );

    Command::new("git")
        .args(["config", "user.email", "e2e@test.dev"])
        .output()
        .ok();
    Command::new("git")
        .args(["config", "user.name", "E2E Test"])
        .output()
        .ok();
    Command::new("git")
        .args(["config", "commit.gpgsign", "false"])
        .output()
        .ok();

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

    std::env::set_current_dir(&original).ok();
    drop(lock);

    match result {
        Ok(v) => v,
        Err(e) => std::panic::resume_unwind(e),
    }
}

/// Run a git command in the current working directory.
///
/// Only valid inside a `with_git_repo` closure (or when CWD is already a
/// git repo managed by the test).
pub fn git(args: &[&str]) -> Output {
    Command::new("git").args(args).output().unwrap()
}

pub fn file_exists(path: &Path) -> bool {
    path.exists()
}

/// Create a temp directory, initialise git inside it, and return both the
/// `TempDir` handle and the directory path. Does NOT change CWD.
///
/// Use this helper for tests that only need a git repo directory but do not
/// need the global CWD to point at it.
#[allow(dead_code)]
pub fn temp_git_dir() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_path_buf();

    let output = Command::new("git")
        .args(["init"])
        .current_dir(&path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git init failed in {}",
        path.display()
    );

    Command::new("git")
        .args(["config", "user.email", "e2e@test.dev"])
        .current_dir(&path)
        .output()
        .ok();
    Command::new("git")
        .args(["config", "user.name", "E2E Test"])
        .current_dir(&path)
        .output()
        .ok();
    Command::new("git")
        .args(["config", "commit.gpgsign", "false"])
        .current_dir(&path)
        .output()
        .ok();

    (dir, path)
}
