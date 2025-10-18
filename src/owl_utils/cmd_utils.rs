use std::io::{BufReader, Read, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use super::owl_error::{OwlError, bad_chars, file_error, file_not_found, process_error};

fn stderr_only(cmd_tag: &'static str, mut child: Child) -> Result<String, OwlError> {
    let stderr_pipe = child.stderr.take().expect("stderr handle");

    let status = child
        .wait()
        .unwrap_or_else(|_| panic!("[{}] not running", cmd_tag));

    let mut buffer = String::new();

    let mut reader = BufReader::new(stderr_pipe);
    reader
        .read_to_string(&mut buffer)
        .map_err(|_| bad_chars!(&format!("{}; stderr", cmd_tag)))?;

    if status.success() {
        Ok(buffer)
    } else {
        Err(process_error!(cmd_tag, buffer))
    }
}

fn stdout_else_stderr(cmd_tag: &'static str, mut child: Child) -> Result<String, OwlError> {
    let stdout_pipe = child.stdout.take().expect("stdout handle");
    let stderr_pipe = child.stderr.take().expect("stderr handle");

    let status = child
        .wait()
        .unwrap_or_else(|_| panic!("[{}] not running", cmd_tag));

    if status.success() {
        let mut buffer = String::new();

        let mut reader = BufReader::new(stdout_pipe);
        reader
            .read_to_string(&mut buffer)
            .map_err(|_| bad_chars!(&format!("{}; stdout", cmd_tag)))?;

        Ok(buffer)
    } else {
        let mut buffer = String::new();

        let mut reader = BufReader::new(stderr_pipe);
        reader
            .read_to_string(&mut buffer)
            .map_err(|_| bad_chars!(&format!("{}; stderr", cmd_tag)))?;

        buffer.push_str("(run program manually for stack trace)");

        Err(process_error!(cmd_tag, buffer))
    }
}

pub fn bat_file(filepath: &str) -> Result<(), OwlError> {
    let path = Path::new(filepath);

    if !path.exists() {
        return Err(file_not_found!("bat_file::check_file", filepath));
    }

    let mut child = Command::new("bat")
        .arg(filepath)
        .spawn()
        .expect("[bat] failed to spawn");

    let status = child.wait().expect("[bat] not running");

    if status.success() {
        Ok(())
    } else {
        Err(process_error!(
            "bat",
            format!("could not bat '{}'", filepath)
        ))
    }
}

pub fn git_add(dir: &str) -> Result<String, OwlError> {
    let child = Command::new("git")
        .args(["add", "-A"])
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("[git add] failed to spawn");

    stdout_else_stderr("git add -A", child)
}

pub fn git_checkout(dir: &str, branch: &str) -> Result<String, OwlError> {
    let child = Command::new("git")
        .args(["checkout", "-b", branch])
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("[git checkout] failed to spawn");

    stderr_only("git checkout", child)
}

pub fn git_commit(dir: &str) -> Result<String, OwlError> {
    let child = Command::new("git")
        .args(["commit", "-m", "\"owlgo CLI submission\""])
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("[git commit] failed to spawn");

    stdout_else_stderr("git commit", child)
}

pub fn git_fetch(dir: &str, remote: &str, branch: &str) -> Result<String, OwlError> {
    let child = Command::new("git")
        .args(["fetch", remote, branch])
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("[git fetch] failed to spawn");

    stderr_only("git fetch", child)
}

pub fn git_init(dir: &str) -> Result<String, OwlError> {
    let child = Command::new("git")
        .arg("init")
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("[git init] failed to spawn");

    stdout_else_stderr("git init", child)
}

pub fn git_pull(dir: &str, remote: &str, branch: &str) -> Result<String, OwlError> {
    let child = Command::new("git")
        .args(["pull", remote, branch])
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("[git pull] failed to spawn");

    stdout_else_stderr("git pull", child)
}

pub fn git_push(dir: &str, remote: &str, branch: &str, force: bool) -> Result<String, OwlError> {
    let child = if force {
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

    stdout_else_stderr("git push", child)
}

pub fn git_remote_add(dir: &str, remote: &str, url: &str) -> Result<String, OwlError> {
    let child = Command::new("git")
        .args(["remote", "add", remote, url])
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("[git remote add] failed to spawn");

    stdout_else_stderr("git remote add", child)?;

    let child = Command::new("git")
        .args(["remote", "-v"])
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("[git remote -v] failed to spawn");

    stdout_else_stderr("git remote -v", child)
}

pub fn git_reset(dir: &str, remote: &str, branch: &str) -> Result<String, OwlError> {
    let child = Command::new("git")
        .args(["reset", "--hard", &format!("{}/{}", remote, branch)])
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("[git reset] failed to spawn");

    stdout_else_stderr("git reset", child)
}

pub fn git_status(dir: &str) -> Result<String, OwlError> {
    let child = Command::new("git")
        .arg("status")
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("[git status] failed to spawn");

    stdout_else_stderr("git status", child)
}

pub fn glow_file(filepath: &str) -> Result<(), OwlError> {
    let path = Path::new(filepath);

    if !path.exists() {
        return Err(file_not_found!("glow_file::check_file", filepath));
    }

    let mut child = Command::new("glow")
        .arg(filepath)
        .spawn()
        .expect("[glow] failed to spawn");

    let status = child.wait().expect("[glow] not running");

    if status.success() {
        Ok(())
    } else {
        Err(process_error!(
            "glow",
            format!("could not glow '{}'", filepath)
        ))
    }
}

pub fn list_all(dir: &str) -> Result<(), OwlError> {
    let mut child = Command::new("tree")
        .args(["-s", "-h", "--du"])
        .arg(dir)
        .spawn()
        .expect("[tree] failed to spawn");

    let status = child.wait().expect("[tree] not running");

    if status.success() {
        Ok(())
    } else {
        Err(process_error!("tree", format!("could not tree '{}'", dir)))
    }
}

pub fn run_cmd(cmd_tag: &'static str, mut cmd: Command) -> Result<(String, u128), OwlError> {
    let start = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("[run_cmd::start_time] unreachable")
        .as_millis();

    let child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|_| panic!("[{}] failed to spawn", cmd_tag));

    stdout_else_stderr(cmd_tag, child).map(|stdout| {
        let stop = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("[run_cmd::stop_time] unreachable")
            .as_millis();

        (stdout, stop - start)
    })
}

pub fn run_cmd_with_stdin(
    cmd_tag: &'static str,
    mut cmd: Command,
    input: &str,
) -> Result<(String, u128), OwlError> {
    let start = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("[run_cmd_with_stdin::start_time] unreachable")
        .as_millis();

    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|_| panic!("[{}] failed to spawn", cmd_tag));

    let mut stdin = child.stdin.take().expect("stdin handle");
    let write_result = stdin
        .write_all(input.as_bytes())
        .map_err(|e| file_error!("run_cmd_with_stdin::stdin_write", e));

    if let Err(e) = write_result {
        child
            .wait()
            .unwrap_or_else(|_| panic!("[{}] not running", cmd_tag));

        return Err(e);
    }

    stdout_else_stderr(cmd_tag, child).map(|stdout| {
        let stop = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("[run_cmd_with_stdin::stop_time] unreachable")
            .as_millis();

        (stdout, stop - start)
    })
}

pub fn run_binary(exe: &str) -> Result<(String, u128), OwlError> {
    let cmd = Command::new(format!("./{}", exe));
    run_cmd("./binary", cmd)
}

pub fn run_binary_with_stdin(exe: &str, input: &str) -> Result<(String, u128), OwlError> {
    let cmd = Command::new(format!("./{}", exe));
    run_cmd_with_stdin("./binary", cmd, input)
}
