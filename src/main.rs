use clap::{arg, Command};
use std::ffi::OsStr;
use std::path::Path;

mod owl_utils;
use owl_utils::{cmd_utils, fs_utils, prog_lang};

const TMP_ARCHIVE: &'static str = ".tmp.zip";

macro_rules! command_not_found {
    ($expr:expr) => {
        Err(format!("command not found: {}", $expr))
    }
}

macro_rules! file_not_found {
    ($expr:expr) => {
        Err(format!("'{}': No such file or directory (os error 2)", $expr))
    }
}

macro_rules! report_err {
    ($expr:expr) => {
        eprintln!("\x1b[31m[owl error]\x1b[0m: {}", $expr);
    }
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
}

fn fetch(url: &str, dir: &str) -> Result<(), String> {
    fs_utils::download_file(url, TMP_ARCHIVE)?;
    fs_utils::extract_archive(TMP_ARCHIVE, dir)?;

    fs_utils::remove_path(TMP_ARCHIVE)
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

            let exe = lang.build(prog)?;

            println!("{}", lang.run(&exe)?);
            fs_utils::remove_path(&exe)
        },
        None => {
            println!("{}", cmd_utils::run_binary(prog)?);
            Ok(())
        }
    }
}

fn test(prog: &str, dir: &str) -> Result<(), String> {
    let path = Path::new(prog);
    let dir_path = Path::new(dir);

    if !path.exists() {
        return file_not_found!(prog);
    }
    if !dir_path.exists() {
        return file_not_found!(dir);
    }

    match path.extension().and_then(OsStr::to_str) {
        Some(ext) => {
            let lang = prog_lang::get_prog_lang(ext)?;

            if !lang.command_exists() {
                return command_not_found!(lang.name());
            }

            let exe = lang.build(prog)?;

            println!("{}", lang.run(&exe)?);
            fs_utils::remove_path(&exe)
        },
        None => {
            println!("{}", cmd_utils::run_binary(prog)?);
            Ok(())
        }
    }
}

fn main() {
    let matches = cli().get_matches();

    match matches.subcommand() {
        Some(("run", sub_matches)) => {
            let prog = sub_matches
                .get_one::<String>("PROG")
                .expect("required");

            if let Err(e) = run(prog) {
                report_err!(&e);
            }
        },
        Some(("fetch", sub_matches)) => {
            let url = sub_matches
                .get_one::<String>("URL")
                .expect("required");
            let dir = sub_matches
                .get_one::<String>("DIR")
                .expect("required");

            if let Err(e) = fetch(url, dir) {
                report_err!(&e);
            }
        },
        _ => unreachable!(),
    }
}
