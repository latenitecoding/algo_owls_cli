use crate::common::{OwlError, Result};
use std::io::{BufReader, Read, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub fn bat_file(path: &Path) -> Result<()> {
    if !path.exists() {
        return Err(OwlError::FileError(
            format!("'{}': no such file", path.to_string_lossy()),
            "".into(),
        ));
    }

    if path.is_dir() {
        return Err(OwlError::ProcessError(
            format!("Failed to bat dir '{}'", path.to_string_lossy()),
            "cannot bat a dir".into(),
        ));
    }

    let mut child = Command::new("bat")
        .arg(path)
        .spawn()
        .map_err(|e| OwlError::ProcessError("[bat] failed to spawn".into(), e.to_string()))?;

    let status = child
        .wait()
        .map_err(|e| OwlError::ProcessError("[bat] not running".into(), e.to_string()))?;

    if status.success() {
        Ok(())
    } else {
        Err(OwlError::ProcessError(
            format!("Failed to bat file '{}'", path.to_string_lossy()),
            "status failed".into(),
        ))
    }
}

pub fn glow_file(path: &Path) -> Result<()> {
    if !path.exists() {
        return Err(OwlError::FileError(
            format!("'{}': no such file", path.to_string_lossy()),
            "".into(),
        ));
    }

    if path.is_dir() {
        return Err(OwlError::ProcessError(
            format!("Failed to glow dir '{}'", path.to_string_lossy()),
            "cannot glow a dir".into(),
        ));
    }

    let mut child = Command::new("glow")
        .arg(path)
        .spawn()
        .map_err(|e| OwlError::ProcessError("[glow] failed to spawn".into(), e.to_string()))?;

    let status = child
        .wait()
        .map_err(|e| OwlError::ProcessError("[glow] not running".into(), e.to_string()))?;

    if status.success() {
        Ok(())
    } else {
        Err(OwlError::ProcessError(
            format!("Failed to glow file '{}'", path.to_string_lossy()),
            "status failed".into(),
        ))
    }
}

pub fn run_binary(exe: &Path) -> Result<(String, Duration)> {
    let exe_str = exe.to_str().ok_or(OwlError::UriError(
        "Invalid binary file URI".into(),
        "None".into(),
    ))?;

    run_cmd("./binary", Command::new(format!("./{}", exe_str)))
}

pub fn run_binary_with_stdin(exe: &Path, input: &str) -> Result<(String, Duration)> {
    let exe_str = exe.to_str().ok_or(OwlError::UriError(
        "Invalid binary file URI".into(),
        "None".into(),
    ))?;

    run_cmd_with_stdin("./binary", Command::new(format!("./{}", exe_str)), input)
}

pub fn run_cmd(cmd_tag: &'static str, mut cmd: Command) -> Result<(String, Duration)> {
    let start = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("[run_cmd::start_time] unreachable");

    let child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            OwlError::ProcessError(format!("[{}] failed to spawn", cmd_tag), e.to_string())
        })?;

    stdout_else_stderr(cmd_tag, child).map(|stdout| {
        let stop = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("[run_cmd::stop_time] unreachable");

        (stdout, stop - start)
    })
}

pub fn run_cmd_with_stdin(
    cmd_tag: &'static str,
    mut cmd: Command,
    input: &str,
) -> Result<(String, Duration)> {
    let start = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("[run_cmd_with_stdin::start_time] unreachable");

    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            OwlError::ProcessError(format!("[{}] failed to spawn", cmd_tag), e.to_string())
        })?;

    let mut stdin = child.stdin.take().expect("[stdin handle] unreachable");
    let write_result = stdin.write_all(input.as_bytes()).map_err(|e| {
        OwlError::FileError(
            "Failed not write to stdin of child process".into(),
            e.to_string(),
        )
    });

    if let Err(e) = write_result {
        child.wait().map_err(|e| {
            OwlError::ProcessError(format!("[{}] not running", cmd_tag), e.to_string())
        })?;

        return Err(e);
    }

    stdout_else_stderr(cmd_tag, child).map(|stdout| {
        let stop = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("[run_cmd_with_stdin::stop_time] unreachable");

        (stdout, stop - start)
    })
}

pub fn stderr_only(cmd_tag: &'static str, mut child: Child) -> Result<String> {
    let stderr_pipe = child.stderr.take().expect("[stderr handle] unreachable");

    let status = child
        .wait()
        .map_err(|e| OwlError::ProcessError(format!("[{}] not running", cmd_tag), e.to_string()))?;

    let mut buffer = String::new();

    let mut reader = BufReader::new(stderr_pipe);
    reader.read_to_string(&mut buffer).map_err(|e| {
        OwlError::FileError(
            format!("'{}': failed to read stderr", cmd_tag),
            e.to_string(),
        )
    })?;

    if status.success() {
        Ok(buffer)
    } else {
        buffer.push_str("(run program manually for stack trace)");

        Err(OwlError::ProcessError(
            format!("'{}': exit with status failed", cmd_tag),
            buffer,
        ))
    }
}

pub fn stdout_else_stderr(cmd_tag: &'static str, mut child: Child) -> Result<String> {
    let stdout_pipe = child.stdout.take().expect("[stdout handle] unreachable");
    let stderr_pipe = child.stderr.take().expect("[stderr handle] unreachable");

    let status = child
        .wait()
        .map_err(|e| OwlError::ProcessError(format!("[{}] not running", cmd_tag), e.to_string()))?;

    if status.success() {
        let mut buffer = String::new();

        let mut reader = BufReader::new(stdout_pipe);
        reader.read_to_string(&mut buffer).map_err(|e| {
            OwlError::FileError(
                format!("'{}': failed to read stdout", cmd_tag),
                e.to_string(),
            )
        })?;

        Ok(buffer)
    } else {
        let mut buffer = String::new();

        let mut reader = BufReader::new(stderr_pipe);
        reader.read_to_string(&mut buffer).map_err(|e| {
            OwlError::FileError(
                format!("'{}': failed to read stderr", cmd_tag),
                e.to_string(),
            )
        })?;
        buffer.push_str("(run program manually for stack trace)");

        Err(OwlError::ProcessError(
            format!("'{}': exit with status failed", cmd_tag),
            buffer,
        ))
    }
}

pub fn tree_dir(dir: &Path) -> Result<()> {
    let mut child = Command::new("tree")
        .args(["-a", "-s", "-h", "--du", "-I", ".git"])
        .arg(dir)
        .spawn()
        .map_err(|e| OwlError::ProcessError("[tree] failed to spawn".into(), e.to_string()))?;

    let status = child
        .wait()
        .map_err(|e| OwlError::ProcessError("[tree] not running".into(), e.to_string()))?;
    if status.success() {
        Ok(())
    } else {
        Err(OwlError::ProcessError(
            format!("Failed to tree dir '{}'", dir.to_string_lossy()),
            "status failed".into(),
        ))
    }
}
