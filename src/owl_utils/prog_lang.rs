use std::ffi::OsStr;
use std::path::Path;
use std::process::Command;

use super::cmd_utils;
use super::owl_error::{OwlError, file_error, not_supported, program_error};

pub trait ProgLang {
    fn build_cmd(&self, filename: &str) -> Result<Command, OwlError>;
    fn build_files(&self, target_stem: &str) -> Option<Vec<String>>;
    fn name(&self) -> &str;
    fn run_it(&self, target: &str, stdin: Option<&str>) -> Result<(String, u128), OwlError>;
    fn should_build(&self) -> bool;
    fn target_name(&self, target_stem: &str) -> String;
    fn version_cmd(&self) -> Result<Command, OwlError>;

    fn build(&self, filename: &str) -> Result<BuildLog, OwlError> {
        let output = self
            .build_cmd(filename)?
            .output()
            .map_err(|e| program_error!(e))?;

        let stdout = String::from_utf8(output.stdout)
            .map_err(|e| file_error!(e))?
            .to_string();
        let stderr = String::from_utf8(output.stderr)
            .map_err(|e| file_error!(e))?
            .to_string();

        if output.status.success() {
            let target_stem = Path::new(filename)
                .file_stem()
                .and_then(OsStr::to_str)
                .ok_or(file_error!(filename))?;
            let target = self.target_name(target_stem);

            let build_files = self.build_files(target_stem);

            Ok(BuildLog {
                target,
                stdout,
                build_files,
            })
        } else {
            Err(program_error!(stderr))
        }
    }

    fn command_exists(&self) -> bool {
        self.version().is_ok()
    }

    fn version(&self) -> Result<String, OwlError> {
        let output = self
            .version_cmd()?
            .output()
            .map_err(|e| program_error!(e))?;

        if output.status.success() {
            Ok(String::from_utf8(output.stdout)
                .map_err(|e| file_error!(e))?
                .to_string())
        } else {
            Err(program_error!(format!(
                "Unable to determine version of '{}'",
                self.name()
            )))
        }
    }

    fn run(&self, target: &str) -> Result<(String, u128), OwlError> {
        self.run_it(target, None)
    }

    fn run_with_stdin(&self, target: &str, input: &str) -> Result<(String, u128), OwlError> {
        self.run_it(target, Some(input))
    }
}

pub struct BuildLog {
    pub target: String,
    pub stdout: String,
    pub build_files: Option<Vec<String>>,
}

pub fn check_prog_lang(prog: &str) -> Option<Box<dyn ProgLang>> {
    Path::new(prog)
        .extension()
        .and_then(OsStr::to_str)
        .and_then(|ext| get_prog_lang(ext).ok())
}

pub fn get_prog_lang(lang_ext: &str) -> Result<Box<dyn ProgLang>, OwlError> {
    match lang_ext {
        "adb" | "ads" => {
            let ada_lang = ComptimeLang {
                name: "ada",
                cmd_str: "gnatmake",
                ver_arg: "--version",
                build_cmd_str: "gnatmake",
                build_args: &["-g", "-O2"],
                exe_flag: Some(("-o", ArgsPosition::Pre)),
                fn_build_files: Some(|target_stem| {
                    vec![
                        format!("b~{}.adb", target_stem),
                        format!("b~{}.ads", target_stem),
                        format!("b~{}.ali", target_stem),
                        format!("b~{}.o", target_stem),
                        format!("{}.ali", target_stem),
                        format!("{}.o", target_stem),
                    ]
                }),
            };
            Ok(Box::new(ada_lang))
        }
        "c" => {
            let c_lang = ComptimeLang {
                name: "c",
                cmd_str: "gcc",
                ver_arg: "--version",
                build_cmd_str: "gcc",
                build_args: &["-g", "-O2", "-std=gnu23", "-static", "-lm"],
                exe_flag: Some(("-o", ArgsPosition::Pre)),
                fn_build_files: None,
            };
            Ok(Box::new(c_lang))
        }
        "cpp" | "cc" | "C" | "cxx" | "c++" => {
            let cpp_lang = ComptimeLang {
                name: "cpp",
                cmd_str: "g++",
                ver_arg: "--version",
                build_cmd_str: "g++",
                build_args: &["-g", "-O2", "-std=gnu++23", "-static", "-lrt", "-lpthread"],
                exe_flag: Some(("-o", ArgsPosition::Pre)),
                fn_build_files: None,
            };
            Ok(Box::new(cpp_lang))
        }
        "cr" => {
            let crystal_lang = ComptimeLang {
                name: "crystal",
                cmd_str: "crystal",
                ver_arg: "--version",
                build_cmd_str: "crystal",
                build_args: &["build", "-O", "2", "--no-color"],
                exe_flag: Some(("-o", ArgsPosition::Post)),
                fn_build_files: None,
            };
            Ok(Box::new(crystal_lang))
        }
        "dart" => {
            let dart_lang = ComptimeLang {
                name: "dart",
                cmd_str: "dart",
                ver_arg: "--version",
                build_cmd_str: "dart",
                build_args: &["compile", "exe"],
                exe_flag: Some(("-o", ArgsPosition::Pre)),
                fn_build_files: None,
            };
            Ok(Box::new(dart_lang))
        }
        "erl" => Ok(Box::new(ErlLang::new())),
        "ex" => {
            let elixir_lang = RuntimeLang {
                name: "elixir",
                cmd_str: "elixir",
                cmd_args: &[],
                ver_arg: "--version",
            };
            Ok(Box::new(elixir_lang))
        }
        "go" => {
            let go_lang = ComptimeLang {
                name: "go",
                cmd_str: "go",
                ver_arg: "version",
                build_cmd_str: "go",
                build_args: &["build"],
                exe_flag: Some(("-o", ArgsPosition::Pre)),
                fn_build_files: None,
            };
            Ok(Box::new(go_lang))
        }
        "hs" => {
            let haskell_lang = ComptimeLang {
                name: "haskell",
                cmd_str: "ghc",
                ver_arg: "--version",
                build_cmd_str: "ghc",
                build_args: &["-O2", "-ferror-spans", "-threaded", "-rtsopts", "-dynamic"],
                exe_flag: Some(("-o", ArgsPosition::Pre)),
                fn_build_files: Some(|thread_stem| {
                    vec![format!("{}.hi", thread_stem), format!("{}.o", thread_stem)]
                }),
            };
            Ok(Box::new(haskell_lang))
        }
        "java" => {
            let java_lang = CustomLang {
                name: "java",
                build_cmd_str: "javac",
                build_args: &["-encoding", "UTF-8"],
                run_cmd_str: "java",
                run_args: &["-Dfile.encoding=UTF-8", "-XX:+UseSerialGC", "-Xss64m"],
                ver_arg: "--version",
                fn_target_name: |target_stem| format!("{}.class", target_stem),
                fn_build_files: None,
            };
            Ok(Box::new(java_lang))
        }
        "jl" => {
            let julia_lang = RuntimeLang {
                name: "julia",
                cmd_str: "julia",
                cmd_args: &[],
                ver_arg: "--version",
            };
            Ok(Box::new(julia_lang))
        }
        "js" => {
            let js_lang = RuntimeLang {
                name: "javascript",
                cmd_str: "node",
                cmd_args: &[],
                ver_arg: "--version",
            };
            Ok(Box::new(js_lang))
        }
        "kt" => {
            let kotlin_lang = CustomLang {
                name: "kotlin",
                build_cmd_str: "kotlinc",
                build_args: &[],
                run_cmd_str: "kotlin",
                run_args: &["-J-XX:+UseSerialGC", "-J-Xss64m"],
                ver_arg: "-version",
                fn_target_name: |target_stem| {
                    let mut chars = target_stem.chars();
                    let first_char = chars
                        .next()
                        .expect("filename should have first character")
                        .to_uppercase();
                    format!("{}{}Kt.class", first_char, chars.as_str())
                },
                fn_build_files: Some(|_| vec!["META-INF".to_string()]),
            };
            Ok(Box::new(kotlin_lang))
        }
        "lean" => {
            let lean_lang = RuntimeLang {
                name: "lean",
                cmd_str: "lean",
                cmd_args: &["--run"],
                ver_arg: "--version",
            };
            Ok(Box::new(lean_lang))
        }
        "lua" => {
            let lua_lang = RuntimeLang {
                name: "lua",
                cmd_str: "lua",
                cmd_args: &[],
                ver_arg: "-v",
            };
            Ok(Box::new(lua_lang))
        }
        "ml" => {
            let ocaml_lang = ComptimeLang {
                name: "ocaml",
                cmd_str: "ocamlopt",
                ver_arg: "--version",
                build_cmd_str: "ocamlopt",
                build_args: &["-I", "+unix", "unix.cmxa", "-I", "+str", "str.cmxa"],
                exe_flag: Some(("-o", ArgsPosition::Pre)),
                fn_build_files: Some(|target_stem| {
                    vec![
                        format!("{}.cmi", target_stem),
                        format!("{}.cmx", target_stem),
                        format!("{}.o", target_stem),
                    ]
                }),
            };
            Ok(Box::new(ocaml_lang))
        }
        "odin" => {
            let odin_lang = ComptimeLang {
                name: "odin",
                cmd_str: "odin",
                ver_arg: "version",
                build_cmd_str: "odin",
                build_args: &["build"],
                exe_flag: Some(("-file -out:", ArgsPosition::Post)),
                fn_build_files: None,
            };
            Ok(Box::new(odin_lang))
        }
        "py" | "py3" => {
            let py_lang = RuntimeLang {
                name: "python",
                cmd_str: "python3",
                cmd_args: &[],
                ver_arg: "--version",
            };
            Ok(Box::new(py_lang))
        }
        "rb" => {
            let ruby_lang = RuntimeLang {
                name: "ruby",
                cmd_str: "ruby",
                cmd_args: &["--yjit"],
                ver_arg: "--version",
            };
            Ok(Box::new(ruby_lang))
        }
        "rs" => {
            let rust_lang = ComptimeLang {
                name: "rust",
                cmd_str: "rustc",
                ver_arg: "--version",
                build_cmd_str: "rustc",
                build_args: &["-C", "opt-level=3", "-C", "target-cpu=native"],
                exe_flag: Some(("-o", ArgsPosition::Post)),
                fn_build_files: None,
            };
            Ok(Box::new(rust_lang))
        }
        "ts" => {
            let ts_lang = CustomLang {
                name: "typescript",
                build_cmd_str: "tsc",
                build_args: &["--module", "commonjs"],
                run_cmd_str: "node",
                run_args: &[],
                ver_arg: "--version",
                fn_target_name: |target_stem| format!("{}.js", target_stem),
                fn_build_files: None,
            };
            Ok(Box::new(ts_lang))
        }
        "zig" => {
            let zig_lang = ComptimeLang {
                name: "zig",
                cmd_str: "zig",
                ver_arg: "version",
                build_cmd_str: "zig",
                build_args: &["build-exe", "-O", "ReleaseFast"],
                exe_flag: Some(("-femit-bin=", ArgsPosition::Pre)),
                fn_build_files: None,
            };
            Ok(Box::new(zig_lang))
        }
        _ => Err(not_supported!(lang_ext)),
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum ArgsPosition {
    Post,
    Pre,
}

#[derive(Debug)]
struct ComptimeLang {
    name: &'static str,
    cmd_str: &'static str,
    ver_arg: &'static str,
    build_cmd_str: &'static str,
    build_args: &'static [&'static str],
    exe_flag: Option<(&'static str, ArgsPosition)>,
    fn_build_files: Option<fn(&str) -> Vec<String>>,
}

impl ProgLang for ComptimeLang {
    fn build_cmd(&self, filename: &str) -> Result<Command, OwlError> {
        let mut cmd = Command::new(self.build_cmd_str);
        cmd.args(self.build_args);

        let target_stem = Path::new(filename)
            .file_stem()
            .and_then(OsStr::to_str)
            .ok_or(file_error!(filename))?
            .to_string();

        if let Some((flag, pos)) = self.exe_flag {
            if pos == ArgsPosition::Post {
                cmd.arg(filename);
            }

            if flag.contains('=') || flag.contains(':') {
                let exe_arg = format!("{}{}", flag, &target_stem);

                if exe_arg.contains(' ') {
                    let split = exe_arg.split(' ').collect::<Vec<&str>>();
                    cmd.args(split);
                } else {
                    cmd.arg(exe_arg);
                }
            } else {
                cmd.args([flag, &target_stem]);
            }

            if pos == ArgsPosition::Pre {
                cmd.arg(filename);
            }
        } else {
            cmd.arg(filename);
        }

        Ok(cmd)
    }

    fn build_files(&self, target_stem: &str) -> Option<Vec<String>> {
        self.fn_build_files
            .map(|get_build_files| (get_build_files)(target_stem))
    }

    fn name(&self) -> &str {
        self.name
    }

    fn run_it(&self, target: &str, stdin: Option<&str>) -> Result<(String, u128), OwlError> {
        match stdin {
            Some(input) => cmd_utils::run_binary_with_stdin(target, input),
            None => cmd_utils::run_binary(target),
        }
    }

    fn should_build(&self) -> bool {
        true
    }

    fn target_name(&self, target_stem: &str) -> String {
        target_stem.to_string()
    }

    fn version_cmd(&self) -> Result<Command, OwlError> {
        let mut cmd = Command::new(self.cmd_str);
        cmd.arg(self.ver_arg);

        Ok(cmd)
    }
}

pub struct RuntimeLang {
    name: &'static str,
    cmd_str: &'static str,
    cmd_args: &'static [&'static str],
    ver_arg: &'static str,
}

impl ProgLang for RuntimeLang {
    fn build_cmd(&self, filename: &str) -> Result<Command, OwlError> {
        Err(program_error!(format!(
            "No build command ({}) for '{}'",
            self.name(),
            filename
        )))
    }

    fn build_files(&self, _: &str) -> Option<Vec<String>> {
        None
    }

    fn name(&self) -> &str {
        self.name
    }

    fn run_it(&self, target: &str, stdin: Option<&str>) -> Result<(String, u128), OwlError> {
        let mut run_cmd = Command::new(self.cmd_str);
        run_cmd.args(self.cmd_args);
        run_cmd.arg(target);

        match stdin {
            Some(input) => cmd_utils::run_cmd_with_stdin(run_cmd, input),
            None => cmd_utils::run_cmd(run_cmd),
        }
    }

    fn should_build(&self) -> bool {
        false
    }

    fn target_name(&self, target_stem: &str) -> String {
        target_stem.to_string()
    }

    fn version_cmd(&self) -> Result<Command, OwlError> {
        let mut cmd = Command::new(self.cmd_str);
        cmd.arg(self.ver_arg);

        Ok(cmd)
    }
}

pub struct CustomLang {
    name: &'static str,
    build_cmd_str: &'static str,
    build_args: &'static [&'static str],
    run_cmd_str: &'static str,
    run_args: &'static [&'static str],
    ver_arg: &'static str,
    fn_build_files: Option<fn(&str) -> Vec<String>>,
    fn_target_name: fn(&str) -> String,
}

impl ProgLang for CustomLang {
    fn build_cmd(&self, filename: &str) -> Result<Command, OwlError> {
        let mut cmd = Command::new(self.build_cmd_str);
        cmd.args(self.build_args);
        cmd.arg(filename);

        Ok(cmd)
    }

    fn build_files(&self, target_stem: &str) -> Option<Vec<String>> {
        self.fn_build_files
            .map(|get_build_files| (get_build_files)(target_stem))
    }

    fn name(&self) -> &str {
        self.name
    }

    fn run_it(&self, target: &str, stdin: Option<&str>) -> Result<(String, u128), OwlError> {
        let mut cmd = Command::new(self.run_cmd_str);
        cmd.args(self.run_args);

        let target_stem = Path::new(target)
            .file_stem()
            .and_then(OsStr::to_str)
            .ok_or(file_error!(target))?;

        cmd.arg(target_stem);

        match stdin {
            Some(input) => cmd_utils::run_cmd_with_stdin(cmd, input),
            None => cmd_utils::run_cmd(cmd),
        }
    }

    fn should_build(&self) -> bool {
        true
    }

    fn target_name(&self, target_stem: &str) -> String {
        (self.fn_target_name)(target_stem)
    }

    fn version_cmd(&self) -> Result<Command, OwlError> {
        let mut cmd = Command::new(self.build_cmd_str);
        cmd.arg(self.ver_arg);

        Ok(cmd)
    }
}

pub struct ErlLang {
    name: &'static str,
    cmd_str: &'static str,
    build_args: &'static [&'static str],
    post_run_args: &'static [&'static str],
    pre_run_args: &'static [&'static str],
    ver_args: &'static [&'static str],
    fn_target_name: fn(&str) -> String,
}

impl ErlLang {
    fn new() -> Self {
        ErlLang {
            name: "erlang",
            cmd_str: "erl",
            build_args: &["-compile"],
            post_run_args: &["-s", "init", "stop", "-noshell"],
            pre_run_args: &["-run"],
            ver_args: &["-s", "erlang", "halt"],
            fn_target_name: |target_stem| format!("{}.beam", target_stem),
        }
    }
}

impl ProgLang for ErlLang {
    fn build_cmd(&self, filename: &str) -> Result<Command, OwlError> {
        let mut cmd = Command::new(self.cmd_str);
        cmd.args(self.build_args);
        cmd.arg(filename);

        Ok(cmd)
    }

    fn build_files(&self, _: &str) -> Option<Vec<String>> {
        None
    }

    fn name(&self) -> &str {
        self.name
    }

    fn run_it(&self, target: &str, stdin: Option<&str>) -> Result<(String, u128), OwlError> {
        let mut cmd = Command::new(self.cmd_str);
        cmd.args(self.pre_run_args);

        let target_stem = Path::new(target)
            .file_stem()
            .and_then(OsStr::to_str)
            .ok_or(file_error!(target))?;

        cmd.arg(target_stem);
        cmd.args(self.post_run_args);

        match stdin {
            Some(input) => cmd_utils::run_cmd_with_stdin(cmd, input),
            None => cmd_utils::run_cmd(cmd),
        }
    }

    fn should_build(&self) -> bool {
        true
    }

    fn target_name(&self, target_stem: &str) -> String {
        (self.fn_target_name)(target_stem)
    }

    fn version_cmd(&self) -> Result<Command, OwlError> {
        let mut cmd = Command::new(self.cmd_str);
        cmd.args(self.ver_args);

        Ok(cmd)
    }
}
