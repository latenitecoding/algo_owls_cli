use clap::{Arg, ArgAction, Command, arg};
use std::cmp::Ordering;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process;
use url::Url;

mod common;
use common::OwlError;

mod owl_core;
use owl_core::ReviewPrompt;

mod owl_utils;
use owl_utils::{
    FileExplorerApp, PromptMode, Uri, cmd_utils, fs_utils, prog_utils, toml_utils, tui_utils,
};

use crate::owl_utils::FileApp;

const CHAT_DIR: &str = ".chat";
const GIT_DIR: &str = ".git";
const MANIFEST: &str = ".manifest.toml";
const MANIFEST_HEAD_URL: &str = "https://gist.githubusercontent.com/latenitecoding/84c043f4c9092998773640a2202f2d36/raw/owl_manifest_short";
const MANIFEST_URL: &str = "https://gist.githubusercontent.com/latenitecoding/b6fdd8656c0b6a60795581f84d0f2fa4/raw/owlgo_manifest";
const OWL_DIR: &str = ".owlgo";
const PROMPT_DIR: &str = ".prompt";
const PROMPT_FILE: &str = ".prompt.md";
const TEMPLATE_STEM: &str = ".template";
const TMP_ARCHIVE: &str = ".tmp.zip";
const STASH_DIR: &str = ".stash";

// it must be that [manifest] is at the top
const TOML_TEMPLATE: &str = r#"
[manifest]
version = "0.1.5"
timestamp = "0.0.0"
ai_sdk = "claude"
api_key = ""

[extensions]

[ext_uri]

[personal_quests]

[prompts]

[quests]
"#;

macro_rules! report_owl_err {
    ($expr:expr) => {
        eprintln!("\x1b[31m[owlgo error]\x1b[0m: {}", $expr);
        process::exit(1);
    };
}

fn cli() -> Command {
    Command::new("owlgo")
        .about("A lightweight CLI to assist in solving CP problems")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("add")
                .about("adds new personal quest/extension/prompt to the manifest")
                .arg(arg!(<NAME> "The name of the quest/extension/prompt"))
                .arg(arg!(<URI> "The URL/PATH to fetch from"))
                .arg(arg!(-F --fetch "Fetches test cases and prompts"))
                .arg(arg!(-e --ext "The URL is an extension to be committed"))
                .arg(arg!(-P --prompt "The URL is a manifest to be committed"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("clear")
                .about("removes test cases and/or stashed files")
                .arg(
                    arg!(-s --stash "Removes all stashed programs/prompts/chats (and the git dir)"),
                )
                .arg(arg!(-p --program "Removes all stashed programs"))
                .arg(arg!(-P --prompt "Removes all stashed prompts"))
                .arg(arg!(-C --chat "Removes LLM chat history"))
                .arg(arg!(-m --manifest "Removes the manifest"))
                .arg(arg!(-k --keep "Tests are not cleared"))
                .arg(arg!(--all "Removes everything not excluded by other flags")),
        )
        .subcommand(
            Command::new("fetch")
                .about("fetches quests/extensions/prompts to your machine")
                .arg(arg!(<NAME> "The name of the quest/extension/prompt"))
                .arg(arg!(-e --ext "The name is a manifest extension"))
                .arg(arg!(-P --prompt "The name is a prompt"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("git")
                .about("provides git integration from within the stash directory")
                .subcommand(
                    Command::new("push")
                        .about("pushes all stashed files to the remote")
                        .arg(arg!(-f --force "Forces the remote to match the local stash")),
                )
                .subcommand(
                    Command::new("remote")
                        .about("sets the stash to branch main on the git remote")
                        .arg(arg!(<REMOTE> "The git remote"))
                        .arg(arg!(-f --force "Replaces the current git remote"))
                        .arg_required_else_help(true),
                )
                .subcommand(
                    Command::new("sync")
                        .about("syncs the stash directory to match the remote")
                        .arg(arg!(-f --force "Removes all local changes")),
                )
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("init")
                .about("creates a local file from a stashed template")
                .arg(arg!(<PROG> "The program to initialize from the template"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("list")
                .about("outputs information on stashed files")
                .arg(arg!(-I --tui "Enters an interactive TUI to preview files"))
                .arg(arg!(-O --root "List starting from the root of the owlgo directory"))
                .arg(arg!(-P --prompt "List starting in the prompt directory"))
                .arg(arg!(-C --chat "List starting in the chat directory")),
        )
        .subcommand(
            Command::new("quest")
                .about("tests program against all test cases in the selected quest")
                .arg(arg!(<NAME> "The name of the quest"))
                .arg(arg!(<PROG> "The program to test"))
                .arg(arg!(-t --test <TEST> "The specific test to run by name"))
                .arg(arg!(-c --case <CASE> "The specific test to run by case number"))
                .arg(arg!(-r --rand "Test against a random test case"))
                .arg(arg!(--hints "Prints the hint(s)/feedback (if any)"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("restore")
                .about("restores the file/program to the version stashed away")
                .arg(arg!(<PROG> "The file/program to restore"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("review")
                .about("submits the program to an LLM for a code review")
                .arg(arg!(<PROG> "The program to review"))
                .arg(arg!([PROMPT] "The prompt or description to give"))
                .arg(arg!(--sdk <SDK> "Updates the chosen LLM sdk (e.g, 'claude')"))
                .arg(arg!(--key <KEY> "Updates the API key for the chosen LLM"))
                .arg(arg!(-I --tui "Enters an interactive TUI to chat with chosen LLM"))
                .arg(Arg::new("file")
                    .short('f')
                    .long("file")
                    .action(ArgAction::SetTrue)
                    .help("The prompt/desc is in a file")
                    .conflicts_with_all(["quest", "stash"])
                )
                .arg(Arg::new("quest")
                    .short('q')
                    .long("quest")
                    .action(ArgAction::SetTrue)
                    .help("The prompt/desc is related to a specific set of test cases")
                    .conflicts_with_all(["file", "stash"])
                )
                .arg(Arg::new("stash")
                    .short('s')
                    .long("stash")
                    .action(ArgAction::SetTrue)
                    .help("The prompt/desc is from stash")
                    .conflicts_with_all(["file", "quest"])
                )
                .arg(arg!(-F --forget "Forget chat history after each prompt"))
                .arg(Arg::new("debug")
                    .short('D')
                    .long("debug")
                    .action(ArgAction::SetTrue)
                    .help("Prompt for debugging help")
                    .conflicts_with_all(["default", "explain", "explore", "optimize", "test"])
                )
                .arg(Arg::new("default")
                    .short('d')
                    .long("def")
                    .action(ArgAction::SetTrue)
                    .help("Use the default prompt")
                    .conflicts_with_all(["debug", "explain", "explore", "optimize", "test"])
                )
                .arg(Arg::new("explain")
                    .short('x')
                    .long("explain")
                    .action(ArgAction::SetTrue)
                    .help("Prompt for help with the problem description")
                    .conflicts_with_all(["debug", "default", "explore", "optimize", "test"])
                )
                .arg(Arg::new("explore")
                    .short('X')
                    .long("explore")
                    .action(ArgAction::SetTrue)
                    .help("Prompt for alternative implementation")
                    .conflicts_with_all(["debug", "default", "explain", "optimize", "test"])
                )
                .arg(Arg::new("optimize")
                    .short('z')
                    .long("opt")
                    .action(ArgAction::SetTrue)
                    .help("Prompt for optimization help")
                    .conflicts_with_all(["debug", "default", "explain", "explore", "test"])
                )
                .arg(Arg::new("test")
                    .short('t')
                    .long("test")
                    .action(ArgAction::SetTrue)
                    .help("Prompt for help identifying tests and edge cases")
                    .conflicts_with_all(["debug", "default", "explain", "explore", "optimize"])
                )
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
                .about("prints test input/expected or stashed files")
                .arg(arg!([NAME] "The name of the quest/solution/program/prompt"))
                .arg(arg!(-I --tui "Show the file in a TUI (redirects to list if no other args are provided)"))
                .arg(arg!(-t --test <TEST> "The specific test to print by name"))
                .arg(arg!(-c --case <CASE> "The specific test to print by case number"))
                .arg(arg!(-r --rand "Print a random test case"))
                .arg(arg!(-a --ans "Print the answer instead of the input"))
                .arg(arg!(-p --program "Show a stashed program instead of a test case"))
                .arg(arg!(-P --prompt "Show a stashed prompt instead of a test case"))
                .arg(arg!(-m --manifest "Show the manifest"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("stash")
                .about("stashes the program/prompt/file away for later")
                .arg(arg!(<PROG> "The program/prompt/file to stash"))
                .arg(arg!(-T --templ "Stashes the program away as a template"))
                .arg(arg!(-P --prompt "Stashes the file away as a prompt"))
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
        .subcommand(Command::new("update").about("checks owlgo and its manifest for updates"))
        .subcommand(
            Command::new("version")
                .about("outputs the current version")
                .arg(arg!(-L --lang <EXT> "Outputs the system version of the language")),
        )
}

#[tokio::main]
async fn main() {
    let matches = cli().get_matches();

    match matches.subcommand() {
        Some(("add", sub_matches)) => {
            let name = sub_matches.get_one::<String>("NAME").expect("required");
            let uri_str = sub_matches.get_one::<String>("URI").expect("required");
            let and_fetch = sub_matches.get_one::<bool>("fetch").is_some_and(|&f| f);
            let is_extension = sub_matches.get_one::<bool>("ext").is_some_and(|&f| f);
            let is_prompt = sub_matches.get_one::<bool>("prompt").is_some_and(|&f| f);

            let uri = Uri::try_from(uri_str.as_str()).expect("provided URI is valid");

            let action = if is_extension {
                owl_core::add_extension(name, &uri, and_fetch).await
            } else if is_prompt {
                owl_core::add_prompt(name, &uri, and_fetch).await
            } else {
                owl_core::add_quest(name, &uri, and_fetch).await
            };

            if let Err(e) = action {
                report_owl_err!(e);
            }
        }
        Some(("clear", sub_matches)) => {
            let do_stash = sub_matches.get_one::<bool>("stash").is_some_and(|&f| f);
            let do_programs = sub_matches.get_one::<bool>("program").is_some_and(|&f| f);
            let do_prompts = sub_matches.get_one::<bool>("prompt").is_some_and(|&f| f);
            let do_chat = sub_matches.get_one::<bool>("chat").is_some_and(|&f| f);
            let do_manif = sub_matches.get_one::<bool>("manifest").is_some_and(|&f| f);
            let keep_tests = sub_matches.get_one::<bool>("keep").is_some_and(|&f| f);
            let do_all = sub_matches.get_one::<bool>("all").is_some_and(|&f| f);

            let action = fs_utils::ensure_path_from_home(&[OWL_DIR], None)
                .and_then(|owl_dir| {
                    let mut manifest_path = owl_dir.clone();
                    manifest_path.push(MANIFEST);

                    if (do_all || do_manif) && manifest_path.exists() {
                        fs_utils::remove_path(&manifest_path)?;
                    }

                    Ok(owl_dir)
                })
                .and_then(|owl_dir| {
                    let mut stash_dir = owl_dir.clone();
                    stash_dir.push(STASH_DIR);

                    if (do_all || do_stash) && stash_dir.exists() {
                        fs_utils::remove_path(&stash_dir)?;
                    }

                    Ok(stash_dir)
                })
                .and_then(|stash_dir| {
                    let mut chat_dir = stash_dir.clone();
                    chat_dir.push(CHAT_DIR);

                    if !do_all && !do_stash && do_chat && chat_dir.exists() {
                        fs_utils::remove_path(&chat_dir)?;
                    }

                    Ok(stash_dir)
                })
                .and_then(|stash_dir| {
                    let mut prompt_dir = stash_dir.clone();
                    prompt_dir.push(PROMPT_DIR);

                    if !do_all && !do_stash && do_prompts && prompt_dir.exists() {
                        fs_utils::remove_path(&prompt_dir)?;
                    }

                    Ok(())
                })
                .and_then(|_| {
                    if !do_all && !do_stash && do_programs {
                        owl_core::clear_programs()?
                    }

                    if !keep_tests {
                        owl_core::clear_quests()?;
                    }

                    Ok(())
                });

            if let Err(e) = action {
                report_owl_err!(e);
            }
        }
        Some(("fetch", sub_matches)) => {
            let name = sub_matches.get_one::<String>("NAME").expect("required");
            let is_ext = sub_matches.get_one::<bool>("ext").is_some_and(|&f| f);
            let is_prompt = sub_matches.get_one::<bool>("prompt").is_some_and(|&f| f);

            let action = if is_ext {
                owl_core::fetch_extension(name).await
            } else if is_prompt {
                owl_core::fetch_prompt(name).await
            } else {
                owl_core::fetch_quest(name).await
            };

            if let Err(e) = action {
                report_owl_err!(e);
            }
        }
        Some(("git", sub_matches)) => match sub_matches.subcommand() {
            Some(("push", sub_matches)) => {
                let use_force = sub_matches.get_one::<bool>("force").is_some_and(|&f| f);

                if let Err(e) = owl_core::push_git_remote(use_force) {
                    report_owl_err!(e);
                }
            }
            Some(("remote", sub_matches)) => {
                let remote = sub_matches.get_one::<String>("REMOTE").expect("required");
                let use_force = sub_matches.get_one::<bool>("force").is_some_and(|&f| f);

                if let Err(e) = owl_core::set_git_remote(remote, use_force) {
                    report_owl_err!(e);
                }
            }
            Some(("sync", sub_matches)) => {
                let use_force = sub_matches.get_one::<bool>("force").is_some_and(|&f| f);

                if let Err(e) = owl_core::sync_git_remote(use_force) {
                    report_owl_err!(e);
                }
            }
            _ => unreachable!(),
        },
        Some(("init", sub_matches)) => {
            let prog = sub_matches.get_one::<String>("PROG").expect("required");

            let prog_path = Path::new(prog);

            if prog_path.exists() {
                let e = OwlError::FileError(
                    format!(
                        "'{}': file already exists in stash",
                        prog_path.to_string_lossy()
                    ),
                    "".into(),
                );

                report_owl_err!(e);
            }

            let action = prog_path
                .extension()
                .and_then(OsStr::to_str)
                .ok_or(OwlError::UriError(
                    format!("'{}': has no file extension", prog_path.to_string_lossy()),
                    "None".into(),
                ))
                .map(|ext| format!("{}.{}", TEMPLATE_STEM, ext))
                .and_then(|file_str| {
                    fs_utils::ensure_path_from_home(&[OWL_DIR, STASH_DIR], Some(&file_str))
                })
                .map(|path| fs_utils::copy_file(&path, prog_path));

            if let Err(e) = action {
                report_owl_err!(e);
            }
        }
        Some(("list", sub_matches)) => {
            let start_from_root = sub_matches.get_one::<bool>("root").is_some_and(|&f| f);
            let start_from_prompt = sub_matches.get_one::<bool>("prompt").is_some_and(|&f| f);
            let start_from_chat = sub_matches.get_one::<bool>("chat").is_some_and(|&f| f);
            let use_tui = sub_matches.get_one::<bool>("tui").is_some_and(|&f| f);

            let target_dir = if start_from_root {
                fs_utils::ensure_path_from_home(&[OWL_DIR], None).expect("owlgo dir exists")
            } else if start_from_prompt {
                fs_utils::ensure_path_from_home(&[OWL_DIR, STASH_DIR, PROMPT_DIR], None)
                    .expect("prompt dir exists")
            } else if start_from_chat {
                fs_utils::ensure_path_from_home(&[OWL_DIR, STASH_DIR, CHAT_DIR], None)
                    .expect("chat dir exists")
            } else {
                fs_utils::ensure_path_from_home(&[OWL_DIR, STASH_DIR], None)
                    .expect("stash dir exists")
            };

            let action = if use_tui {
                tui_utils::enter_raw_mode().and_then(|_| {
                    match FileExplorerApp::default().run(&target_dir) {
                        Ok(_) => tui_utils::exit_raw_mode(),
                        Err(e) => tui_utils::exit_raw_mode().and(Err(e)),
                    }
                })
            } else {
                cmd_utils::tree_dir(&target_dir).or_else(|_| {
                    let dir_str = target_dir
                        .to_str()
                        .map(String::from)
                        .unwrap_or(target_dir.to_string_lossy().to_string());

                    fs_utils::dir_tree(&target_dir)
                        .map(|files| {
                            files
                                .into_iter()
                                .map(|file| {
                                    file.to_str().unwrap_or(&file.to_string_lossy()).to_string()
                                })
                                .collect::<Vec<String>>()
                                .join("\n")
                        })
                        .map(|stdout| println!("{}\n{}", dir_str, stdout))
                })
            };

            if let Err(e) = action {
                report_owl_err!(e);
            }
        }
        Some(("quest", sub_matches)) => {
            let name = sub_matches.get_one::<String>("NAME").expect("required");
            let prog = sub_matches.get_one::<String>("PROG").expect("required");
            let test = sub_matches.get_one::<String>("test");
            let mut case = sub_matches
                .get_one::<String>("case")
                .map(|s| s.parse::<usize>().expect("case argument is type usize"));
            let rand = sub_matches.get_one::<bool>("rand").is_some_and(|&f| f);
            let use_hints = sub_matches.get_one::<bool>("hints").is_some_and(|&f| f);

            if rand {
                case = Some(rand::random::<u64>() as usize);
            }

            let action = match test {
                Some(test_name) => {
                    owl_core::quest_once(name, Path::new(prog), test_name, use_hints).await
                }
                None => owl_core::quest(name, Path::new(prog), case, use_hints).await,
            };

            if let Err(e) = action {
                report_owl_err!(e);
            }
        }
        Some(("restore", sub_matches)) => {
            let prog = sub_matches.get_one::<String>("PROG").expect("required");
            let prog_path = Path::new(prog);

            let action = prog_path
                .file_name()
                .and_then(OsStr::to_str)
                .ok_or(OwlError::UriError(
                    format!("'{}': has no filename", prog_path.to_string_lossy()),
                    "None".into(),
                ))
                .and_then(|file_name| {
                    let stash_path =
                        fs_utils::ensure_path_from_home(&[OWL_DIR, STASH_DIR], Some(file_name))?;

                    fs_utils::copy_file(&stash_path, prog_path)
                });

            if let Err(e) = action {
                report_owl_err!(e);
            }
        }
        Some(("review", sub_matches)) => {
            let prog = sub_matches.get_one::<String>("PROG").expect("required");
            let prompt = sub_matches
                .get_one::<String>("PROMPT")
                .map(String::to_owned);

            let ai_sdk = sub_matches.get_one::<String>("sdk");
            let api_key = sub_matches.get_one::<String>("key");

            let use_tui = sub_matches.get_one::<bool>("tui").is_some_and(|&f| f);

            let is_file = sub_matches.get_one::<bool>("file").is_some_and(|&f| f);
            let in_quest = sub_matches.get_one::<bool>("quest").is_some_and(|&f| f);
            let in_stash = sub_matches.get_one::<bool>("stash").is_some_and(|&f| f);

            let do_forget = sub_matches.get_one::<bool>("forget").is_some_and(|&f| f);

            let use_default = sub_matches.get_one::<bool>("default").is_some_and(|&f| f);
            let use_debug = sub_matches.get_one::<bool>("debug").is_some_and(|&f| f);
            let use_explain = sub_matches.get_one::<bool>("explain").is_some_and(|&f| f);
            let use_explore = sub_matches.get_one::<bool>("explore").is_some_and(|&f| f);
            let use_opt = sub_matches.get_one::<bool>("optimize").is_some_and(|&f| f);
            let use_test = sub_matches.get_one::<bool>("test").is_some_and(|&f| f);

            if ai_sdk.is_some() || api_key.is_some() {
                let action = fs_utils::ensure_path_from_home(&[OWL_DIR], Some(MANIFEST)).and_then(
                    |manifest_path| {
                        let mut manifest_doc = if manifest_path.exists() {
                            toml_utils::read_toml(&manifest_path)?
                        } else {
                            TOML_TEMPLATE
                                .parse::<toml_edit::DocumentMut>()
                                .map_err(|e| {
                                    OwlError::TomlError(
                                        "Failed to parse TOML template".into(),
                                        e.to_string(),
                                    )
                                })?
                        };

                        if let Some(sdk) = ai_sdk {
                            manifest_doc["manifest"]["ai_sdk"] = toml_edit::value(sdk);
                        }

                        if let Some(key) = api_key {
                            manifest_doc["manifest"]["api_key"] = toml_edit::value(key);
                        }

                        toml_utils::write_manifest(&manifest_doc, &manifest_path)
                    },
                );

                if let Err(e) = action {
                    report_owl_err!(e);
                }
            }

            let mode = if use_debug {
                PromptMode::Debug
            } else if use_explain {
                PromptMode::Explain
            } else if use_explore {
                PromptMode::Explore
            } else if use_opt {
                PromptMode::Optimize
            } else if use_test {
                PromptMode::Test
            } else if prompt.is_some() && !use_default {
                PromptMode::Custom
            } else {
                PromptMode::Default
            };

            let check_prompt = prompt.map(|prompt_str| {
                if is_file {
                    ReviewPrompt::IsFile(PathBuf::from(prompt_str))
                } else if in_stash {
                    ReviewPrompt::InStash(prompt_str)
                } else if in_quest {
                    ReviewPrompt::InQuest(prompt_str)
                } else {
                    ReviewPrompt::UserPrompt(prompt_str)
                }
            });

            if let Err(e) =
                owl_core::review_program(Path::new(prog), check_prompt, mode, do_forget, use_tui)
                    .await
            {
                report_owl_err!(e);
            }
        }
        Some(("run", sub_matches)) => {
            let prog = sub_matches.get_one::<String>("PROG").expect("required");

            if let Err(e) = owl_core::run_program(Path::new(prog)) {
                report_owl_err!(e);
            }
        }
        Some(("show", sub_matches)) => {
            let test = sub_matches.get_one::<String>("test");
            let mut case = sub_matches
                .get_one::<String>("case")
                .map(|s| s.parse::<usize>().expect("case argument is type usize"));
            let rand = sub_matches.get_one::<bool>("rand").is_some_and(|&f| f);
            let show_ans = sub_matches.get_one::<bool>("ans").is_some_and(|&f| f);
            let show_program = sub_matches.get_one::<bool>("program").is_some_and(|&f| f);
            let show_prompt = sub_matches.get_one::<bool>("prompt").is_some_and(|&f| f);
            let show_manifest = sub_matches.get_one::<bool>("manifest").is_some_and(|&f| f);
            let use_tui = sub_matches.get_one::<bool>("tui").is_some_and(|&f| f);

            let action = if show_program || show_prompt || show_manifest {
                let path = if show_manifest {
                    fs_utils::ensure_path_from_home(&[OWL_DIR], Some(MANIFEST))
                        .expect("manifest exists")
                } else {
                    let name = sub_matches.get_one::<String>("NAME").expect("required");

                    if show_program {
                        fs_utils::ensure_path_from_home(&[OWL_DIR, STASH_DIR], Some(name))
                            .expect("program exists")
                    } else {
                        fs_utils::ensure_path_from_home(
                            &[OWL_DIR, STASH_DIR, PROMPT_DIR],
                            Some(name),
                        )
                        .expect("prompt exists")
                    }
                };

                if use_tui {
                    tui_utils::enter_raw_mode().and_then(|_| match FileApp::default().run(&path) {
                        Ok(_) => tui_utils::exit_raw_mode(),
                        Err(e) => tui_utils::exit_raw_mode().and(Err(e)),
                    })
                } else if show_manifest || show_program {
                    owl_core::show_it(&path)
                } else {
                    owl_core::show_and_glow(&path)
                }
            } else {
                let name = sub_matches.get_one::<String>("NAME").expect("required");

                if let Some(test_name) = test {
                    owl_core::show_test(name, test_name, show_ans, use_tui).await
                } else {
                    if rand {
                        case = Some(rand::random::<u64>() as usize);
                    }

                    owl_core::show_quest(name, case, show_ans, use_tui).await
                }
            };

            if let Err(e) = action {
                report_owl_err!(e);
            }
        }
        Some(("stash", sub_matches)) => {
            let prog = sub_matches.get_one::<String>("PROG").expect("required");
            let is_templ = sub_matches.get_one::<bool>("templ").is_some_and(|&f| f);
            let is_prompt = sub_matches.get_one::<bool>("prompt").is_some_and(|&f| f);

            if let Err(e) = owl_core::stash_file(Path::new(prog), is_templ, is_prompt) {
                report_owl_err!(e);
            }
        }
        Some(("test", sub_matches)) => {
            let prog = sub_matches.get_one::<String>("PROG").expect("required");
            let in_file = sub_matches.get_one::<String>("IN").expect("required");
            let ans_file = sub_matches.get_one::<String>("ANS").expect("required");

            if let Err(e) =
                owl_core::test_program(Path::new(prog), Path::new(in_file), Path::new(ans_file))
            {
                report_owl_err!(e);
            }
        }
        Some(("update", _)) => {
            let header_url = Url::parse(MANIFEST_HEAD_URL).expect("remote manifest header is URL");
            let manifest_url = Url::parse(MANIFEST_URL).expect("remote manifest is URL");
            let manifest_path = fs_utils::ensure_path_from_home(&[OWL_DIR], Some(MANIFEST))
                .expect("owlgo dir exists");
            let prompt_dir =
                fs_utils::ensure_path_from_home(&[OWL_DIR, STASH_DIR, PROMPT_DIR], None)
                    .expect("prompt dir exists");

            if let Err(e) = toml_utils::update_manifest(
                &header_url,
                &manifest_url,
                &manifest_path,
                &prompt_dir,
                Path::new(TMP_ARCHIVE),
            )
            .await
            {
                report_owl_err!(e);
            }
        }
        Some(("version", sub_matches)) => {
            let lang = sub_matches.get_one::<String>("lang");

            let action = match lang {
                Some(ext) => prog_utils::try_prog_lang(ext)
                    .and_then(|prog_lang| prog_lang.version())
                    .map(|stdout| println!("{}", stdout)),
                None => fs_utils::ensure_path_from_home(&[OWL_DIR], Some(MANIFEST)).and_then(
                    |manifest_path| {
                        if !manifest_path.exists() {
                            toml_utils::create_toml(&manifest_path, TOML_TEMPLATE)?;
                        }

                        let version = toml_utils::get_embedded_version(TOML_TEMPLATE)?;
                        let (manifest_version, timestamp) =
                            toml_utils::get_manifest_version_timestamp(&manifest_path)?;

                        println!("owlgo version {}", version);

                        if toml_utils::compare_stamps(&manifest_version, &version)?
                            == Ordering::Less
                            || timestamp == "0.0.0"
                        {
                            println!("\nmanifest out of date...");
                            println!("run `owlgo update`");
                        }

                        Ok(())
                    },
                ),
            };

            if let Err(e) = action {
                report_owl_err!(e);
            }
        }
        _ => unreachable!(),
    }
}
