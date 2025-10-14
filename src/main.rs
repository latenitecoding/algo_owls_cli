use clap::{Command, arg};
use std::collections::VecDeque;
use std::ffi::OsStr;
use std::fs::{self, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::Path;
use toml_edit::{DocumentMut, value};

mod owl_utils;
use owl_utils::{cmd_utils, fs_utils, prog_lang};

const OWL_DIR: &str = ".owl";
const MANIFEST: &str = ".manifest.toml";
const TMP_ARCHIVE: &str = ".tmp.zip";

const TOML_TEMPLATE: &str = r#"
[manifest]
version = "0.0.0"

[quests]

[personal]
"#;

macro_rules! command_not_found {
    ($expr:expr) => {
        Err(format!("command not found: {}", $expr))
    };
}

macro_rules! file_not_found {
    ($expr:expr) => {
        Err(format!(
            "'{}': No such file or directory (os error 2)",
            $expr
        ))
    };
}

macro_rules! report_err {
    ($expr:expr) => {
        eprintln!("\x1b[31m[owl error]\x1b[0m: {}", $expr);
    };
}

macro_rules! test_failure {
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
                .arg(arg!(<IN> "The input for the test case"))
                .arg(arg!(<ANS> "The answer to the test case"))
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

fn add(name: &str, url: &str) -> Result<(), String> {
    let mut manifest_path = dirs::home_dir().expect("should find home directory");

    manifest_path.push(OWL_DIR);

    if !manifest_path.exists() {
        fs::create_dir_all(&manifest_path).expect("should create owl directory");
    }

    manifest_path.push(MANIFEST);

    if !manifest_path.exists() {
        let manifest_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&manifest_path)
            .expect("should create manifest");

        let mut writer = BufWriter::new(manifest_file);

        let entry = format!("{} = \"{}\"\n", name, url);

        writer
            .write_all(TOML_TEMPLATE.as_bytes())
            .expect("should write TOML template to manifest");
        writer
            .write_all(entry.as_bytes())
            .expect("should write entry to manifest");
        writer.flush().expect("should flush data to manifest");

        return Ok(());
    }

    let toml_str =
        fs::read_to_string(&manifest_path).expect("should be able to read from manifest");
    let mut doc = toml_str
        .parse::<DocumentMut>()
        .expect("should parse manifest");

    if doc["personal"].get(name).is_none() {
        let manifest_file = OpenOptions::new()
            .append(true)
            .open(&manifest_path)
            .expect("should open manifest");

        let mut writer = BufWriter::new(manifest_file);

        let entry = format!("{} = \"{}\"\n", name, url);

        writer
            .write_all(entry.as_bytes())
            .expect("should write entry to manifest");
        writer.flush().expect("should flush data to manifest");

        return Ok(());
    }

    let manifest_file = OpenOptions::new()
        .write(true)
        .open(&manifest_path)
        .expect("should open manifest");

    let mut writer = BufWriter::new(manifest_file);

    doc["personal"][name] = value(url);

    writer
        .write_all(doc.to_string().as_bytes())
        .expect("should write to manifest");
    writer.flush().expect("should flush data to manifest");

    Ok(())
}

fn fetch(url: &str, dir: &str) -> Result<(), String> {
    fs_utils::download_file(url, TMP_ARCHIVE)?;
    fs_utils::extract_archive(TMP_ARCHIVE, dir)?;

    fs_utils::remove_path(TMP_ARCHIVE)
}

fn fetch_by_name(name: &str, dir: &str) -> Result<(), String> {
    let mut manifest_path = dirs::home_dir().expect("should find home directory");
    manifest_path.push(OWL_DIR);
    manifest_path.push(MANIFEST);

    if !manifest_path.exists() {
        return file_not_found!(manifest_path.to_str().expect("should read manifest path"));
    }

    let toml_str =
        fs::read_to_string(&manifest_path).expect("should be able to read from manifest");
    let doc = toml_str
        .parse::<DocumentMut>()
        .expect("should parse manifest");

    let entry = doc["quests"].get(name).or(doc["personal"].get(name));

    if entry.is_none() {
        return Err(format!("No manifest entry found: '{}'", name));
    }

    let url = entry
        .expect("unreachable")
        .as_value()
        .expect("should have entry in manifest")
        .as_str()
        .expect("should parse entry in manifest");

    fetch(url, dir)
}

fn quest(name: &str, prog: &str) -> Result<(), String> {
    let mut quest_path = dirs::home_dir().expect("should find home directory");
    quest_path.push(OWL_DIR);
    quest_path.push(name);

    let quest_dir = quest_path
        .to_str()
        .expect("should parse quest path")
        .to_string();

    if !quest_path.exists() {
        fetch_by_name(name, &quest_dir)?;
    }

    let mut test_cases: Vec<String> = Vec::new();

    let mut queue: VecDeque<String> = VecDeque::new();
    queue.push_back(quest_dir);

    while let Some(dir) = queue.pop_front() {
        for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
            let path = entry.map_err(|e| e.to_string())?.path();

            if path.is_dir() {
                queue.push_back(path.to_str().expect("should parse path").to_string());
            }
            if path.is_file()
                && let Some(ext) = path.extension().and_then(OsStr::to_str)
                && ext == "in"
            {
                test_cases.push(path.to_str().expect("should parse path").to_string());
            }
        }
    }

    let prog_path = Path::new(prog);

    if !prog_path.exists() {
        return file_not_found!(prog);
    }

    let target = match prog_path.extension().and_then(OsStr::to_str) {
        Some(ext) => {
            let lang = prog_lang::get_prog_lang(ext)?;

            if !lang.command_exists() {
                return command_not_found!(lang.name());
            }

            let build_log = lang.build(prog)?;
            println!("{}", build_log.stdout);

            build_log.target
        }
        None => prog.to_string(),
    };

    let total = test_cases.len();
    let mut passed = 0;
    let mut failed = 0;

    for test_case in test_cases {
        let in_path = Path::new(&test_case);
        let mut ans_path = in_path
            .parent()
            .expect("should parse parent dir")
            .to_path_buf();

        let stem = format!(
            "{}.ans",
            in_path
                .file_stem()
                .and_then(OsStr::to_str)
                .expect("should parse file stem")
        );
        ans_path.push(&stem);

        let ans_file = &ans_path.to_str().expect("should parse ans path");

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

    fs_utils::remove_path(&target).expect("should remove build target");

    if failed > 0 {
        Err("test failures".to_owned())
    } else {
        println!("\x1b[32mall tests passed\x1b[0m 🏆🏆🏆\n");
        Ok(())
    }
}

fn quest_it(target: &str, in_file: &str, ans_file: &str) -> Result<(), String> {
    let prog_path = Path::new(target);
    let in_path = Path::new(in_file);
    let ans_path = Path::new(ans_file);

    if !prog_path.exists() {
        return file_not_found!(target);
    }
    if !in_path.exists() {
        return file_not_found!(in_file);
    }
    if !ans_path.exists() {
        return file_not_found!(ans_file);
    }

    let stdin = fs::read_to_string(in_path).expect("should read from in file");
    let ans = fs::read_to_string(ans_path).expect("should read from ans file");

    match prog_path.extension().and_then(OsStr::to_str) {
        Some(ext) => {
            let lang = prog_lang::get_prog_lang(ext)?;

            if !lang.command_exists() {
                return command_not_found!(lang.name());
            }

            let run_result = lang.run_with_stdin(target, &stdin);

            run_result.and_then(|actual| {
                if actual == ans {
                    Ok(())
                } else {
                    test_failure!(in_file, ans, actual);
                    Err("failed test".to_owned())
                }
            })
        }
        None => cmd_utils::run_binary_with_stdin(target, &stdin).and_then(|actual| {
            if actual == ans {
                Ok(())
            } else {
                test_failure!(in_file, ans, actual);
                Err("failed test".to_owned())
            }
        }),
    }
}

fn run(prog: &str) -> Result<(), String> {
    let path = Path::new(prog);

    if !path.exists() {
        return file_not_found!(prog);
    }

    match path.extension().and_then(OsStr::to_str) {
        Some(ext) => {
            let lang = prog_lang::get_prog_lang(ext)?;

            if !lang.command_exists() {
                return command_not_found!(lang.name());
            }

            let build_log = lang.build(prog)?;
            println!("{}", build_log.stdout);

            let run_result = lang.run(&build_log.target);

            fs_utils::remove_path(&build_log.target).expect("should remove build target");

            run_result.map(|stdout| println!("{}", stdout))
        }
        None => {
            println!("{}", cmd_utils::run_binary(prog)?);
            Ok(())
        }
    }
}

fn test(prog: &str, in_file: &str, ans_file: &str) -> Result<(), String> {
    let prog_path = Path::new(prog);

    match prog_path.extension().and_then(OsStr::to_str) {
        Some(ext) => {
            let lang = prog_lang::get_prog_lang(ext)?;

            if !lang.command_exists() {
                return command_not_found!(lang.name());
            }

            let build_log = lang.build(prog)?;
            println!("{}", build_log.stdout);

            let test_result = quest_it(&build_log.target, in_file, ans_file);

            fs_utils::remove_path(&build_log.target).expect("should remove test target");

            test_result.map_err(|_| "test failures".to_string())
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
                report_err!(&e);
            }
        }
        Some(("fetch", sub_matches)) => {
            let url = sub_matches.get_one::<String>("URL").expect("required");
            let dir = sub_matches.get_one::<String>("DIR").expect("required");

            if let Err(e) = fetch(url, dir) {
                report_err!(&e);
            }
        }
        Some(("test", sub_matches)) => {
            let prog = sub_matches.get_one::<String>("PROG").expect("required");
            let in_file = sub_matches.get_one::<String>("IN").expect("required");
            let ans_file = sub_matches.get_one::<String>("ANS").expect("required");

            if let Err(e) = test(prog, in_file, ans_file) {
                report_err!(&e);
            }
        }
        Some(("add", sub_matches)) => {
            let name = sub_matches.get_one::<String>("NAME").expect("required");
            let url = sub_matches.get_one::<String>("URL").expect("required");

            if let Err(e) = add(name, url) {
                report_err!(&e);
            }
        }
        Some(("quest", sub_matches)) => {
            let name = sub_matches.get_one::<String>("NAME").expect("required");
            let prog = sub_matches.get_one::<String>("PROG").expect("required");

            if let Err(e) = quest(name, prog) {
                report_err!(&e);
            }
        }
        _ => unreachable!(),
    }
}
