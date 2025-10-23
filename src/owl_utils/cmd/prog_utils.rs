use super::cmd_utils;
use crate::common::{OwlError, Result};
use crate::owl_utils::fs::fs_utils;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

pub fn build_program(prog: &Path) -> Result<Option<BuildLog>> {
    match check_prog_lang(prog) {
        Some(lang) => {
            if !lang.command_exists() {
                return Err(OwlError::CommandNotFound(format!(
                    "'{}': command not found",
                    lang.name()
                )));
            }

            if lang.should_build() {
                let build_log = lang.build(prog)?;
                println!("{}", build_log.stdout);

                Ok(Some(build_log))
            } else {
                Ok(None)
            }
        }
        None => Ok(None),
    }
}

pub fn check_prog_lang(prog: &Path) -> Option<Box<dyn ProgLang>> {
    prog.extension()
        .and_then(OsStr::to_str)
        .and_then(|ext| try_prog_lang(ext).ok())
}

pub fn cleanup_program(
    prog: &Path,
    target: &Path,
    build_files: Option<Vec<PathBuf>>,
) -> Result<()> {
    if target != prog {
        fs_utils::remove_path(target)?;
    }

    if let Some(build_files) = &build_files {
        for build_file in build_files {
            fs_utils::remove_path(build_file)?;
        }
    }

    Ok(())
}

pub fn try_prog_lang(lang_ext: &str) -> Result<Box<dyn ProgLang>> {
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
                build_args: &[
                    "-O2",
                    "-ferror-spans",
                    "-threaded",
                    "-rtsopts",
                    "-dynamic",
                    "-outputdir",
                    ".",
                ],
                exe_flag: Some(("-o", ArgsPosition::Pre)),
                fn_build_files: Some(|thread_stem| {
                    vec![
                        "Main.o".into(),
                        "Main.hi".into(),
                        format!("{}.hi", thread_stem),
                        format!("{}.o", thread_stem),
                    ]
                }),
            };
            Ok(Box::new(haskell_lang))
        }
        "java" => {
            let java_lang = CustomLang {
                name: "java",
                build_cmd_str: "javac",
                build_args: &["-encoding", "UTF-8", "-d", "."],
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
        "ml" => Ok(Box::new(OcamlLang::new())),
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
                build_args: &["--module", "commonjs", "-outDir", "."],
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
        _ => Err(OwlError::Unsupported(lang_ext.to_string())),
    }
}

pub trait ProgLang {
    fn build_cmd(&self, path: &Path) -> Result<Command>;
    fn build_files(&self, parent: &Path, target_stem: &str) -> Option<Vec<PathBuf>>;
    fn name(&self) -> &str;
    fn run_it(&self, path: &Path, stdin: Option<&str>) -> Result<(String, Duration)>;
    fn should_build(&self) -> bool;
    fn target_path(&self, parent: &Path, target_stem: &str) -> PathBuf;
    fn version_cmd(&self) -> Result<Command>;

    fn build(&self, path: &Path) -> Result<BuildLog> {
        let output = self
            .build_cmd(path)?
            .output()
            .expect("[build] failed to spawn");

        if output.status.success() {
            let stdout = String::from_utf8(output.stdout)
                .map_err(|e| {
                    OwlError::FileError(
                        format!("'{}': could not read stdout", self.name()),
                        e.to_string(),
                    )
                })?
                .to_string();

            let parent = path.parent().ok_or(OwlError::FileError(
                format!("'{}': has no parent dir", path.to_string_lossy()),
                "".into(),
            ))?;

            let target_stem =
                path.file_stem()
                    .and_then(OsStr::to_str)
                    .ok_or(OwlError::UriError(
                        format!("'{}': has no file stem", path.to_string_lossy()),
                        "".into(),
                    ))?;

            Ok(BuildLog {
                target: self.target_path(parent, target_stem),
                stdout,
                build_files: self.build_files(parent, target_stem),
            })
        } else {
            let mut stderr = String::from_utf8(output.stderr)
                .map_err(|e| {
                    OwlError::FileError(
                        format!("'{}': could not read stdout", self.name()),
                        e.to_string(),
                    )
                })?
                .to_string();

            stderr.push_str("(run program manually for stack trace)");

            Err(OwlError::ProcessError(
                "'build': exit with status failed".into(),
                stderr,
            ))
        }
    }

    fn command_exists(&self) -> bool {
        self.version().is_ok()
    }

    fn version(&self) -> Result<String> {
        let output = self
            .version_cmd()?
            .output()
            .expect("[version] failed to spawn");

        if output.status.success() {
            Ok(String::from_utf8(output.stdout)
                .map_err(|e| {
                    OwlError::FileError(
                        format!("'{} version': could not read stdout", self.name()),
                        e.to_string(),
                    )
                })?
                .to_string())
        } else {
            let stderr = String::from_utf8(output.stderr)
                .map_err(|e| {
                    OwlError::FileError(
                        format!("'{} version': could not read stderr", self.name()),
                        e.to_string(),
                    )
                })?
                .to_string();

            Err(OwlError::ProcessError(
                format!("'{} version': unable to determine version", self.name()),
                stderr,
            ))
        }
    }

    fn run(&self, path: &Path) -> Result<(String, Duration)> {
        self.run_it(path, None)
    }

    fn run_with_stdin(&self, path: &Path, input: &str) -> Result<(String, Duration)> {
        self.run_it(path, Some(input))
    }
}

pub struct BuildLog {
    pub target: PathBuf,
    pub stdout: String,
    pub build_files: Option<Vec<PathBuf>>,
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
    fn build_cmd(&self, path: &Path) -> Result<Command> {
        let mut cmd = Command::new(self.build_cmd_str);
        cmd.args(self.build_args);

        let target_stem = path
            .file_stem()
            .and_then(OsStr::to_str)
            .ok_or(OwlError::UriError(
                format!("'{}': has no file stem", path.to_string_lossy()),
                "".into(),
            ))?;

        if let Some((flag, pos)) = self.exe_flag {
            if pos == ArgsPosition::Post {
                cmd.arg(path);
            }

            if flag.contains('=') || flag.contains(':') {
                let exe_arg = format!("{}{}", flag, target_stem);

                if exe_arg.contains(' ') {
                    let split = exe_arg.split(' ').collect::<Vec<&str>>();
                    cmd.args(split);
                } else {
                    cmd.arg(exe_arg);
                }
            } else {
                cmd.args([flag, target_stem]);
            }

            if pos == ArgsPosition::Pre {
                cmd.arg(path);
            }
        } else {
            cmd.arg(path);
        }

        Ok(cmd)
    }

    fn build_files(&self, _: &Path, target_stem: &str) -> Option<Vec<PathBuf>> {
        self.fn_build_files
            .map(|get_build_files| (get_build_files)(target_stem))
            .map(|build_names| {
                build_names
                    .into_iter()
                    .map(PathBuf::from)
                    .collect::<Vec<PathBuf>>()
            })
    }

    fn name(&self) -> &str {
        self.name
    }

    fn run_it(&self, path: &Path, stdin: Option<&str>) -> Result<(String, Duration)> {
        match stdin {
            Some(input) => cmd_utils::run_binary_with_stdin(path, input),
            None => cmd_utils::run_binary(path),
        }
    }

    fn should_build(&self) -> bool {
        true
    }

    fn target_path(&self, _: &Path, target_stem: &str) -> PathBuf {
        PathBuf::from(target_stem)
    }

    fn version_cmd(&self) -> Result<Command> {
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
    fn build_cmd(&self, path: &Path) -> Result<Command> {
        Err(OwlError::ProcessError(
            format!(
                "No build command ({}) for '{}'",
                self.name(),
                path.to_string_lossy()
            ),
            "".into(),
        ))
    }

    fn build_files(&self, _: &Path, _: &str) -> Option<Vec<PathBuf>> {
        None
    }

    fn name(&self) -> &str {
        self.name
    }

    fn run_it(&self, path: &Path, stdin: Option<&str>) -> Result<(String, Duration)> {
        let mut run_cmd = Command::new(self.cmd_str);
        run_cmd.args(self.cmd_args);
        run_cmd.arg(path);

        match stdin {
            Some(input) => cmd_utils::run_cmd_with_stdin(self.cmd_str, run_cmd, input),
            None => cmd_utils::run_cmd(self.cmd_str, run_cmd),
        }
    }

    fn should_build(&self) -> bool {
        false
    }

    fn target_path(&self, parent: &Path, target_stem: &str) -> PathBuf {
        let mut path = parent.to_path_buf();
        path.push(target_stem);

        path
    }

    fn version_cmd(&self) -> Result<Command> {
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
    fn build_cmd(&self, path: &Path) -> Result<Command> {
        let mut cmd = Command::new(self.build_cmd_str);
        cmd.args(self.build_args);
        cmd.arg(path);

        Ok(cmd)
    }

    fn build_files(&self, _: &Path, target_stem: &str) -> Option<Vec<PathBuf>> {
        self.fn_build_files
            .map(|get_build_files| (get_build_files)(target_stem))
            .map(|build_names| {
                build_names
                    .into_iter()
                    .map(PathBuf::from)
                    .collect::<Vec<PathBuf>>()
            })
    }

    fn name(&self) -> &str {
        self.name
    }

    fn run_it(&self, path: &Path, stdin: Option<&str>) -> Result<(String, Duration)> {
        let mut cmd = Command::new(self.run_cmd_str);
        cmd.args(self.run_args);

        let target_stem = path
            .file_stem()
            .and_then(OsStr::to_str)
            .ok_or(OwlError::UriError(
                format!("'{}': has no file stem", path.to_string_lossy()),
                "".into(),
            ))?;

        cmd.arg(target_stem);

        match stdin {
            Some(input) => cmd_utils::run_cmd_with_stdin(self.run_cmd_str, cmd, input),
            None => cmd_utils::run_cmd(self.run_cmd_str, cmd),
        }
    }

    fn should_build(&self) -> bool {
        true
    }

    fn target_path(&self, _: &Path, target_stem: &str) -> PathBuf {
        PathBuf::from((self.fn_target_name)(target_stem))
    }

    fn version_cmd(&self) -> Result<Command> {
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
    fn build_cmd(&self, path: &Path) -> Result<Command> {
        let mut cmd = Command::new(self.cmd_str);
        cmd.args(self.build_args);
        cmd.arg(path);

        Ok(cmd)
    }

    fn build_files(&self, _: &Path, _: &str) -> Option<Vec<PathBuf>> {
        None
    }

    fn name(&self) -> &str {
        self.name
    }

    fn run_it(&self, path: &Path, stdin: Option<&str>) -> Result<(String, Duration)> {
        let mut cmd = Command::new(self.cmd_str);
        cmd.args(self.pre_run_args);

        let target_stem = path
            .file_stem()
            .and_then(OsStr::to_str)
            .ok_or(OwlError::UriError(
                format!("'{}': has no file stem", path.to_string_lossy()),
                "".into(),
            ))?;

        cmd.arg(target_stem);
        cmd.args(self.post_run_args);

        match stdin {
            Some(input) => cmd_utils::run_cmd_with_stdin(self.cmd_str, cmd, input),
            None => cmd_utils::run_cmd(self.cmd_str, cmd),
        }
    }

    fn should_build(&self) -> bool {
        true
    }

    fn target_path(&self, _: &Path, target_stem: &str) -> PathBuf {
        PathBuf::from((self.fn_target_name)(target_stem))
    }

    fn version_cmd(&self) -> Result<Command> {
        let mut cmd = Command::new(self.cmd_str);
        cmd.args(self.ver_args);

        Ok(cmd)
    }
}

struct OcamlLang {
    name: &'static str,
    cmd_str: &'static str,
    ver_arg: &'static str,
    build_cmd_str: &'static str,
    build_args: &'static [&'static str],
}

impl OcamlLang {
    pub fn new() -> Self {
        OcamlLang {
            name: "ocaml",
            cmd_str: "ocamlopt",
            ver_arg: "--version",
            build_cmd_str: "ocamlopt",
            build_args: &["-I", "+unix", "unix.cmxa", "-I", "+str", "str.cmxa"],
        }
    }
}

impl ProgLang for OcamlLang {
    fn build_cmd(&self, path: &Path) -> Result<Command> {
        let mut cmd = Command::new(self.build_cmd_str);
        cmd.args(self.build_args);
        cmd.arg(path);

        let target_stem = path
            .file_stem()
            .and_then(OsStr::to_str)
            .ok_or(OwlError::UriError(
                format!("'{}': has no file stem", path.to_string_lossy()),
                "".into(),
            ))?;

        cmd.args(["-o", target_stem]);

        Ok(cmd)
    }

    fn build_files(&self, parent: &Path, target_stem: &str) -> Option<Vec<PathBuf>> {
        let output_files = vec![
            format!("{}.cmi", target_stem),
            format!("{}.cmx", target_stem),
            format!("{}.o", target_stem),
        ];

        let output_paths = output_files
            .into_iter()
            .map(|build_name| {
                let mut path = parent.to_path_buf();
                path.push(build_name);

                path
            })
            .collect::<Vec<PathBuf>>();

        if output_paths.is_empty() {
            None
        } else {
            Some(output_paths)
        }
    }

    fn name(&self) -> &str {
        self.name
    }

    fn run_it(&self, path: &Path, stdin: Option<&str>) -> Result<(String, Duration)> {
        match stdin {
            Some(input) => cmd_utils::run_binary_with_stdin(path, input),
            None => cmd_utils::run_binary(path),
        }
    }

    fn should_build(&self) -> bool {
        true
    }

    fn target_path(&self, _: &Path, target_stem: &str) -> PathBuf {
        PathBuf::from(target_stem)
    }

    fn version_cmd(&self) -> Result<Command> {
        let mut cmd = Command::new(self.cmd_str);
        cmd.arg(self.ver_arg);

        Ok(cmd)
    }
}
