use clap::{arg, Command};
use std::ffi::OsStr;
use std::fs::File;
use std::io::copy;
use std::path::Path;

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
                .arg(arg!(<OUT> "The local file to copy to"))
                .arg_required_else_help(true),
        )
}

fn clean(exe: &str) -> Result<(), String> {
    let output = std::process::Command::new("rm")
        .arg(exe)
        .output()
        .expect("should be able to remove executable");

    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

fn fetch(url: &str, out: &str) -> Result<(), String> {
    let mut resp = reqwest::blocking::get(url).map_err(|e| e.to_string())?;
    let mut file = File::create(out).map_err(|e| e.to_string())?;
    copy(&mut resp, &mut file).map_err(|e| e.to_string())?;
    Ok(())
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
            clean(&exe)
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
            let out = sub_matches
                .get_one::<String>("OUT")
                .expect("required");

            if let Err(e) = fetch(url, out) {
                report_err!(&e);
            }
        },
        _ => unreachable!(),
    }
}
