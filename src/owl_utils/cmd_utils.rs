use std::io::{BufReader, Read, Write};
use std::process::{Command, Stdio};

fn stdout_else_stderr(mut child: std::process::Child) -> Result<String, String> {
    let stdout_pipe = child.stdout.take().expect("should take stdout");
    let stderr_pipe = child.stderr.take().expect("should take stderr");

    let status = child.wait().expect("child process should stop");

    if status.success() {
        let mut buffer = String::new();

        let mut reader = BufReader::new(stdout_pipe);
        reader.read_to_string(&mut buffer).map_err(|e| e.to_string())?;

        Ok(buffer)
    } else {
        let mut buffer = String::new();
        buffer.push_str("child process failed\n\nPROGRAM LOG:\n");

        let mut reader = BufReader::new(stderr_pipe);
        reader.read_to_string(&mut buffer).map_err(|e| e.to_string())?;

        Err(buffer)
    }
}

pub fn run_cmd(mut cmd: Command) -> Result<String, String> {
    let child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("should spawn child process");

    stdout_else_stderr(child)
}

pub fn run_cmd_with_stdin(mut cmd: Command, input: &str) -> Result<String, String> {
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("should spawn child process");

    let mut stdin = child.stdin.take().expect("should take stdin");
    stdin.write_all(input.as_bytes()).expect("should write to stdin");

    stdout_else_stderr(child)
}

pub fn run_binary(exe: &str) -> Result<String, String> {
    let cmd = Command::new(format!("./{}", exe));
    run_cmd(cmd)
}

pub fn run_binary_with_stdin(exe: &str, input: &str) -> Result<String, String> {
    let cmd = Command::new(format!("./{}", exe));
    run_cmd_with_stdin(cmd, input)
}
