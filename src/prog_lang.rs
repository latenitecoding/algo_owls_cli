use crossbeam_channel::{select, unbounded};
use std::ffi::OsStr;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;

pub trait ProgLang {
    fn name(&self) -> &str;

    fn build(&self, filename: &str) -> Result<String, String>;
    fn command_exists(&self) -> bool;
    fn run(&self, exe: &str) -> Result<String, String>;
    fn version(&self) -> Option<String>;
}

pub fn get_prog_lang(lang: &str) -> Result<Box<dyn ProgLang>, String> {
    match lang {
        "zig" => Ok(Box::new(ZigLang::new())),
        _ => Err(format!("Unrecognized programming language: {}", lang)),
    }
}

pub fn run_binary(exe: &str) -> Result<String, String> {
    let cmd = Command::new(format!("./{}", exe));
    run_cmd(cmd)
}

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
        Err(merged_output)
    }
}

pub struct ZigLang {
    cmd: &'static str,
    ver_arg: &'static str,
}

impl ZigLang {
    pub fn new() -> Self {
        ZigLang { cmd: "zig", ver_arg: "version" }
    }
}

impl ProgLang for ZigLang {
    fn name(&self) -> &str {
        self.cmd
    }

    fn build(&self, filename: &str) -> Result<String, String> {
        let output = Command::new(self.cmd)
            .arg("build-exe")
            .args(["-O", "ReleaseFast"])
            .arg(filename)
            .output()
            .expect("'zig build-exe' should be recognized");
        if output.status.success() {
            println!("{}", String::from_utf8_lossy(&output.stdout));
            let stem = Path::new(filename)
                .file_stem()
                .and_then(OsStr::to_str)
                .expect("file should exist");
            Ok(stem.to_string())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }

    fn command_exists(&self) -> bool {
        self.version().is_some()
    }

    fn run(&self, exe: &str) -> Result<String, String> {
        run_binary(exe)
    }

    fn version(&self) -> Option<String> {
        let res = Command::new(self.cmd)
            .arg(self.ver_arg)
            .output();
        match res {
            Ok(output) => {
                Some(String::from_utf8_lossy(&output.stdout).to_string())
            },
            Err(_) => None,
        }
    }
}
