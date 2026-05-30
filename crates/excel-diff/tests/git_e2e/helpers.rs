use std::path::Path;
use std::process::{Command, Output};
use std::sync::Mutex;

static GIT_LOCK: Mutex<()> = Mutex::new(());

/// Run a closure with CWD set to a temp git repo.
/// The mutex serialises all git tests so CWD changes never collide.
/// `catch_unwind` guarantees CWD and mutex are cleaned up even on panic.
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

pub fn git(args: &[&str]) -> Output {
    Command::new("git").args(args).output().unwrap()
}

pub fn file_exists(path: &Path) -> bool {
    path.exists()
}

/// Create a temp dir and return (dir, fn that cleans up).
/// Does NOT change CWD — caller manages that.
pub fn temp_git_dir() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_path_buf();

    let output = Command::new("git")
        .args(["init"])
        .current_dir(&path)
        .output()
        .unwrap();
    assert!(output.status.success(), "git init failed in {}", path.display());

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