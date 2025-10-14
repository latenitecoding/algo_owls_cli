use std::io::{BufReader, Read, Write};
use std::process::{Child, Command, Stdio};

use super::owl_error::{OwlError, file_error, program_error};

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

pub fn run_cmd(mut cmd: Command) -> Result<String, OwlError> {
    let child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| program_error!(e))?;

    stdout_else_stderr(child)
}

pub fn run_cmd_with_stdin(mut cmd: Command, input: &str) -> Result<String, OwlError> {
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

    stdout_else_stderr(child)
}

pub fn run_binary(exe: &str) -> Result<String, OwlError> {
    let cmd = Command::new(format!("./{}", exe));
    run_cmd(cmd)
}

pub fn run_binary_with_stdin(exe: &str, input: &str) -> Result<String, OwlError> {
    let cmd = Command::new(format!("./{}", exe));
    run_cmd_with_stdin(cmd, input)
}
