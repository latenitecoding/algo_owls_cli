use std::process::Command;

pub trait ProgLang {
    fn name(&self) -> &str;
    fn command_exists(&self) -> bool;
    fn version(&self) -> Option<String>;
}

pub fn get_prog_lang(lang: &str) -> Result<Box<dyn ProgLang>, String> {
    match lang {
        "zig" => Ok(Box::new(ZigLang::new())),
        _ => Err(format!("Unrecognized programming language: {}", lang)),
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

    fn command_exists(&self) -> bool {
        self.version().is_some()
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
