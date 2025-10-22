use super::cmd_utils;
use crate::common::OwlError;
use std::path::Path;
use std::process::{Command, Stdio};

pub fn git_add(dir: &Path) -> Result<String, OwlError> {
    let child = Command::new("git")
        .args(["add", "-A"])
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("[git add] failed to spawn");

    cmd_utils::stdout_else_stderr("git add -A", child)
}

pub fn git_checkout(dir: &Path, branch: &str) -> Result<String, OwlError> {
    let child = Command::new("git")
        .args(["checkout", "-b", branch])
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("[git checkout] failed to spawn");

    cmd_utils::stderr_only("git checkout", child)
}

pub fn git_commit(dir: &Path) -> Result<String, OwlError> {
    let child = Command::new("git")
        .args(["commit", "-m", "\"owlgo CLI submission\""])
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("[git commit] failed to spawn");

    cmd_utils::stdout_else_stderr("git commit", child)
}

pub fn git_fetch(dir: &Path, remote: &str, branch: &str) -> Result<String, OwlError> {
    let child = Command::new("git")
        .args(["fetch", remote, branch])
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("[git fetch] failed to spawn");

    cmd_utils::stderr_only("git fetch", child)
}

pub fn git_init(dir: &Path) -> Result<String, OwlError> {
    let child = Command::new("git")
        .arg("init")
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("[git init] failed to spawn");

    cmd_utils::stdout_else_stderr("git init", child)
}

pub fn git_pull(dir: &Path, remote: &str, branch: &str) -> Result<String, OwlError> {
    let child = Command::new("git")
        .args(["pull", remote, branch])
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("[git pull] failed to spawn");

    cmd_utils::stdout_else_stderr("git pull", child)
}

pub fn git_push(
    dir: &Path,
    remote: &str,
    branch: &str,
    use_force: bool,
) -> Result<String, OwlError> {
    let child = if use_force {
        Command::new("git")
            .args(["push", "-f", "--set-upstream", remote, branch])
            .current_dir(dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("[git push -f] failed to spawn")
    } else {
        Command::new("git")
            .args(["push", "--set-upstream", remote, branch])
            .current_dir(dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("[git push] failed to spawn")
    };

    cmd_utils::stdout_else_stderr("git push", child)
}

pub fn git_remote_add(dir: &Path, remote: &str, url: &str) -> Result<String, OwlError> {
    let child = Command::new("git")
        .args(["remote", "add", remote, url])
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("[git remote add] failed to spawn");

    cmd_utils::stdout_else_stderr("git remote add", child)?;

    let child = Command::new("git")
        .args(["remote", "-v"])
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("[git remote -v] failed to spawn");

    cmd_utils::stdout_else_stderr("git remote -v", child)
}

pub fn git_reset(dir: &Path, remote: &str, branch: &str) -> Result<String, OwlError> {
    let child = Command::new("git")
        .args(["reset", "--hard", &format!("{}/{}", remote, branch)])
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("[git reset] failed to spawn");

    cmd_utils::stdout_else_stderr("git reset", child)
}

pub fn git_status(dir: &Path) -> Result<String, OwlError> {
    let child = Command::new("git")
        .arg("status")
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("[git status] failed to spawn");

    cmd_utils::stdout_else_stderr("git status", child)
}
