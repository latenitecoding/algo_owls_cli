use anthropic_sdk::{Anthropic, ContentBlock, MessageCreateBuilder};
use chrono::{DateTime, Utc};
use clap::{Command, arg};
use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::process;

mod owl_utils;
use owl_utils::{cmd_utils, fs_utils, owl_error::OwlError, prog_lang};

const CHAT_DIR: &str = ".chat";
const GIT_DIR: &str = ".git";
const MANIFEST: &str = ".manifest.toml";
const MANIFEST_HEAD_URL: &str = "https://gist.githubusercontent.com/latenitecoding/84c043f4c9092998773640a2202f2d36/raw/owl_manifest_short";
const MANIFEST_URL: &str = "https://gist.githubusercontent.com/latenitecoding/b6fdd8656c0b6a60795581f84d0f2fa4/raw/owlgo_manifest";
const OWL_DIR: &str = ".owlgo";
const PROMPT_DIR: &str = ".prompt";
const TEMPLATE_STEM: &str = ".template";
const TMP_ARCHIVE: &str = ".tmp.zip";
const STASH_DIR: &str = ".stash";

// it must be that [manifest] is at the top and [personal] is at the bottom
const TOML_TEMPLATE: &str = r#"
[manifest]
version = "0.1.3"
timestamp = "0.0.0"
ai_sdk = "claude"
api_key = ""

[extensions]

[ext_uri]

[quests]

[personal]
"#;

macro_rules! report_owl_err {
    ($expr:expr) => {
        eprintln!("\x1b[31m[owlgo error]\x1b[0m: {}", $expr);
        process::exit(1);
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
    Command::new("owlgo")
        .about("A lightweight CLI to assist in solving CP problems")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("add")
                .about("adds new personal quest(s) to manifest")
                .arg(arg!(<NAME> "The name of the quest/manifest"))
                .arg(arg!(<URI> "The URL/PATH to fetch from"))
                .arg(arg!(-f --fetch "Fetches test cases"))
                .arg(arg!(-m --manifest "The URL is a manifest to be committed"))
                .arg(arg!(-l --local "The URI is a local path"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("clear")
                .about("removes all stashed test cases (and solutions)")
                .arg(arg!(-s --stash "Removes all stashed programs/prompts (and the git dir)"))
                .arg(arg!(-p --program "Removes all stashed programs"))
                .arg(arg!(-P --prompt "Removes all stashed prompts"))
                .arg(arg!(-c --chat "Removes AI chat history"))
                .arg(arg!(-m --manifest "Removes the manifest"))
                .arg(arg!(-k --keep-test "Tests are not cleared"))
                .arg(arg!(--all "Removes everything not excluded by other flags")),
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
        .subcommand(Command::new("list").about("outputs information on stashed files"))
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
                .arg(arg!(-C --case <CASE> "The specific test to run by case number"))
                .arg(arg!(-r --rand "Test against a random test case"))
                .arg(arg!(-n --hint "Prints the hint/feedback (if any)"))
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
            Command::new("review")
                .about("submits the program to an LLM for a code review")
                .arg(arg!(<PROG> "The program to review"))
                .arg(arg!(<PROMPT> "The prompt to give"))
                .arg(arg!(-f --file "The prompt is in a file"))
                .arg(arg!(-R --forget "Forget chat history after each prompt"))
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
                .arg(arg!(<NAME> "The name of the quest/solution"))
                .arg(arg!(-t --test <TEST> "The specific test to print by name"))
                .arg(arg!(-c --case <CASE> "The specific test to print by case number"))
                .arg(arg!(-r --rand "Print a random test case"))
                .arg(arg!(-a --ans "Print the answer instead of the input"))
                .arg(arg!(-p --prog "Show a stashed program instead of a test case"))
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
        .subcommand(
            Command::new("update")
                .about("checks owlgo and its manifest for updates")
                .arg(arg!(-e --ext "Update extensions")),
        )
        .subcommand(
            Command::new("version")
                .about("outputs the current version")
                .arg(arg!(-L --lang <EXT> "Outputs the system version of the language")),
        )
}

fn add(
    name: &str,
    uri: &str,
    and_fetch: bool,
    is_manifest: bool,
    is_local: bool,
) -> Result<(), OwlError> {
    // this should always rewrite entries in the personal table
    // of the manifest TOML, which is the last table in the manifest
    // new entires can always be appended
    let mut manifest_path = fs_utils::ensure_dir_from_home(&[OWL_DIR])?;
    manifest_path.push(MANIFEST);

    if !is_manifest && !manifest_path.exists() {
        fs_utils::create_toml_with_entry(
            check_path!(manifest_path)?,
            TOML_TEMPLATE,
            "personal",
            name,
            uri,
        )?;
    } else if !is_manifest && manifest_path.exists() {
        fs_utils::update_toml_entry(check_path!(manifest_path)?, "personal", name, uri)?;
    }

    if !is_manifest && and_fetch {
        fetch_by_name(name)?;
    }

    if is_manifest && !manifest_path.exists() {
        fs_utils::create_toml(check_path!(manifest_path)?, TOML_TEMPLATE)?;
    }

    if is_manifest {
        let some_tmp_archive = if and_fetch { Some(TMP_ARCHIVE) } else { None };

        fs_utils::commit_manifest(
            check_path!(manifest_path)?,
            name,
            uri,
            some_tmp_archive,
            is_local,
        )?;
    }

    Ok(())
}

fn build_program(prog: &str) -> Result<Option<prog_lang::BuildLog>, OwlError> {
    match prog_lang::check_prog_lang(prog) {
        Some(lang) => {
            if !lang.command_exists() {
                return Err(command_not_found!("build_program::check_lang", lang.name()));
            }

            if lang.should_build() {
                let build_log = lang.build(prog)?;
                println!("{}", build_log.stdout);

                Ok(Some(build_log))
            } else {
                Ok(None)
            }
        }
        None => Ok(None),
    }
}

fn cleanup_program(
    prog: &str,
    target: &str,
    build_files: Option<Vec<String>>,
) -> Result<(), OwlError> {
    if target != prog {
        fs_utils::remove_path(target)?;
    }

    if let Some(build_files) = &build_files {
        for build_file in build_files {
            fs_utils::remove_path(build_file)?;
        }
    }

    Ok(())
}

fn clear_stash(
    and_stash: bool,
    and_program: bool,
    and_prompt: bool,
    and_chat: bool,
    and_manif: bool,
    keep_test: bool,
    all_files: bool,
) -> Result<(), OwlError> {
    let owl_path = fs_utils::ensure_dir_from_home(&[OWL_DIR])?;

    for entry in fs::read_dir(check_path!(owl_path)?)
        .map_err(|e| file_error!("clear_stash::read_owl_dir", e))?
    {
        let path = entry
            .map_err(|e| file_error!("clear_stash::check_sub_dir", e))?
            .path();
        let stem = check_file_stem!(path)?;

        if path.is_file() && (all_files || (stem == MANIFEST && and_manif)) {
            fs_utils::remove_path(check_path!(path)?)?;
        }

        if path.is_dir()
            && (all_files || (stem == STASH_DIR && and_stash) || (stem == CHAT_DIR && and_chat))
        {
            fs_utils::remove_path(check_path!(path)?)?;
        }

        if path.is_dir() && !all_files && stem != STASH_DIR && stem != CHAT_DIR && !keep_test {
            fs_utils::remove_path(check_path!(path)?)?;
        }

        if path.is_dir()
            && !all_files
            && stem == STASH_DIR
            && !and_stash
            && (and_program || and_prompt)
        {
            for s_entry in fs::read_dir(check_path!(path)?)
                .map_err(|e| file_error!("clear_stash::read_stash_dir", e))?
            {
                let s_path = s_entry
                    .map_err(|e| file_error!("clear_stash::check_stash_dir", e))?
                    .path();
                let s_stem = check_file_stem!(s_path)?;

                if s_path.is_file() && and_program {
                    fs_utils::remove_path(check_path!(s_path)?)?;
                }

                if s_path.is_dir() && s_stem == PROMPT_DIR && and_prompt {
                    fs_utils::remove_path(check_path!(s_path)?)?;
                }

                if s_path.is_dir() && s_stem != GIT_DIR && s_stem != PROMPT_DIR && and_program {
                    fs_utils::remove_path(check_path!(s_path)?)?;
                }
            }
        }
    }

    Ok(())
}

fn fetch(name: &str, dir: &str) -> Result<(), OwlError> {
    let mut manifest_path = fs_utils::ensure_dir_from_home(&[OWL_DIR])?;
    manifest_path.push(MANIFEST);

    if !manifest_path.exists() {
        let path_str = check_path!(manifest_path)?;
        return Err(file_not_found!("fetch::manifest_path", path_str));
    }

    let url = fs_utils::get_toml_entry(check_path!(manifest_path)?, &["personal", "quests"], name)?;

    fs_utils::download_archive(&url, TMP_ARCHIVE, dir)
}

fn fetch_by_name(name: &str) -> Result<(), OwlError> {
    let mut fetch_path = fs_utils::ensure_dir_from_home(&[OWL_DIR])?;
    fetch_path.push(name);

    fetch(name, check_path!(fetch_path)?)
}

fn init_program(prog: &str) -> Result<(), OwlError> {
    if Path::new(prog).exists() {
        return Err(file_error!(
            "init_program::prog_exists",
            format!("file already exists: '{}'", prog)
        ));
    }

    let mut stash_path = fs_utils::ensure_dir_from_home(&[OWL_DIR, STASH_DIR])?;

    let ext = Path::new(prog)
        .extension()
        .and_then(OsStr::to_str)
        .ok_or(file_error!("init_program::prog_ext", prog))?;

    let stash_file = format!("{}.{}", TEMPLATE_STEM, ext);

    stash_path.push(stash_file);

    fs_utils::copy_file(check_path!(stash_path)?, prog)
}

fn list_stash() -> Result<(), OwlError> {
    let stash_path = fs_utils::ensure_dir_from_home(&[OWL_DIR, STASH_DIR])?;

    cmd_utils::list_all(check_path!(stash_path)?).or_else(|_| {
        fs_utils::list_dir(check_path!(stash_path)?.to_string())
            .map(|files| println!("{}", files.join("\n")))
    })
}

fn push_git_remote(force: bool) -> Result<(), OwlError> {
    let mut stash_path = fs_utils::ensure_dir_from_home(&[OWL_DIR, STASH_DIR])?;
    stash_path.push(".git");

    if !stash_path.exists() {
        return Err(file_not_found!(
            "push_git_remote::stash_git_dir",
            check_path!(stash_path)?
        ));
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
    use_hints: bool,
) -> Result<(), OwlError> {
    let mut quest_path = fs_utils::ensure_dir_from_home(&[OWL_DIR])?;
    quest_path.push(name);

    let quest_dir = check_path!(quest_path)?.to_string();

    if !quest_path.exists() {
        fetch(name, &quest_dir)?;
    }

    let prog_path = Path::new(prog);

    if !prog_path.exists() {
        return Err(file_not_found!("quest::prog_path", prog));
    }

    let (target, build_files) = match build_program(prog)? {
        Some(bl) => (bl.target, bl.build_files),
        None => (prog.to_string(), None),
    };

    let test_cases: Vec<String> = fs_utils::find_by_ext(quest_dir, "in")?;
    let total = test_cases.len();

    let mut passed = 0;
    let mut failed = 0;
    let mut count = 0;
    let mut total_duration = 0;

    for test_case in test_cases {
        count += 1;

        let in_path = Path::new(&test_case);
        let in_stem = in_path
            .file_stem()
            .and_then(OsStr::to_str)
            .ok_or(file_error!("quest::in_file_stem", test_case))?;

        if let Some(name) = test_name
            && in_stem != name
        {
            continue;
        }

        if case_id > 0 && (count % total) != (case_id % total) {
            continue;
        }

        match quest_it(&target, &test_case, count, total, use_hints) {
            Ok((true, elapsed)) => {
                passed += 1;
                total_duration += elapsed;
            }
            Ok((false, _)) | Err(_) => failed += 1,
        }
    }

    println!(
        "passed: {}, failed: {}, elapsed: {}ms",
        passed, failed, total_duration
    );

    cleanup_program(prog, &target, build_files)?;

    if failed > 0 {
        Err(test_failure!("test failures"))
    } else {
        println!("\x1b[32mall tests passed\x1b[0m 🏆🏆🏆\n");
        Ok(())
    }
}

fn quest_it(
    target: &str,
    test_case: &str,
    count: usize,
    total: usize,
    use_hints: bool,
) -> Result<(bool, u128), OwlError> {
    let in_path = Path::new(&test_case);
    let in_stem = in_path
        .file_stem()
        .and_then(OsStr::to_str)
        .ok_or(file_error!("quest_it::in_file_stem", test_case))?;

    let ans_file = fs_utils::as_ans_file(test_case)?;

    match test_it(target, test_case, &ans_file) {
        Ok(elapsed) => {
            println!(
                "({}/{}) [{}ms] {} \x1b[32mpassed test\x1b[0m 🎉\n",
                count, total, elapsed, in_stem
            );
            Ok((true, elapsed))
        }
        Err(e) => {
            if use_hints && let Ok(mut parent_dir) = check_parent!(in_path) {
                let feedback_file = format!("{}.md", in_stem);
                parent_dir.push(feedback_file);

                let _ = check_path!(parent_dir).and_then(|parent_str| {
                    cmd_utils::glow_file(parent_str).or_else(|_| {
                        fs_utils::cat_file(parent_str).map(|contents| eprintln!("{}", contents))
                    })
                });
            }

            eprintln!(
                "({}/{}) {} \x1b[31m{}\x1b[0m 😭\n",
                count, total, in_stem, e
            );

            Ok((false, 0))
        }
    }
}

fn restore_program(prog: &str) -> Result<(), OwlError> {
    let mut stash_path = fs_utils::ensure_dir_from_home(&[OWL_DIR, STASH_DIR])?;
    stash_path.push(prog);

    fs_utils::copy_file(check_path!(stash_path)?, prog)
}

async fn review_program(
    prog: &str,
    prompt: &str,
    is_file: bool,
    forget_chat: bool,
) -> Result<(), OwlError> {
    let mut manifest_path = fs_utils::ensure_dir_from_home(&[OWL_DIR])?;
    manifest_path.push(MANIFEST);

    if !manifest_path.exists() {
        eprintln!("manifest doesn't exist...");
        eprintln!("run 'owlgo update'");
        return Err(file_not_found!(
            "review_program::open_manifest",
            check_path!(manifest_path)?
        ));
    }

    let (ai_sdk, api_key) = fs_utils::get_toml_ai_sdk(check_path!(manifest_path)?)?;

    if ai_sdk.is_empty() {
        eprintln!("no LLM has been selected!");
        return Err(no_entry_found!("ai_sdk"));
    }

    if api_key.is_empty() {
        eprintln!("no API key has been provided!");
        return Err(no_entry_found!("api_key"));
    }

    match ai_sdk.as_str() {
        "claude" => println!("Sending code review to Claude..."),
        _ => return Err(not_supported!(ai_sdk)),
    };

    let client = Anthropic::new(api_key).map_err(|e| llm_error!(ai_sdk, e))?;

    let prog_str = fs_utils::cat_file(prog)?;

    let prompt_str = if is_file {
        fs_utils::cat_file(prompt)?
    } else {
        prompt.to_string()
    };

    let response = client
        .messages()
        .create(
            MessageCreateBuilder::new("claude-sonnet-4-5", 1024)
                .user(format!("Hello, Claude! I'm writing a program to solve a problem. That problem has this description:\n{}. Here is the program that I wrote:\n{}\nCould you please review this code and tell me what I can improve?", prompt_str, prog_str))
                .build(),
        )
        .await
        .map_err(|e| llm_error!(ai_sdk, e))?;

    let mut buffer = String::new();
    for content_block in response.content {
        if let ContentBlock::Text { text } = content_block {
            buffer.push_str("\nClaude: ");
            buffer.push_str(&text);
        }
    }

    let mut chat_path = fs_utils::ensure_dir_from_home(&[OWL_DIR, CHAT_DIR])?;

    let now: DateTime<Utc> = Utc::now();
    let timestamp = now.format("%Y-%m-%d-%H-%M-%S").to_string();

    let chat_file_stem = format!("{}_{}.md", ai_sdk, timestamp);
    chat_path.push(&chat_file_stem);

    let chat_file_str = check_path!(chat_path)?;

    let _ = fs_utils::record_chat(chat_file_str, &buffer)
        .and_then(|_| {
            cmd_utils::glow_file(chat_file_str).or_else(|_| {
                fs_utils::cat_file(chat_file_str).map(|contents| println!("{}", contents))
            })
        })
        .map_err(|_| println!("{}", buffer));

    if forget_chat {
        fs_utils::remove_path(chat_file_str)?;
    }

    Ok(())
}

fn run(prog: &str) -> Result<(), OwlError> {
    if !Path::new(prog).exists() {
        return Err(file_not_found!("run::prog_path", prog));
    }

    match prog_lang::check_prog_lang(prog) {
        Some(lang) => {
            let (target, build_files) = match build_program(prog)? {
                Some(bl) => (bl.target, bl.build_files),
                None => (prog.to_string(), None),
            };

            let run_result = lang.run(&target);

            cleanup_program(prog, &target, build_files)?;

            run_result.map(|(stdout, _)| println!("{}", stdout))
        }
        None => {
            let (stdout, _) = cmd_utils::run_binary(prog)?;
            println!("{}", stdout);
            Ok(())
        }
    }
}

fn set_git_remote(remote: &str, force: bool) -> Result<(), OwlError> {
    let mut stash_path = fs_utils::ensure_dir_from_home(&[OWL_DIR, STASH_DIR])?;
    stash_path.push(".git");

    if stash_path.exists() && !force {
        return Err(file_error!(
            "set_git_remote::stash_git_dir",
            ".git directory already exists"
        ));
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

fn show_it(target_file: &str, show_ans: bool) -> Result<(), OwlError> {
    let contents = if show_ans {
        let ans_file = fs_utils::as_ans_file(target_file)?;

        fs::read_to_string(ans_file).map_err(|e| file_error!("show_it:read_ans_file", e))?
    } else {
        fs::read_to_string(target_file).map_err(|e| file_error!("show_it::read_target_file", e))?
    };

    println!("{}", contents);

    Ok(())
}

fn show_program(prog: &str) -> Result<(), OwlError> {
    let mut stash_path = fs_utils::ensure_dir_from_home(&[OWL_DIR, STASH_DIR])?;
    stash_path.push(prog);

    if !stash_path.exists() {
        return Err(file_not_found!("show_program::prog_path", prog));
    }

    cmd_utils::bat_file(check_path!(stash_path)?).or_else(|_| {
        fs_utils::cat_file(check_path!(stash_path)?).map(|contents| println!("{}", contents))
    })
}

fn show_test_case(
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
        return show_it(&test_cases[(case_id - 1) % test_cases.len()], show_ans);
    }

    for test_case in test_cases {
        show_it(&test_case, show_ans)?;
    }

    Ok(())
}

fn show_version(lang_ext: Option<&String>) -> Result<(), OwlError> {
    match lang_ext {
        Some(ext) => {
            let lang = prog_lang::get_prog_lang(ext)?;

            match lang.version() {
                Ok(stdout) => println!("{}", stdout),
                Err(_) => return Err(command_not_found!("show_version::check_lang", ext)),
            }
        }
        None => {
            let mut manifest_path = fs_utils::ensure_dir_from_home(&[OWL_DIR])?;
            manifest_path.push(MANIFEST);

            if !manifest_path.exists() {
                fs_utils::create_toml(check_path!(manifest_path)?, TOML_TEMPLATE)?;
            }

            let version = fs_utils::extract_toml_version(TOML_TEMPLATE)?;
            let (manifest_version, timestamp) =
                fs_utils::get_toml_version_timestamp(check_path!(manifest_path)?)?;

            println!("owlgo version {}", version);

            if fs_utils::compare_stamps(&manifest_version, &version)? || timestamp == "0.0.0" {
                println!("\nmanifest out of date...");
                println!("run `owlgo update`");
            }
        }
    }

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
        .ok_or(file_error!("stash::prog_ext", prog))?;

    let stash_file = format!("{}.{}", TEMPLATE_STEM, ext);

    stash_path.push(stash_file);

    fs_utils::copy_file(prog, check_path!(stash_path)?)
}

fn sync_git_remote(force: bool) -> Result<(), OwlError> {
    let mut stash_path = fs_utils::ensure_dir_from_home(&[OWL_DIR, STASH_DIR])?;
    stash_path.push(".git");

    if !stash_path.exists() {
        return Err(file_not_found!(
            "sync_git_remote::stash_git_dir",
            check_path!(stash_path)?
        ));
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

fn test(prog: &str, in_file: &str, ans_file: &str) -> Result<(), OwlError> {
    let test_result = match prog_lang::check_prog_lang(prog) {
        Some(_) => {
            let (target, build_files) = match build_program(prog)? {
                Some(bl) => (bl.target, bl.build_files),
                None => (prog.to_string(), None),
            };

            let test_result = test_it(&target, in_file, ans_file);

            cleanup_program(prog, &target, build_files)?;

            test_result
        }
        None => test_it(prog, in_file, ans_file),
    };

    match test_result {
        Ok(elapsed) => {
            println!("[{}ms] \x1b[32mpassed test\x1b[0m 🎉\n", elapsed);
            Ok(())
        }
        Err(e) => {
            eprintln!("\x1b[31m{}\x1b[0m 😭\n", e);
            Ok(())
        }
    }
}

fn test_it(target: &str, in_file: &str, ans_file: &str) -> Result<u128, OwlError> {
    let prog_path = Path::new(target);
    let in_path = Path::new(in_file);
    let ans_path = Path::new(ans_file);

    if !prog_path.exists() {
        return Err(file_not_found!("test_it::target_prog", target));
    }
    if !in_path.exists() {
        return Err(file_not_found!("test_it::in_file", in_file));
    }
    if !ans_path.exists() {
        return Err(file_not_found!("test_it::ans_file", ans_file));
    }

    let stdin = fs::read_to_string(in_path).map_err(|e| file_error!("test_it::read_in_file", e))?;
    let ans = fs::read_to_string(ans_path).map_err(|e| file_error!("test_it::read_ans_file", e))?;

    match prog_lang::check_prog_lang(target) {
        Some(lang) => {
            if !lang.command_exists() {
                return Err(command_not_found!("test_it::check_lang", lang.name()));
            }

            let run_result = lang.run_with_stdin(target, &stdin);

            run_result.and_then(|(actual, elapsed)| {
                if actual == ans {
                    Ok(elapsed)
                } else {
                    report_test_failed!(in_file, ans, actual);
                    Err(test_failure!("failed test"))
                }
            })
        }
        None => cmd_utils::run_binary_with_stdin(target, &stdin).and_then(|(actual, elapsed)| {
            if actual == ans {
                Ok(elapsed)
            } else {
                report_test_failed!(in_file, ans, actual);
                Err(test_failure!("failed test"))
            }
        }),
    }
}

fn update(and_extensions: bool) -> Result<(), OwlError> {
    let mut manifest_path = fs_utils::ensure_dir_from_home(&[OWL_DIR])?;
    manifest_path.push(MANIFEST);

    if !manifest_path.exists() {
        fs_utils::create_toml(check_path!(manifest_path)?, TOML_TEMPLATE)?;
    }

    let version = fs_utils::extract_toml_version(TOML_TEMPLATE)?;
    let (_, timestamp) = fs_utils::get_toml_version_timestamp(check_path!(manifest_path)?)?;

    let (version_out_of_date, timestamp_out_of_date) =
        fs_utils::check_for_updates(MANIFEST_HEAD_URL, &version, &timestamp)?;

    if timestamp_out_of_date {
        println!("manifest out of date...");
        println!("updating manifest...");

        fs_utils::update_toml(check_path!(manifest_path)?, MANIFEST_URL)?
    }

    if and_extensions {
        println!("updating extensions...");
        fs_utils::update_extensions(check_path!(manifest_path)?, TMP_ARCHIVE)?
    }

    if version_out_of_date {
        println!("owlgo out of date...");
        println!("run `cargo install --force owlgo`")
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    let matches = cli().get_matches();

    match matches.subcommand() {
        Some(("add", sub_matches)) => {
            let name = sub_matches.get_one::<String>("NAME").expect("required");
            let uri = sub_matches.get_one::<String>("URI").expect("required");
            let fetch = sub_matches.get_one::<bool>("fetch").is_some_and(|&f| f);
            let manif = sub_matches.get_one::<bool>("manifest").is_some_and(|&f| f);
            let local_path = sub_matches.get_one::<bool>("local").is_some_and(|&f| f);

            if let Err(e) = add(name, uri, fetch, manif, local_path) {
                report_owl_err!(&e);
            }
        }
        Some(("clear", sub_matches)) => {
            let stash = sub_matches.get_one::<bool>("stash").is_some_and(|&f| f);
            let program = sub_matches.get_one::<bool>("program").is_some_and(|&f| f);
            let prompt = sub_matches.get_one::<bool>("prompt").is_some_and(|&f| f);
            let chat = sub_matches.get_one::<bool>("chat").is_some_and(|&f| f);
            let manif = sub_matches.get_one::<bool>("manifest").is_some_and(|&f| f);
            let keep_test = sub_matches.get_one::<bool>("keep-test").is_some_and(|&f| f);
            let all = sub_matches.get_one::<bool>("all").is_some_and(|&f| f);

            if let Err(e) = clear_stash(stash, program, prompt, chat, manif, keep_test, all) {
                report_owl_err!(&e);
            }
        }
        Some(("fetch", sub_matches)) => {
            let name = sub_matches.get_one::<String>("NAME").expect("required");

            if let Err(e) = fetch_by_name(name) {
                report_owl_err!(&e);
            }
        }
        Some(("init", sub_matches)) => {
            let prog = sub_matches.get_one::<String>("PROG").expect("required");

            if let Err(e) = init_program(prog) {
                report_owl_err!(&e);
            }
        }
        Some(("list", _)) => {
            if let Err(e) = list_stash() {
                report_owl_err!(&e);
            }
        }
        Some(("push", sub_matches)) => {
            let force = sub_matches.get_one::<bool>("force").is_some_and(|&f| f);

            if let Err(e) = push_git_remote(force) {
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
            let rand = sub_matches.get_one::<bool>("rand").is_some_and(|&f| f);
            let use_hints = sub_matches.get_one::<bool>("hint").is_some_and(|&f| f);

            if rand {
                case = rand::random::<u64>() as usize;
            }

            if let Err(e) = quest(name, prog, test, case, use_hints) {
                report_owl_err!(&e);
            }
        }
        Some(("remote", sub_matches)) => {
            let remote = sub_matches.get_one::<String>("REMOTE").expect("required");
            let force = sub_matches.get_one::<bool>("force").is_some_and(|&f| f);

            if let Err(e) = set_git_remote(remote, force) {
                report_owl_err!(&e);
            }
        }
        Some(("restore", sub_matches)) => {
            let prog = sub_matches.get_one::<String>("PROG").expect("required");

            if let Err(e) = restore_program(prog) {
                report_owl_err!(&e);
            }
        }
        Some(("review", sub_matches)) => {
            let prog = sub_matches.get_one::<String>("PROG").expect("required");
            let prompt = sub_matches.get_one::<String>("PROMPT").expect("required");
            let is_file = sub_matches.get_one::<bool>("file").is_some_and(|&f| f);
            let forget = sub_matches.get_one::<bool>("forget").is_some_and(|&f| f);

            if let Err(e) = review_program(prog, prompt, is_file, forget).await {
                report_owl_err!(&e);
            }
        }
        Some(("run", sub_matches)) => {
            let prog = sub_matches.get_one::<String>("PROG").expect("required");

            if let Err(e) = run(prog) {
                report_owl_err!(&e);
            }
        }
        Some(("show", sub_matches)) => {
            let name = sub_matches.get_one::<String>("NAME").expect("required");
            let test = sub_matches.get_one::<String>("test");
            let mut case = sub_matches
                .get_one::<String>("case")
                .map_or(0, |s| s.parse().expect("case id should be a number"));
            let rand = sub_matches.get_one::<bool>("rand").is_some_and(|&f| f);
            let ans = sub_matches.get_one::<bool>("ans").is_some_and(|&f| f);
            let prog = sub_matches.get_one::<bool>("prog").is_some_and(|&f| f);

            if prog {
                if let Err(e) = show_program(name) {
                    report_owl_err!(&e);
                }
            } else {
                if rand {
                    case = rand::random::<u64>() as usize;
                }

                if let Err(e) = show_test_case(name, test, case, ans) {
                    report_owl_err!(&e);
                }
            }
        }
        Some(("stash", sub_matches)) => {
            let prog = sub_matches.get_one::<String>("PROG").expect("required");
            let templ = sub_matches.get_one::<bool>("templ").is_some_and(|&f| f);

            if let Err(e) = stash(prog, templ) {
                report_owl_err!(&e);
            }
        }
        Some(("sync", sub_matches)) => {
            let force = sub_matches.get_one::<bool>("force").is_some_and(|&f| f);

            if let Err(e) = sync_git_remote(force) {
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
        Some(("update", sub_matches)) => {
            let extensions = sub_matches.get_one::<bool>("ext").is_some_and(|&f| f);

            if let Err(e) = update(extensions) {
                report_owl_err!(&e);
            }
        }
        Some(("version", sub_matches)) => {
            let lang = sub_matches.get_one::<String>("lang");

            if let Err(e) = show_version(lang) {
                report_owl_err!(&e);
            }
        }
        _ => unreachable!(),
    }
}
