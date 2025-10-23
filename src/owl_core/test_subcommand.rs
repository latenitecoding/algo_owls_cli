use crate::common::{OwlError, Result};
use crate::owl_utils::{cmd_utils, prog_utils};
use std::fs;
use std::path::Path;
use std::time::Duration;

macro_rules! report_test_failed {
    ($test_case:expr, $expected:expr, $actual:expr) => {
        eprintln!(
            concat!(
                "\x1b[31m{}\x1b[0m: {}\n\n",
                "\x1b[1;33m{}\x1b[0m\n\n{}\n",
                "\x1b[1;35m{}\x1b[0m\n\n{}\n",
            ),
            "[test failure]",
            $test_case
                .to_str()
                .map(String::from)
                .unwrap_or($test_case.to_string_lossy().to_string()),
            ">>> expected <<<",
            $expected,
            ">>> actual <<<",
            $actual
        )
    };
}

pub fn test_it(target: &Path, in_file: &Path, ans_file: &Path) -> Result<Duration> {
    if !target.exists() {
        return Err(OwlError::FileError(
            format!("'{}': no such file", target.to_string_lossy()),
            "".into(),
        ));
    }
    if !in_file.exists() {
        return Err(OwlError::FileError(
            format!("'{}': no such file", in_file.to_string_lossy()),
            "".into(),
        ));
    }
    if !ans_file.exists() {
        return Err(OwlError::FileError(
            format!("'{}': no such file", ans_file.to_string_lossy()),
            "".into(),
        ));
    }

    let stdin = fs::read_to_string(in_file).map_err(|e| {
        OwlError::FileError(
            format!("could not read from '{}'", in_file.to_string_lossy()),
            e.to_string(),
        )
    })?;
    let ans = fs::read_to_string(ans_file).map_err(|e| {
        OwlError::FileError(
            format!("could not read from '{}'", ans_file.to_string_lossy()),
            e.to_string(),
        )
    })?;

    match prog_utils::check_prog_lang(target) {
        Some(lang) => {
            if !lang.command_exists() {
                return Err(OwlError::CommandNotFound(format!(
                    "'{}': command not found",
                    lang.name()
                )));
            }

            let run_result = lang.run_with_stdin(target, &stdin);

            run_result.and_then(|(actual, elapsed)| {
                if actual == ans {
                    Ok(elapsed)
                } else {
                    report_test_failed!(in_file, ans, actual);
                    Err(OwlError::TestFailure("failed test".into()))
                }
            })
        }
        None => cmd_utils::run_binary_with_stdin(target, &stdin).and_then(|(actual, elapsed)| {
            if actual == ans {
                Ok(elapsed)
            } else {
                report_test_failed!(in_file, ans, actual);
                Err(OwlError::TestFailure("failed test".into()))
            }
        }),
    }
}

pub fn test_program(prog: &Path, in_file: &Path, ans_file: &Path) -> Result<()> {
    let test_result = match prog_utils::check_prog_lang(prog) {
        Some(_) => {
            let (target, build_files) = match prog_utils::build_program(prog)? {
                Some(bl) => (bl.target, bl.build_files),
                None => (prog.to_path_buf(), None),
            };

            let test_result = test_it(&target, in_file, ans_file);

            prog_utils::cleanup_program(prog, &target, build_files)?;

            test_result
        }
        None => test_it(prog, in_file, ans_file),
    };

    match test_result {
        Ok(elapsed) => {
            println!(
                "[{}ms] \x1b[32mpassed test\x1b[0m 🎉\n",
                elapsed.as_millis()
            );
            Ok(())
        }
        Err(e) => {
            eprintln!("\x1b[31m{}\x1b[0m 😭\n", e);
            Ok(())
        }
    }
}
