use clap::{arg, Command};
use std::ffi::OsStr;
use std::path::Path;

mod prog_lang;
use prog_lang::get_prog_lang;

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
            Ok(())
        },
        None => {
            println!("Run './{}'", prog);
            Ok(())
        }
    }
}

fn report_err(msg: &str) {
    eprintln!("\x1b[31m[owl error]\x1b[0m: {}", msg);
}

fn main() {
    let matches = cli().get_matches();

    match matches.subcommand() {
        Some(("run", sub_matches)) => {
            let prog = sub_matches
                .get_one::<String>("PROG")
                .expect("required");
            if let Err(e) = run(prog) {
                report_err(&e);
            }
        },
        _ => unreachable!(),
    }
}
