use clap::{Command, arg};
use std::ffi::OsStr;
use std::fs;
use std::path::Path;

mod owl_utils;
use owl_utils::{cmd_utils, fs_utils, owl_error::OwlError, prog_lang};

const OWL_DIR: &str = ".owl";
const MANIFEST: &str = ".manifest.toml";
const TEMPLATE_STEM: &str = ".template";
const TMP_ARCHIVE: &str = ".tmp.zip";
const STASH_DIR: &str = ".stash";

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
            Command::new("clean")
                .about("removes all stashed test cases (and solutions)")
                .arg(arg!(--stash "Removes all stashed programs"))
                .arg(arg!(--all "Removes all test cases and stashed programs")),
        )
        .subcommand(
            Command::new("fetch")
                .about("fetches sample test cases for the given quest")
                .arg(arg!(<NAME> "The name of the quest"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("init")
                .about("creates a local file from a stashed template")
                .arg(arg!(<PROG> "The program to initialize from the template"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("push")
                .about("pushes all stashed solutions to the remote")
                .arg(arg!(-f --force "Forces the remote to match the local stash")),
        )
        .subcommand(
            Command::new("quest")
                .about("tests program against all test cases")
                .arg(arg!(<NAME> "The name of the quest"))
                .arg(arg!(<PROG> "The program to test"))
                .arg(arg!(-t --test <TEST> "The specific test to run by name"))
                .arg(arg!(-c --case <CASE> "The specific test to run by case number"))
                .arg(arg!(-r --rand "Test against a random test case"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("remote")
                .about("sets the stash to branch main on the git remote")
                .arg(arg!(<REMOTE> "The git remote"))
                .arg(arg!(-f --force "Replaces the current git remote"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("restore")
                .about("restores the program to the version stashed away")
                .arg(arg!(<PROG> "The program to restore"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("run")
                .about("builds and executes target program")
                .arg(arg!(<PROG> "The program to run"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("show")
                .about("prints the input(s) or answer(s) to the test cases")
                .arg(arg!(<NAME> "The name of the quest"))
                .arg(arg!(-t --test <TEST> "The specific test to print by name"))
                .arg(arg!(-c --case <CASE> "The specific test to print by case number"))
                .arg(arg!(-r --rand "Print a random test case"))
                .arg(arg!(-a --ans "Print the answer instead of the input"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("stash")
                .about("stashes the program away for later")
                .arg(arg!(<PROG> "The program to stash"))
                .arg(arg!(-t --templ "Stashes the program away as a template"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("sync")
                .about("syncs the stash directory to match the remote")
                .arg(arg!(-f --force "Removes all local changes")),
        )
        .subcommand(
            Command::new("test")
                .about("runs program against sample test case")
                .arg(arg!(<PROG> "The program to test"))
                .arg(arg!(<IN> "The input file for the test case"))
                .arg(arg!(<ANS> "The answer file to the test case"))
                .arg_required_else_help(true),
        )
}

fn add(name: &str, url: &str, and_fetch: bool) -> Result<(), OwlError> {
    // this should always rewrite entries in the personal table
    // of the manifest TOML, which is the last table in the manifest
    // new entires can always be appended
    let mut manifest_path = fs_utils::ensure_dir_from_home(&[OWL_DIR])?;
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

            if lang.should_build() {
                let build_log = lang.build(prog)?;
                println!("{}", build_log.stdout);

                Ok(build_log.target)
            } else {
                Ok(prog.to_string())
            }
        }
        None => Ok(prog.to_string()),
    }
}

fn clear_stash(only_stash: bool, all_files: bool) -> Result<(), OwlError> {
    let owl_path = fs_utils::ensure_dir_from_home(&[OWL_DIR])?;

    for entry in fs::read_dir(check_path!(owl_path)?).map_err(|e| file_error!(e))? {
        let path = entry.map_err(|e| file_error!(e))?.path();
        let stem = path
            .file_stem()
            .and_then(OsStr::to_str)
            .ok_or(file_error!(check_path!(path)?))?;

        if path.is_dir()
            && (all_files
                || (stem == STASH_DIR && only_stash)
                || (stem != STASH_DIR && !only_stash))
        {
            fs_utils::remove_path(check_path!(path)?)?
        }
    }

    Ok(())
}

fn fetch(name: &str, dir: &str) -> Result<(), OwlError> {
    let mut manifest_path = fs_utils::ensure_dir_from_home(&[OWL_DIR])?;
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
    let mut fetch_path = fs_utils::ensure_dir_from_home(&[OWL_DIR])?;
    fetch_path.push(name);

    fetch(name, check_path!(fetch_path)?)
}

fn init_program(prog: &str) -> Result<(), OwlError> {
    if Path::new(prog).exists() {
        return Err(file_error!(format!("file already exists: '{}'", prog)));
    }

    let mut stash_path = fs_utils::ensure_dir_from_home(&[OWL_DIR, STASH_DIR])?;

    let ext = Path::new(prog)
        .extension()
        .and_then(OsStr::to_str)
        .ok_or(file_error!(prog))?;

    let stash_file = format!("{}.{}", TEMPLATE_STEM, ext);

    stash_path.push(stash_file);

    fs_utils::copy_file(check_path!(stash_path)?, prog)
}

fn push_git_remote(force: bool) -> Result<(), OwlError> {
    let mut stash_path = fs_utils::ensure_dir_from_home(&[OWL_DIR, STASH_DIR])?;
    stash_path.push(".git");

    if !stash_path.exists() {
        return Err(file_not_found!(check_path!(stash_path)?));
    }

    stash_path.pop();

    let stash_dir = check_path!(stash_path)?;

    let stdout = cmd_utils::git_add(stash_dir)?;
    println!("{}", stdout);

    let stdout = cmd_utils::git_commit(stash_dir)?;
    println!("{}", stdout);

    let stdout = cmd_utils::git_push(stash_dir, "origin", "main", force)?;
    println!("{}", stdout);

    let stdout = cmd_utils::git_status(stash_dir)?;
    println!("{}", stdout);

    Ok(())
}

fn quest(
    name: &str,
    prog: &str,
    test_name: Option<&String>,
    case_id: usize,
) -> Result<(), OwlError> {
    let mut quest_path = fs_utils::ensure_dir_from_home(&[OWL_DIR])?;
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

        if case_id > 0 && count != (case_id % total) {
            continue;
        }

        match quest_it(&target, &test_case, count, total) {
            Ok(true) => passed += 1,
            Ok(false) | Err(_) => failed += 1,
        }
    }

    println!("passed: {}, failed: {}", passed, failed);

    if target != prog {
        fs_utils::remove_path(&target)?;
    }

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

fn restore_program(prog: &str) -> Result<(), OwlError> {
    let mut stash_path = fs_utils::ensure_dir_from_home(&[OWL_DIR, STASH_DIR])?;
    stash_path.push(prog);

    fs_utils::copy_file(check_path!(stash_path)?, prog)
}

fn run(prog: &str) -> Result<(), OwlError> {
    if !Path::new(prog).exists() {
        return Err(file_not_found!(prog));
    }

    match prog_lang::check_prog_lang(prog) {
        Some(lang) => {
            let target = build_program(prog)?;

            let run_result = lang.run(&target);

            if target != prog {
                fs_utils::remove_path(&target)?;
            }

            run_result.map(|stdout| println!("{}", stdout))
        }
        None => {
            println!("{}", cmd_utils::run_binary(prog)?);
            Ok(())
        }
    }
}

fn set_git_remote(remote: &str, force: bool) -> Result<(), OwlError> {
    let mut stash_path = fs_utils::ensure_dir_from_home(&[OWL_DIR, STASH_DIR])?;
    stash_path.push(".git");

    if stash_path.exists() && !force {
        return Err(file_error!(".git directory already exists"));
    }

    if stash_path.exists() && force {
        fs_utils::remove_path(check_path!(stash_path)?)?;
    }

    stash_path.pop();

    let stash_dir = check_path!(stash_path)?;

    let stdout = cmd_utils::git_init(stash_dir)?;
    println!("{}", stdout);

    let stdout = cmd_utils::git_remote_add(stash_dir, "origin", remote)?;
    println!("{}", stdout);

    let stdout = cmd_utils::git_checkout(stash_dir, "main")?;
    println!("{}", stdout);

    let stdout = cmd_utils::git_status(stash_dir)?;
    println!("{}", stdout);

    Ok(())
}

fn sync_git_remote(force: bool) -> Result<(), OwlError> {
    let mut stash_path = fs_utils::ensure_dir_from_home(&[OWL_DIR, STASH_DIR])?;
    stash_path.push(".git");

    if !stash_path.exists() {
        return Err(file_not_found!(check_path!(stash_path)?));
    }

    stash_path.pop();

    let stash_dir = check_path!(stash_path)?;

    let stdout = cmd_utils::git_fetch(stash_dir, "origin", "main")?;
    println!("{}", stdout);

    if force {
        let stdout = cmd_utils::git_reset(stash_dir, "origin", "main")?;
        println!("{}", stdout);
    }

    let stdout = cmd_utils::git_pull(stash_dir, "origin", "main")?;
    println!("{}", stdout);

    let stdout = cmd_utils::git_status(stash_dir)?;
    println!("{}", stdout);

    Ok(())
}

fn show(
    name: &str,
    test_name: Option<&String>,
    case_id: usize,
    show_ans: bool,
) -> Result<(), OwlError> {
    let mut quest_path = fs_utils::ensure_dir_from_home(&[OWL_DIR])?;
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
        return show_it(&test_cases[(case_id - 1) & test_cases.len()], show_ans);
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

fn stash(prog: &str, as_templ: bool) -> Result<(), OwlError> {
    let mut stash_path = fs_utils::ensure_dir_from_home(&[OWL_DIR, STASH_DIR])?;

    if !as_templ {
        stash_path.push(prog);
        return fs_utils::copy_file(prog, check_path!(stash_path)?);
    }

    let ext = Path::new(prog)
        .extension()
        .and_then(OsStr::to_str)
        .ok_or(file_error!(prog))?;

    let stash_file = format!("{}.{}", TEMPLATE_STEM, ext);

    stash_path.push(stash_file);

    fs_utils::copy_file(prog, check_path!(stash_path)?)
}

fn test(prog: &str, in_file: &str, ans_file: &str) -> Result<(), OwlError> {
    match prog_lang::check_prog_lang(prog) {
        Some(_) => {
            let target = build_program(prog)?;

            let test_result = test_it(&target, in_file, ans_file);

            if target != prog {
                fs_utils::remove_path(&target)?;
            }

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
            let mut case = sub_matches
                .get_one::<String>("case")
                .map_or(0, |s| s.parse().expect("case id should be a number"));
            let rand = sub_matches.get_one::<bool>("rand").map_or(false, |&f| f);

            if rand {
                case = rand::random::<u64>() as usize;
            }

            if let Err(e) = quest(name, prog, test, case) {
                report_owl_err!(&e);
            }
        }
        Some(("show", sub_matches)) => {
            let name = sub_matches.get_one::<String>("NAME").expect("required");
            let test = sub_matches.get_one::<String>("test");
            let mut case = sub_matches
                .get_one::<String>("case")
                .map_or(0, |s| s.parse().expect("case id should be a number"));
            let rand = sub_matches.get_one::<bool>("rand").map_or(false, |&f| f);
            let ans = sub_matches.get_one::<bool>("ans").map_or(false, |&f| f);

            if rand {
                case = rand::random::<u64>() as usize;
            }

            if let Err(e) = show(name, test, case, ans) {
                report_owl_err!(&e);
            }
        }
        Some(("stash", sub_matches)) => {
            let prog = sub_matches.get_one::<String>("PROG").expect("required");
            let templ = sub_matches.get_one::<bool>("templ").map_or(false, |&f| f);

            if let Err(e) = stash(prog, templ) {
                report_owl_err!(&e);
            }
        }
        Some(("restore", sub_matches)) => {
            let prog = sub_matches.get_one::<String>("PROG").expect("required");

            if let Err(e) = restore_program(prog) {
                report_owl_err!(&e);
            }
        }
        Some(("init", sub_matches)) => {
            let prog = sub_matches.get_one::<String>("PROG").expect("required");

            if let Err(e) = init_program(prog) {
                report_owl_err!(&e);
            }
        }
        Some(("clean", sub_matches)) => {
            let stash = sub_matches.get_one::<bool>("stash").map_or(false, |&f| f);
            let all = sub_matches.get_one::<bool>("all").map_or(false, |&f| f);

            if let Err(e) = clear_stash(stash, all) {
                report_owl_err!(&e);
            }
        }
        Some(("remote", sub_matches)) => {
            let remote = sub_matches.get_one::<String>("REMOTE").expect("required");
            let force = sub_matches.get_one::<bool>("force").map_or(false, |&f| f);

            if let Err(e) = set_git_remote(remote, force) {
                report_owl_err!(&e);
            }
        }
        Some(("sync", sub_matches)) => {
            let force = sub_matches.get_one::<bool>("force").map_or(false, |&f| f);

            if let Err(e) = sync_git_remote(force) {
                report_owl_err!(&e);
            }
        }
        Some(("push", sub_matches)) => {
            let force = sub_matches.get_one::<bool>("force").map_or(false, |&f| f);

            if let Err(e) = push_git_remote(force) {
                report_owl_err!(&e);
            }
        }
        _ => unreachable!(),
    }
}
