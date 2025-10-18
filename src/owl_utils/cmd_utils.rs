use std::io::{BufReader, Read, Write};
use std::process::{Child, Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use super::owl_error::{OwlError, process_error, time_error};

fn stderr_only(mut child: Child) -> Result<String, OwlError> {
    let stderr_pipe = child
        .stderr
        .take()
        .ok_or(process_error!("stderr_only::take_stderr_pipe", "stderr"))?;

    let status = child
        .wait()
        .map_err(|e| process_error!("stderr_only::wait_on_program", e))?;

    let mut buffer = String::new();

    let mut reader = BufReader::new(stderr_pipe);
    reader
        .read_to_string(&mut buffer)
        .map_err(|e| process_error!("stderr_only::read_stderr", e))?;

    if status.success() {
        Ok(buffer)
    } else {
        Err(process_error!("stderr_only::status_failed", buffer))
    }
}

fn stdout_else_stderr(mut child: Child) -> Result<String, OwlError> {
    let stdout_pipe = child.stdout.take().ok_or(process_error!(
        "stdout_else_stderr::take_stdout_pipe",
        "stdout"
    ))?;
    let stderr_pipe = child.stderr.take().ok_or(process_error!(
        "stdout_else_stderr::take_stderr_pipe",
        "stderr"
    ))?;

    let status = child
        .wait()
        .map_err(|e| process_error!("stdout_else_stderr::wait_on_program", e))?;

    if status.success() {
        let mut buffer = String::new();

        let mut reader = BufReader::new(stdout_pipe);
        reader
            .read_to_string(&mut buffer)
            .map_err(|e| process_error!("stdout_else_stderr::read_stdout", e))?;

        Ok(buffer)
    } else {
        let mut buffer = String::new();

        let mut reader = BufReader::new(stderr_pipe);
        reader
            .read_to_string(&mut buffer)
            .map_err(|e| process_error!("stdout_else_stderr::read_stderr", e))?;

        Err(process_error!("stdout_else_stderr::status_failed", buffer))
    }
}

pub fn bat_file(filepath: &str) -> Result<(), OwlError> {
    let mut child = Command::new("bat")
        .arg(filepath)
        .spawn()
        .map_err(|e| process_error!("bat_file::spawn", e))?;

    let status = child
        .wait()
        .map_err(|e| process_error!("bat_file::wait_on_program", e))?;

    if status.success() {
        Ok(())
    } else {
        Err(process_error!(
            "bat_file::status_failed",
            format!("could not bat {}", filepath)
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
        .map_err(|e| process_error!("git_add::spawn", e))?;

    stdout_else_stderr(child)
}

pub fn git_checkout(dir: &str, branch: &str) -> Result<String, OwlError> {
    let child = Command::new("git")
        .args(["checkout", "-b", branch])
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| process_error!("git_checkout::spawn", e))?;

    stderr_only(child)
}

pub fn git_commit(dir: &str) -> Result<String, OwlError> {
    let child = Command::new("git")
        .args(["commit", "-m", "\"owlgo CLI submission\""])
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| process_error!("git_commit::spawn", e))?;

    stdout_else_stderr(child)
}

pub fn git_fetch(dir: &str, remote: &str, branch: &str) -> Result<String, OwlError> {
    let child = Command::new("git")
        .args(["fetch", remote, branch])
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| process_error!("git_fetch::spawn", e))?;

    stderr_only(child)
}

pub fn git_init(dir: &str) -> Result<String, OwlError> {
    let child = Command::new("git")
        .arg("init")
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| process_error!("git_init::spawn", e))?;

    stdout_else_stderr(child)
}

pub fn git_pull(dir: &str, remote: &str, branch: &str) -> Result<String, OwlError> {
    let child = Command::new("git")
        .args(["pull", remote, branch])
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| process_error!("git_pull::spawn", e))?;

    stdout_else_stderr(child)
}

pub fn git_push(dir: &str, remote: &str, branch: &str, force: bool) -> Result<String, OwlError> {
    let child = if force {
        Command::new("git")
            .args(["push", "-f", "--set-upstream", remote, branch])
            .current_dir(dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| process_error!("git_push::spawn_force", e))?
    } else {
        Command::new("git")
            .args(["push", "--set-upstream", remote, branch])
            .current_dir(dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| process_error!("git_push::spawn", e))?
    };

    stdout_else_stderr(child)
}

pub fn git_remote_add(dir: &str, remote: &str, url: &str) -> Result<String, OwlError> {
    let child = Command::new("git")
        .args(["remote", "add", remote, url])
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| process_error!("git_remote_add::spawn", e))?;

    stdout_else_stderr(child)?;

    let child = Command::new("git")
        .args(["remote", "-v"])
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| process_error!("git_remote_add::spawn_verbose", e))?;

    stdout_else_stderr(child)
}

pub fn git_reset(dir: &str, remote: &str, branch: &str) -> Result<String, OwlError> {
    let child = Command::new("git")
        .args(["reset", "--hard", &format!("{}/{}", remote, branch)])
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| process_error!("git_reset::spawn", e))?;

    stdout_else_stderr(child)
}

pub fn git_status(dir: &str) -> Result<String, OwlError> {
    let child = Command::new("git")
        .arg("status")
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| process_error!("git_status::spawn", e))?;

    stdout_else_stderr(child)
}

pub fn list_all(dir: &str) -> Result<(), OwlError> {
    let mut child = Command::new("tree")
        .args(["-s", "-h", "--du"])
        .arg(dir)
        .spawn()
        .map_err(|e| process_error!("list_all::spawn", e))?;

    let status = child
        .wait()
        .map_err(|e| process_error!("list_all::wait_on_program", e))?;

    if status.success() {
        Ok(())
    } else {
        Err(process_error!(
            "list_all::failed_status",
            format!("could not tree {}", dir)
        ))
    }
}

pub fn run_cmd(mut cmd: Command) -> Result<(String, u128), OwlError> {
    let start = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| time_error!("run_cmd::start_time", e))?
        .as_millis();

    let child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| process_error!("run_cmd::spawn", e))?;

    stdout_else_stderr(child).and_then(|stdout| {
        let stop = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| time_error!("run_cmd::stop_time", e))?
            .as_millis();

        Ok((stdout, stop - start))
    })
}

pub fn run_cmd_with_stdin(mut cmd: Command, input: &str) -> Result<(String, u128), OwlError> {
    let start = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| time_error!("run_cmd_with_stdin::start_time", e))?
        .as_millis();

    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| process_error!("run_cmd_with_stdin::spawn", e))?;

    let mut stdin = child.stdin.take().ok_or(process_error!(
        "run_cmd_with_stdin::take_stdin_pipe",
        "stdin"
    ))?;
    stdin
        .write_all(input.as_bytes())
        .map_err(|e| process_error!("run_cmd_with_stdin::write_stdin", e))?;

    stdout_else_stderr(child).and_then(|stdout| {
        let stop = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| time_error!("run_cmd_with_stdin::stop_time", e))?
            .as_millis();

        Ok((stdout, stop - start))
    })
}

pub fn run_binary(exe: &str) -> Result<(String, u128), OwlError> {
    let cmd = Command::new(format!("./{}", exe));
    run_cmd(cmd)
}

pub fn run_binary_with_stdin(exe: &str, input: &str) -> Result<(String, u128), OwlError> {
    let cmd = Command::new(format!("./{}", exe));
    run_cmd_with_stdin(cmd, input)
}
