use std::ffi::OsStr;
use std::path::Path;
use std::process::Command;

use super::cmd_utils;
use super::owl_error::{OwlError, file_error, not_supported, program_error};

pub trait ProgLang {
    fn name(&self) -> &str;

    fn build(&self, filename: &str) -> Result<BuildLog, OwlError>;
    fn command_exists(&self) -> bool;
    fn run(&self, target: &str) -> Result<String, String>;
    fn run_with_stdin(&self, target: &str, input: &str) -> Result<String, String>;
    fn version(&self) -> Option<String>;
}

pub struct BuildLog {
    pub target: String,
    pub stdout: String,
}

pub fn get_prog_lang(lang: &str) -> Result<Box<dyn ProgLang>, OwlError> {
    match lang {
        "zig" => Ok(Box::new(ZigLang::new())),
        _ => Err(not_supported!(lang)),
    }
}

pub struct ZigLang {
    cmd: &'static str,
    ver_arg: &'static str,
}

impl ZigLang {
    pub fn new() -> Self {
        ZigLang {
            cmd: "zig",
            ver_arg: "version",
        }
    }
}

impl ProgLang for ZigLang {
    fn name(&self) -> &str {
        self.cmd
    }

    fn build(&self, filename: &str) -> Result<BuildLog, OwlError> {
        let output = Command::new(self.cmd)
            .arg("build-exe")
            .args(["-O", "ReleaseFast"])
            .arg(filename)
            .output()
            .expect("'zig build-exe' should be recognized");

        let stdout = String::from_utf8(output.stdout)
            .map_err(|e| file_error!(e))?
            .to_string();
        let stderr = String::from_utf8(output.stderr)
            .map_err(|e| file_error!(e))?
            .to_string();

        if output.status.success() {
            let target = Path::new(filename)
                .file_stem()
                .and_then(OsStr::to_str)
                .ok_or(file_error!(filename))?
                .to_owned();

            Ok(BuildLog { target, stdout })
        } else {
            Err(program_error!(stderr))
        }
    }

    fn command_exists(&self) -> bool {
        self.version().is_some()
    }

    fn run(&self, target: &str) -> Result<String, String> {
        cmd_utils::run_binary(target)
    }

    fn run_with_stdin(&self, target: &str, input: &str) -> Result<String, String> {
        cmd_utils::run_binary_with_stdin(target, input)
    }

    fn version(&self) -> Option<String> {
        let res = Command::new(self.cmd).arg(self.ver_arg).output();

        match res {
            Ok(output) => Some(String::from_utf8_lossy(&output.stdout).to_string()),
            Err(_) => None,
        }
    }
}
