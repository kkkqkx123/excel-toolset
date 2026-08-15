use super::*;

#[test]
fn test_diff_install_git_driver() {
    let r = run_json(&["diff", "install-git-driver"]);
    assert_ok(&r);
}

#[test]
fn test_diff_uninstall_git_driver() {
    let r = run_json(&["diff", "uninstall-git-driver"]);
    assert_ok(&r);
}
