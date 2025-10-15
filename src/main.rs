use clap::{Command, arg};
use std::ffi::OsStr;
use std::fs;
use std::path::Path;

mod owl_utils;
use owl_utils::{cmd_utils, fs_utils, owl_error::OwlError, prog_lang};

const OWL_DIR: &str = ".owl";
const MANIFEST: &str = ".manifest.toml";
const TMP_ARCHIVE: &str = ".tmp.zip";

const TOML_TEMPLATE: &str = r#"
[manifest]
version = "0.0.0"

[quests]

[personal]
"#;

macro_rules! report_owl_err {
    ($expr:expr) => {
        eprintln!("\x1b[31m[owl error]\x1b[0m: {}", $expr);
    };
}

macro_rules! report_test_failed {
    ($test_case:expr, $expected:expr, $actual:expr) => {
        eprintln!(
            concat!(
                "\x1b[31m{}\x1b[0m: {}\n\n",
                "\x1b[1;33m{}\x1b[0m\n\n{}\n",
                "\x1b[1;35m{}\x1b[0m\n\n{}\n",
            ),
            "[test failure]", $test_case, ">>> expected <<<", $expected, ">>> actual <<<", $actual
        )
    };
}

fn cli() -> Command {
    Command::new("owl")
        .about("A lightweight CLI to assist in solving CP problems")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("add")
                .about("adds new personal quest to manifest")
                .arg(arg!(<NAME> "The name of the quest"))
                .arg(arg!(<URL> "The URL to fetch from"))
                .arg(arg!(-f --fetch "Fetches test cases"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("fetch")
                .about("fetches sample test cases for the given quest")
                .arg(arg!(<NAME> "The name of the quest"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("run")
                .about("builds and executes target program")
                .arg(arg!(<PROG> "The program to run"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("test")
                .about("runs program against sample test case")
                .arg(arg!(<PROG> "The program to test"))
                .arg(arg!(<IN> "The input file for the test case"))
                .arg(arg!(<ANS> "The answer file to the test case"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("quest")
                .about("tests program against all test cases")
                .arg(arg!(<NAME> "The name of the quest"))
                .arg(arg!(<PROG> "The program to test"))
                .arg(arg!(-t --test <TEST> "The specific test to run by name"))
                .arg(arg!(-c --case <CASE> "The specific test to run by case number"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("show")
                .about("prints the input(s) or answer(s) to the test cases")
                .arg(arg!(<NAME> "The name of the quest"))
                .arg(arg!(-t --test <TEST> "The specific test to print by name"))
                .arg(arg!(-c --case <CASE> "The specific test to print by case number"))
                .arg(arg!(-a --ans "Print the answer instead of the input"))
                .arg_required_else_help(true),
        )
}

fn add(name: &str, url: &str, and_fetch: bool) -> Result<(), OwlError> {
    // this should always rewrite entries in the personal table
    // of the manifest TOML, which is the last table in the manifest
    // new entires can always be appended
    let mut manifest_path = fs_utils::ensure_dir_from_home(OWL_DIR)?;
    manifest_path.push(MANIFEST);

    if !manifest_path.exists() {
        fs_utils::create_toml_with_entry(&manifest_path, TOML_TEMPLATE, "personal", name, url)?
    } else {
        fs_utils::update_toml_entry(&manifest_path, "personal", name, url)?
    };

    if and_fetch {
        fetch_by_name(name)?;
    }

    Ok(())
}

fn build_program(prog: &str) -> Result<String, OwlError> {
    match prog_lang::check_prog_lang(prog) {
        Some(lang) => {
            if !lang.command_exists() {
                return Err(command_not_found!(lang.name()));
            }

            let build_log = lang.build(prog)?;
            println!("{}", build_log.stdout);

            Ok(build_log.target)
        }
        None => Ok(prog.to_string()),
    }
}

fn fetch(name: &str, dir: &str) -> Result<(), OwlError> {
    let mut manifest_path = fs_utils::ensure_dir_from_home(OWL_DIR)?;
    manifest_path.push(MANIFEST);

    if !manifest_path.exists() {
        let path_str = check_path!(manifest_path)?;
        return Err(file_not_found!(path_str));
    }

    let url = fs_utils::get_toml_entry(&manifest_path, &["quests", "personal"], name)?;

    fs_utils::download_file(&url, TMP_ARCHIVE)?;
    fs_utils::extract_archive(TMP_ARCHIVE, dir)?;

    fs_utils::remove_path(TMP_ARCHIVE)
}

fn fetch_by_name(name: &str) -> Result<(), OwlError> {
    let mut fetch_path = fs_utils::ensure_dir_from_home(OWL_DIR)?;
    fetch_path.push(name);

    fetch(name, check_path!(fetch_path)?)
}

fn quest(
    name: &str,
    prog: &str,
    test_name: Option<&String>,
    case_id: usize,
) -> Result<(), OwlError> {
    let mut quest_path = fs_utils::ensure_dir_from_home(OWL_DIR)?;
    quest_path.push(name);

    let quest_dir = check_path!(quest_path)?.to_string();

    if !quest_path.exists() {
        fetch(name, &quest_dir)?;
    }

    let prog_path = Path::new(prog);

    if !prog_path.exists() {
        return Err(file_not_found!(prog));
    }

    let target = build_program(prog)?;

    let test_cases: Vec<String> = fs_utils::find_by_ext(quest_dir, "in")?;
    let total = test_cases.len();

    let mut passed = 0;
    let mut failed = 0;
    let mut count = 0;

    for test_case in test_cases {
        count += 1;

        let in_path = Path::new(&test_case);
        let in_stem = in_path
            .file_stem()
            .and_then(OsStr::to_str)
            .ok_or(file_error!(test_case))?;

        if let Some(name) = test_name {
            if in_stem != name {
                continue;
            }
        }

        if case_id > 0 && count != case_id {
            continue;
        }

        match quest_it(&target, &test_case, count, total) {
            Ok(true) => passed += 1,
            Ok(false) | Err(_) => failed += 1,
        }
    }

    println!("passed: {}, failed: {}", passed, failed);

    fs_utils::remove_path(&target)?;

    if failed > 0 {
        Err(test_failure!("test failures"))
    } else {
        println!("\x1b[32mall tests passed\x1b[0m 🏆🏆🏆\n");
        Ok(())
    }
}

fn quest_it(target: &str, test_case: &str, count: usize, total: usize) -> Result<bool, OwlError> {
    let in_path = Path::new(&test_case);
    let in_stem = in_path
        .file_stem()
        .and_then(OsStr::to_str)
        .ok_or(file_error!(test_case))?;

    let ans_file = fs_utils::as_ans_file(&test_case)?;

    match test_it(&target, &test_case, &ans_file) {
        Ok(_) => {
            println!(
                "({}/{}) {} \x1b[32mpassed test\x1b[0m 🎉\n",
                count, total, in_stem
            );
            Ok(true)
        }
        Err(e) => {
            eprintln!(
                "({}/{}) {} \x1b[31m{}\x1b[0m 😭\n",
                count, total, in_stem, e
            );
            Ok(false)
        }
    }
}

fn run(prog: &str) -> Result<(), OwlError> {
    if !Path::new(prog).exists() {
        return Err(file_not_found!(prog));
    }

    match prog_lang::check_prog_lang(prog) {
        Some(lang) => {
            let target = build_program(prog)?;

            let run_result = lang.run(&target);

            fs_utils::remove_path(&target)?;

            run_result.map(|stdout| println!("{}", stdout))
        }
        None => {
            println!("{}", cmd_utils::run_binary(prog)?);
            Ok(())
        }
    }
}

fn show(
    name: &str,
    test_name: Option<&String>,
    case_id: usize,
    show_ans: bool,
) -> Result<(), OwlError> {
    let mut quest_path = fs_utils::ensure_dir_from_home(OWL_DIR)?;
    quest_path.push(name);

    let quest_dir = check_path!(quest_path)?.to_string();

    if !quest_path.exists() {
        fetch(name, &quest_dir)?;
    }

    if let Some(name) = test_name {
        let test_case = fs_utils::find_by_stem_and_ext(quest_dir, name, "in")?;
        return show_it(&test_case, show_ans);
    }

    let test_cases: Vec<String> = fs_utils::find_by_ext(quest_dir, "in")?;

    if case_id > 0 {
        return show_it(&test_cases[case_id - 1], show_ans);
    }

    for test_case in test_cases {
        show_it(&test_case, show_ans)?;
    }

    Ok(())
}

fn show_it(target_file: &str, show_ans: bool) -> Result<(), OwlError> {
    let contents = if show_ans {
        let ans_file = fs_utils::as_ans_file(target_file)?;

        fs::read_to_string(ans_file).map_err(|e| file_error!(e))?
    } else {
        fs::read_to_string(target_file).map_err(|e| file_error!(e))?
    };

    println!("{}", contents);

    Ok(())
}

fn test(prog: &str, in_file: &str, ans_file: &str) -> Result<(), OwlError> {
    match prog_lang::check_prog_lang(prog) {
        Some(_) => {
            let target = build_program(prog)?;

            let test_result = test_it(&target, in_file, ans_file);

            fs_utils::remove_path(&target)?;

            test_result
        }
        None => test_it(prog, in_file, ans_file),
    }
}

fn test_it(target: &str, in_file: &str, ans_file: &str) -> Result<(), OwlError> {
    let prog_path = Path::new(target);
    let in_path = Path::new(in_file);
    let ans_path = Path::new(ans_file);

    if !prog_path.exists() {
        return Err(file_not_found!(target));
    }
    if !in_path.exists() {
        return Err(file_not_found!(in_file));
    }
    if !ans_path.exists() {
        return Err(file_not_found!(ans_file));
    }

    let stdin = fs::read_to_string(in_path).map_err(|e| file_error!(e))?;
    let ans = fs::read_to_string(ans_path).map_err(|e| file_error!(e))?;

    match prog_lang::check_prog_lang(target) {
        Some(lang) => {
            if !lang.command_exists() {
                return Err(command_not_found!(lang.name()));
            }

            let run_result = lang.run_with_stdin(target, &stdin);

            run_result.and_then(|actual| {
                if actual == ans {
                    Ok(())
                } else {
                    report_test_failed!(in_file, ans, actual);
                    Err(test_failure!("failed test"))
                }
            })
        }
        None => cmd_utils::run_binary_with_stdin(target, &stdin).and_then(|actual| {
            if actual == ans {
                Ok(())
            } else {
                report_test_failed!(in_file, ans, actual);
                Err(test_failure!("failed test"))
            }
        }),
    }
}

fn main() {
    let matches = cli().get_matches();

    match matches.subcommand() {
        Some(("add", sub_matches)) => {
            let name = sub_matches.get_one::<String>("NAME").expect("required");
            let url = sub_matches.get_one::<String>("URL").expect("required");
            let fetch = sub_matches.get_one::<bool>("fetch").map_or(false, |&f| f);

            if let Err(e) = add(name, url, fetch) {
                report_owl_err!(&e);
            }
        }
        Some(("fetch", sub_matches)) => {
            let name = sub_matches.get_one::<String>("NAME").expect("required");

            if let Err(e) = fetch_by_name(name) {
                report_owl_err!(&e);
            }
        }
        Some(("run", sub_matches)) => {
            let prog = sub_matches.get_one::<String>("PROG").expect("required");

            if let Err(e) = run(prog) {
                report_owl_err!(&e);
            }
        }
        Some(("test", sub_matches)) => {
            let prog = sub_matches.get_one::<String>("PROG").expect("required");
            let in_file = sub_matches.get_one::<String>("IN").expect("required");
            let ans_file = sub_matches.get_one::<String>("ANS").expect("required");

            if let Err(e) = test(prog, in_file, ans_file) {
                report_owl_err!(&e);
            }
        }
        Some(("quest", sub_matches)) => {
            let name = sub_matches.get_one::<String>("NAME").expect("required");
            let prog = sub_matches.get_one::<String>("PROG").expect("required");
            let test = sub_matches.get_one::<String>("test");
            let case = sub_matches
                .get_one::<String>("case")
                .map_or(0, |s| s.parse().expect("case id should be a number"));

            if let Err(e) = quest(name, prog, test, case) {
                report_owl_err!(&e);
            }
        }
        Some(("show", sub_matches)) => {
            let name = sub_matches.get_one::<String>("NAME").expect("required");
            let test = sub_matches.get_one::<String>("test");
            let case = sub_matches
                .get_one::<String>("case")
                .map_or(0, |s| s.parse().expect("case id should be a number"));
            let ans = sub_matches.get_one::<bool>("ans").map_or(false, |&f| f);

            if let Err(e) = show(name, test, case, ans) {
                report_owl_err!(&e);
            }
        }
        _ => unreachable!(),
    }
}
