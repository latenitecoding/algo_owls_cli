use crossbeam_channel::{select, unbounded};
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::thread;

pub fn run_cmd(mut cmd: Command) -> Result<String, String> {
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("should spawn child process");

    let stdout_pipe = child.stdout.take().expect("should take stdout");
    let stderr_pipe = child.stderr.take().expect("should take stderr");

    let (tx1, rx1) = unbounded();
    let (tx2, rx2) = unbounded();

    thread::spawn(move || {
        let reader = BufReader::new(stdout_pipe);
        for line in reader.lines().map_while(Result::ok) {
            tx1.send(line).expect("should send line of stdout");
        }
    });

    thread::spawn(move || {
        let reader = BufReader::new(stderr_pipe);
        for line in reader.lines().map_while(Result::ok) {
            tx2.send(line).expect("should send line of stderr");
        }
    });

    let mut merged_output = String::new();
    select! {
        recv(rx1) -> out => {
            merged_output
                .push_str(&out.expect("should receive from stdout"));
        },
        recv(rx2) -> out => {
            merged_output
                .push_str(&out.expect("should receive from stderr"));
        },
    }

    let status = child.wait().expect("child process should stop");

    if status.success() {
        Ok(merged_output)
    } else {
        merged_output.insert_str(0, "child process failed\n\nPROGRAM LOG:\n");
        Err(merged_output)
    }
}

pub fn run_binary(exe: &str) -> Result<String, String> {
    let cmd = Command::new(format!("./{}", exe));
    run_cmd(cmd)
}
