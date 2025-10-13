use std::ffi::OsStr;
use std::path::Path;
use std::process::Command;

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
    let output = Command::new(format!("./{}", exe))
        .output()
        .expect("should be able to execute binary file");
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.is_empty() {
            return Ok(stdout.to_string());
        }
        if stdout.is_empty() {
            return Ok(
                format!(
                    "================ DEBUG ================\n{}",
                    stderr,
                )
            );
        }
        Ok(
            format!(
                "{}\n\n================ DEBUG ================\n{}",
                stdout,
                stderr
            )
        )
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
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
