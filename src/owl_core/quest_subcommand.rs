use crate::OWL_DIR;
use crate::common::{OwlError, Result};
use crate::owl_utils::{cmd_utils, fs_utils, prog_utils};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub async fn quest(
    quest_name: &str,
    prog: &Path,
    case_id: Option<usize>,
    use_hints: bool,
) -> Result<()> {
    let quest_path = fs_utils::ensure_path_from_home(&[OWL_DIR], Some(quest_name))?;

    if !quest_path.exists() {
        super::fetch_quest(quest_name).await?;
    }

    if !prog.exists() {
        return Err(OwlError::FileError(
            format!("'{}': no such file", prog.to_string_lossy()),
            "".into(),
        ));
    }

    let (target, build_files) = match prog_utils::build_program(prog)? {
        Some(bl) => (bl.target, bl.build_files),
        None => (prog.to_path_buf(), None),
    };

    let test_cases: Vec<PathBuf> = fs_utils::find_by_ext(&quest_path, "in")?;
    let total = test_cases.len();

    let mut passed = 0;
    let mut failed = 0;
    let mut total_duration: Option<Duration> = None;

    let (start, end, mut count) = match case_id {
        Some(d) => (d, d + 1, d - 1),
        None => (0, total, 0),
    };

    for test_case in test_cases.iter().skip(count).take(end - start) {
        count += 1;

        if let Some(d) = case_id
            && (count % total) != (d % total)
        {
            continue;
        }

        match quest_it(&target, test_case, count, total, use_hints) {
            Ok((true, elapsed)) => {
                passed += 1;
                total_duration = match (total_duration, elapsed) {
                    (Some(d), Some(elap_time)) => Some(d + elap_time),
                    (Some(d), _) => Some(d),
                    _ => elapsed,
                };
            }
            Ok((false, _)) | Err(_) => failed += 1,
        }
    }

    println!(
        "passed: {}, failed: {}, elapsed: {}ms",
        passed,
        failed,
        total_duration.map(|d| d.as_millis()).unwrap_or(0)
    );

    prog_utils::cleanup_program(prog, &target, build_files)?;

    if failed > 0 {
        Err(OwlError::TestFailure("test failures".into()))
    } else {
        println!("\x1b[32mall tests passed\x1b[0m 🏆🏆🏆\n");
        Ok(())
    }
}

pub fn quest_it(
    target: &Path,
    test_case: &Path,
    count: usize,
    total: usize,
    use_hints: bool,
) -> Result<(bool, Option<Duration>)> {
    let in_stem = test_case
        .file_stem()
        .and_then(OsStr::to_str)
        .ok_or(OwlError::UriError(
            format!("'{}': has no file stem", test_case.to_string_lossy()),
            "".into(),
        ))?;

    let ans_str = format!("{}.ans", in_stem);

    let mut ans_path = test_case
        .parent()
        .expect("owlgo directory to exist")
        .to_path_buf();
    ans_path.push(&ans_str);

    match super::test_it(target, test_case, &ans_path) {
        Ok(elapsed) => {
            println!(
                "({}/{}) [{}ms] {} \x1b[32mpassed test\x1b[0m 🎉\n",
                count,
                total,
                elapsed.as_millis(),
                in_stem
            );
            Ok((true, Some(elapsed)))
        }
        Err(e) => {
            if use_hints && let Some(parent_dir) = test_case.parent() {
                let feedback_file = format!("{}.md", in_stem);

                let mut feedback_path = parent_dir.to_path_buf();
                feedback_path.push(feedback_file);

                cmd_utils::bat_file(&feedback_path).or_else(|_| {
                    cmd_utils::glow_file(&feedback_path).or_else(|_| {
                        fs::read_to_string(&feedback_path)
                            .map(|contents| eprintln!("{}", contents))
                            .map_err(|e| {
                                OwlError::FileError(
                                    format!("could not read '{}'", feedback_path.to_string_lossy()),
                                    e.to_string(),
                                )
                            })
                    })
                })?
            }

            eprintln!(
                "({}/{}) {} \x1b[31m{}\x1b[0m 😭\n",
                count, total, in_stem, e
            );

            Ok((false, None))
        }
    }
}

pub async fn quest_once(
    quest_name: &str,
    prog: &Path,
    test_name: &str,
    use_hints: bool,
) -> Result<()> {
    let quest_path = fs_utils::ensure_path_from_home(&[OWL_DIR], Some(quest_name))?;

    if !quest_path.exists() {
        super::fetch_quest(quest_name).await?;
    }

    if !prog.exists() {
        return Err(OwlError::FileError(
            format!("'{}': no such file", prog.to_string_lossy()),
            "".into(),
        ));
    }

    let (target, build_files) = match prog_utils::build_program(prog)? {
        Some(bl) => (bl.target, bl.build_files),
        None => (prog.to_path_buf(), None),
    };

    let in_path = fs_utils::find_by_stem_and_ext(&quest_path, test_name, "in")?;

    let mut passed = 0;
    let mut check_elapsed: Option<Duration> = None;

    if let Ok((true, some_duration)) = quest_it(&target, &in_path, 0, 1, use_hints) {
        passed = 1;
        check_elapsed = some_duration;
    }

    println!(
        "passed: {}, failed: {}, elapsed: {}ms",
        passed,
        1 - passed,
        check_elapsed.map(|d| d.as_millis()).unwrap_or(0)
    );

    prog_utils::cleanup_program(prog, &target, build_files)?;

    if passed == 0 {
        Err(OwlError::TestFailure("test failures".into()))
    } else {
        println!("\x1b[32mall tests passed\x1b[0m 🏆🏆🏆\n");
        Ok(())
    }
}
