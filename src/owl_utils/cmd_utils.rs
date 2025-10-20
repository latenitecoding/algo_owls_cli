use crate::common::OwlError;
use std::io::{BufReader, Read, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub fn bat_file(path: &Path) -> Result<(), OwlError> {
    if !path.exists() {
        return Err(OwlError::FileError(
            format!("'{}': no such file", path.to_string_lossy()),
            "".into(),
        ));
    }

    if path.is_dir() {
        return Err(OwlError::ProcessError(
            format!("cannot bat dir '{}'", path.to_string_lossy()),
            "".into(),
        ));
    }

    let mut child = Command::new("bat")
        .arg(path)
        .spawn()
        .expect("[bat] failed to spawn");

    let status = child.wait().expect("[bat] not running");

    if status.success() {
        Ok(())
    } else {
        Err(OwlError::ProcessError(
            format!("could not bat file '{}'", path.to_string_lossy()),
            "".into(),
        ))
    }
}

pub fn glow_file(path: &Path) -> Result<(), OwlError> {
    if !path.exists() {
        return Err(OwlError::FileError(
            format!("'{}': no such file", path.to_string_lossy()),
            "".into(),
        ));
    }

    if path.is_dir() {
        return Err(OwlError::ProcessError(
            format!("cannot glow dir '{}'", path.to_string_lossy()),
            "".into(),
        ));
    }

    let mut child = Command::new("glow")
        .arg(path)
        .spawn()
        .expect("[glow] failed to spawn");

    let status = child.wait().expect("[glow] not running");

    if status.success() {
        Ok(())
    } else {
        Err(OwlError::ProcessError(
            format!("could not glow file '{}'", path.to_string_lossy()),
            "".into(),
        ))
    }
}

pub fn run_binary(exe: &Path) -> Result<(String, Duration), OwlError> {
    let exe_str = exe.to_str().ok_or(OwlError::UriError(
        "invalid binary file URI".into(),
        "".into(),
    ))?;

    run_cmd("./binary", Command::new(format!("./{}", exe_str)))
}

pub fn run_binary_with_stdin(exe: &Path, input: &str) -> Result<(String, Duration), OwlError> {
    let exe_str = exe.to_str().ok_or(OwlError::UriError(
        "invalid binary file URI".into(),
        "".into(),
    ))?;

    run_cmd_with_stdin("./binary", Command::new(format!("./{}", exe_str)), input)
}

pub fn run_cmd(cmd_tag: &'static str, mut cmd: Command) -> Result<(String, Duration), OwlError> {
    let start = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("[run_cmd::start_time] unreachable");

    let child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|_| panic!("[{}] failed to spawn", cmd_tag));

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
) -> Result<(String, Duration), OwlError> {
    let start = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("[run_cmd_with_stdin::start_time] unreachable");

    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|_| panic!("[{}] failed to spawn", cmd_tag));

    let mut stdin = child.stdin.take().expect("stdin handle");
    let write_result = stdin.write_all(input.as_bytes()).map_err(|e| {
        OwlError::FileError(
            "could not write to stdin of child process".into(),
            e.to_string(),
        )
    });

    if let Err(e) = write_result {
        child
            .wait()
            .unwrap_or_else(|_| panic!("[{}] not running", cmd_tag));

        return Err(e);
    }

    stdout_else_stderr(cmd_tag, child).map(|stdout| {
        let stop = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("[run_cmd_with_stdin::stop_time] unreachable");

        (stdout, stop - start)
    })
}

pub fn stderr_only(cmd_tag: &'static str, mut child: Child) -> Result<String, OwlError> {
    let stderr_pipe = child.stderr.take().expect("stderr handle");

    let status = child
        .wait()
        .unwrap_or_else(|_| panic!("[{}] not running", cmd_tag));

    let mut buffer = String::new();

    let mut reader = BufReader::new(stderr_pipe);
    reader.read_to_string(&mut buffer).map_err(|e| {
        OwlError::FileError(
            format!("'{}': could not read stderr", cmd_tag),
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

pub fn stdout_else_stderr(cmd_tag: &'static str, mut child: Child) -> Result<String, OwlError> {
    let stdout_pipe = child.stdout.take().expect("stdout handle");
    let stderr_pipe = child.stderr.take().expect("stderr handle");

    let status = child
        .wait()
        .unwrap_or_else(|_| panic!("[{}] not running", cmd_tag));

    if status.success() {
        let mut buffer = String::new();

        let mut reader = BufReader::new(stdout_pipe);
        reader.read_to_string(&mut buffer).map_err(|e| {
            OwlError::FileError(
                format!("'{}': could not read stdout", cmd_tag),
                e.to_string(),
            )
        })?;

        Ok(buffer)
    } else {
        let mut buffer = String::new();

        let mut reader = BufReader::new(stderr_pipe);
        reader.read_to_string(&mut buffer).map_err(|e| {
            OwlError::FileError(
                format!("'{}': could not read stderr", cmd_tag),
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

pub fn tree_dir(dir: &Path) -> Result<(), OwlError> {
    let mut child = Command::new("tree")
        .args(["-s", "-h", "--du"])
        .arg(dir)
        .spawn()
        .expect("[tree] failed to spawn");

    let status = child.wait().expect("[tree] not running");

    if status.success() {
        Ok(())
    } else {
        Err(OwlError::ProcessError(
            format!("could not tree '{}'", dir.to_string_lossy()),
            "".into(),
        ))
    }
}
