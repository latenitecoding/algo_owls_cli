use clap::{Command, arg};
use std::collections::VecDeque;
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
                "\x1b[31m{}\x1b[0m: '{}'\n\n",
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
            Command::new("run")
                .about("builds and executes target program")
                .arg(arg!(<PROG> "The program to run"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("fetch")
                .about("fetches sample test cases from given URL")
                .arg(arg!(<URL> "The URL to fetch from"))
                .arg(arg!(<DIR> "The local directory to extract into"))
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
            Command::new("add")
                .about("adds new personal quest to manifest")
                .arg(arg!(<NAME> "The name of the quest"))
                .arg(arg!(<URL> "The URL to fetch from"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("quest")
                .about("tests program against all test cases")
                .arg(arg!(<NAME> "The name of the quest"))
                .arg(arg!(<PROG> "The program to test"))
                .arg_required_else_help(true),
        )
}

fn add(name: &str, url: &str) -> Result<(), OwlError> {
    // this should always rewrite entries in the personal table
    // of the manifest TOML, which is the last table in the manifest
    // new entires can always be appended
    let mut manifest_path = fs_utils::ensure_dir_from_home(OWL_DIR)?;
    manifest_path.push(MANIFEST);

    if !manifest_path.exists() {
        fs_utils::create_toml_with_entry(&manifest_path, TOML_TEMPLATE, "personal", name, url)
    } else {
        fs_utils::update_toml_entry(&manifest_path, "personal", name, url)
    }
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

fn fetch(url: &str, dir: &str) -> Result<(), OwlError> {
    fs_utils::download_file(url, TMP_ARCHIVE)?;
    fs_utils::extract_archive(TMP_ARCHIVE, dir)?;

    fs_utils::remove_path(TMP_ARCHIVE)
}

fn fetch_by_name(name: &str, dir: &str) -> Result<(), OwlError> {
    let mut manifest_path = fs_utils::ensure_dir_from_home(OWL_DIR)?;
    manifest_path.push(MANIFEST);

    if !manifest_path.exists() {
        let path_str = check_path!(manifest_path)?;
        return Err(file_not_found!(path_str));
    }

    let url = fs_utils::get_toml_entry(&manifest_path, &["quests", "personal"], name)?;

    fetch(&url, dir)
}

fn quest(name: &str, prog: &str) -> Result<(), OwlError> {
    let mut quest_path = fs_utils::ensure_dir_from_home(OWL_DIR)?;
    quest_path.push(name);

    let quest_dir = check_path!(quest_path)?.to_string();

    if !quest_path.exists() {
        fetch_by_name(name, &quest_dir)?;
    }

    let mut test_cases: Vec<String> = Vec::new();

    let mut queue: VecDeque<String> = VecDeque::new();
    queue.push_back(quest_dir);

    while let Some(dir) = queue.pop_front() {
        for entry in fs::read_dir(dir).map_err(|e| file_error!(e))? {
            let path = entry.map_err(|e| file_error!(e))?.path();

            if path.is_dir() {
                queue.push_back(check_path!(path)?.to_string());
            } else if path.is_file()
                && let Some(ext) = path.extension().and_then(OsStr::to_str)
                && ext == "in"
            {
                test_cases.push(check_path!(path)?.to_string());
            }
        }
    }

    let prog_path = Path::new(prog);

    if !prog_path.exists() {
        return Err(file_not_found!(prog));
    }

    let target = build_program(prog)?;

    let total = test_cases.len();
    let mut passed = 0;
    let mut failed = 0;

    for test_case in test_cases {
        let in_path = Path::new(&test_case);
        let mut ans_path = in_path
            .parent()
            .ok_or(file_error!(format!("no parent of: '{}'", test_case)))?
            .to_path_buf();

        let stem = format!(
            "{}.ans",
            in_path
                .file_stem()
                .and_then(OsStr::to_str)
                .ok_or(file_error!(test_case))?
        );
        ans_path.push(&stem);

        let ans_file = check_path!(&ans_path)?;

        match quest_it(&target, &test_case, ans_file) {
            Ok(_) => {
                println!(
                    "({}/{}) \x1b[32mpassed test\x1b[0m 🎉\n",
                    passed + failed + 1,
                    total
                );
                passed += 1;
            }
            Err(e) => {
                eprintln!(
                    "({}/{}) \x1b[31m{}\x1b[0m 😭\n",
                    passed + failed + 1,
                    total,
                    e
                );
                failed += 1;
            }
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

fn quest_it(target: &str, in_file: &str, ans_file: &str) -> Result<(), OwlError> {
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

fn test(prog: &str, in_file: &str, ans_file: &str) -> Result<(), OwlError> {
    match prog_lang::check_prog_lang(prog) {
        Some(_) => {
            let target = build_program(prog)?;

            let test_result = quest_it(&target, in_file, ans_file);

            fs_utils::remove_path(&target)?;

            test_result
        }
        None => quest_it(prog, in_file, ans_file),
    }
}

fn main() {
    let matches = cli().get_matches();

    match matches.subcommand() {
        Some(("run", sub_matches)) => {
            let prog = sub_matches.get_one::<String>("PROG").expect("required");

            if let Err(e) = run(prog) {
                report_owl_err!(&e);
            }
        }
        Some(("fetch", sub_matches)) => {
            let url = sub_matches.get_one::<String>("URL").expect("required");
            let dir = sub_matches.get_one::<String>("DIR").expect("required");

            if let Err(e) = fetch(url, dir) {
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
        Some(("add", sub_matches)) => {
            let name = sub_matches.get_one::<String>("NAME").expect("required");
            let url = sub_matches.get_one::<String>("URL").expect("required");

            if let Err(e) = add(name, url) {
                report_owl_err!(&e);
            }
        }
        Some(("quest", sub_matches)) => {
            let name = sub_matches.get_one::<String>("NAME").expect("required");
            let prog = sub_matches.get_one::<String>("PROG").expect("required");

            if let Err(e) = quest(name, prog) {
                report_owl_err!(&e);
            }
        }
        _ => unreachable!(),
    }
}
