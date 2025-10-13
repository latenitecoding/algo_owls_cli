use clap::{arg, Command};
use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::io::copy;
use std::path::Path;
use zip::ZipArchive;

mod prog_lang;
use prog_lang::{get_prog_lang, run_binary};

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

fn remove_path(file_or_dir: &str) -> Result<(), String> {
    let path = Path::new(file_or_dir);
    let metadata = fs::metadata(path).map_err(|e| e.to_string())?;

    if metadata.is_dir() {
        fs::remove_dir_all(path).map_err(|e| e.to_string())?;
    } else if metadata.is_file() {
        fs::remove_file(path).map_err(|e| e.to_string())?;
    }

    Ok(())
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
    let mut resp = reqwest::blocking::get(url).map_err(|e| e.to_string())?;
    let mut file = File::create(".tmp.zip").map_err(|e| e.to_string())?;
    copy(&mut resp, &mut file).map_err(|e| e.to_string())?;

    let zip_file = File::open(".tmp.zip").map_err(|e| e.to_string())?;
    let mut archive = ZipArchive::new(zip_file).map_err(|e| e.to_string())?;
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    archive.extract(dir).map_err(|e| e.to_string())?;

    remove_path(".tmp.zip")
}

fn run(prog: &str) -> Result<(), String> {
    let path = Path::new(prog);

    if !path.exists() {
        return file_not_found!(prog);
    }

    match path.extension().and_then(OsStr::to_str) {
        Some(ext) => {
            let lang = get_prog_lang(ext)?;

            if !lang.command_exists() {
                return command_not_found!(lang.name());
            }

            let exe = lang.build(prog)?;

            println!("{}", lang.run(&exe)?);
            remove_path(&exe)
        },
        None => {
            println!("{}", run_binary(prog)?);
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
