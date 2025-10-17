use std::io::{BufReader, Read, Write};
use std::process::{Child, Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use super::owl_error::{OwlError, file_error, program_error, time_error};

fn stderr_only(mut child: Child) -> Result<String, OwlError> {
    let stderr_pipe = child.stderr.take().ok_or(file_error!("stderr"))?;

    let status = child.wait().map_err(|e| program_error!(e))?;

    let mut buffer = String::new();

    let mut reader = BufReader::new(stderr_pipe);
    reader
        .read_to_string(&mut buffer)
        .map_err(|e| file_error!(e))?;

    if status.success() {
        Ok(buffer)
    } else {
        Err(program_error!(buffer))
    }
}

fn stdout_else_stderr(mut child: Child) -> Result<String, OwlError> {
    let stdout_pipe = child.stdout.take().ok_or(file_error!("stdout"))?;
    let stderr_pipe = child.stderr.take().ok_or(file_error!("stderr"))?;

    let status = child.wait().map_err(|e| program_error!(e))?;

    if status.success() {
        let mut buffer = String::new();

        let mut reader = BufReader::new(stdout_pipe);
        reader
            .read_to_string(&mut buffer)
            .map_err(|e| file_error!(e))?;

        Ok(buffer)
    } else {
        let mut buffer = String::new();

        let mut reader = BufReader::new(stderr_pipe);
        reader
            .read_to_string(&mut buffer)
            .map_err(|e| file_error!(e))?;

        Err(program_error!(buffer))
    }
}

pub fn bat_file(filepath: &str) -> Result<(), OwlError> {
    let mut child = Command::new("bat")
        .arg(filepath)
        .spawn()
        .map_err(|e| program_error!(e))?;

    let status = child.wait().map_err(|e| program_error!(e))?;

    if status.success() {
        Ok(())
    } else {
        Err(program_error!(format!("could not bat {}", filepath)))
    }
}

pub fn git_add(dir: &str) -> Result<String, OwlError> {
    let child = Command::new("git")
        .args(["add", "-A"])
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| program_error!(e))?;

    stdout_else_stderr(child)
}

pub fn git_checkout(dir: &str, branch: &str) -> Result<String, OwlError> {
    let child = Command::new("git")
        .args(["checkout", "-b", branch])
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| program_error!(e))?;

    stderr_only(child)
}

pub fn git_commit(dir: &str) -> Result<String, OwlError> {
    let child = Command::new("git")
        .args(["commit", "-m", "\"owlgo CLI submission\""])
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| program_error!(e))?;

    stdout_else_stderr(child)
}

pub fn git_fetch(dir: &str, remote: &str, branch: &str) -> Result<String, OwlError> {
    let child = Command::new("git")
        .args(["fetch", remote, branch])
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| program_error!(e))?;

    stderr_only(child)
}

pub fn git_init(dir: &str) -> Result<String, OwlError> {
    let child = Command::new("git")
        .arg("init")
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| program_error!(e))?;

    stdout_else_stderr(child)
}

pub fn git_remote_add(dir: &str, remote: &str, url: &str) -> Result<String, OwlError> {
    let child = Command::new("git")
        .args(["remote", "add", remote, url])
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| program_error!(e))?;

    stdout_else_stderr(child)?;

    let child = Command::new("git")
        .args(["remote", "-v"])
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| program_error!(e))?;

    stdout_else_stderr(child)
}

pub fn git_reset(dir: &str, remote: &str, branch: &str) -> Result<String, OwlError> {
    let child = Command::new("git")
        .args(["reset", "--hard", &format!("{}/{}", remote, branch)])
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| program_error!(e))?;

    stdout_else_stderr(child)
}

pub fn git_pull(dir: &str, remote: &str, branch: &str) -> Result<String, OwlError> {
    let child = Command::new("git")
        .args(["pull", remote, branch])
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| program_error!(e))?;

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
            .map_err(|e| program_error!(e))?
    } else {
        Command::new("git")
            .args(["push", "--set-upstream", remote, branch])
            .current_dir(dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| program_error!(e))?
    };

    stdout_else_stderr(child)
}

pub fn git_status(dir: &str) -> Result<String, OwlError> {
    let child = Command::new("git")
        .arg("status")
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| program_error!(e))?;

    stdout_else_stderr(child)
}

pub fn list_all(dir: &str) -> Result<(), OwlError> {
    let mut child = Command::new("tree")
        .args(["-s", "-h", "--du"])
        .arg(dir)
        .spawn()
        .map_err(|e| program_error!(e))?;

    let status = child.wait().map_err(|e| program_error!(e))?;

    if status.success() {
        Ok(())
    } else {
        Err(program_error!(format!("could not bat {}", dir)))
    }
}

pub fn run_cmd(mut cmd: Command) -> Result<(String, u128), OwlError> {
    let start = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| time_error!(e))?
        .as_millis();

    let child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| program_error!(e))?;

    stdout_else_stderr(child).and_then(|stdout| {
        let stop = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| time_error!(e))?
            .as_millis();

        Ok((stdout, stop - start))
    })
}

pub fn run_cmd_with_stdin(mut cmd: Command, input: &str) -> Result<(String, u128), OwlError> {
    let start = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| time_error!(e))?
        .as_millis();

    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| program_error!(e))?;

    let mut stdin = child.stdin.take().ok_or(file_error!("stdin"))?;
    stdin
        .write_all(input.as_bytes())
        .map_err(|e| file_error!(e))?;

    stdout_else_stderr(child).and_then(|stdout| {
        let stop = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| time_error!(e))?
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
