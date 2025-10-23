use crate::common::{OwlError, Result};
use crate::owl_utils::{LlmApp, PromptMode, cmd_utils, fs_utils, llm_utils, tui_utils};
use crate::{CHAT_DIR, MANIFEST, OWL_DIR, PROMPT_DIR, PROMPT_FILE, STASH_DIR};
use chrono::{DateTime, Local};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

pub enum ReviewPrompt {
    InQuest(String),
    InStash(String),
    IsFile(PathBuf),
    UserPrompt(String),
}

pub async fn review_program(
    prog: &Path,
    check_prompt: Option<ReviewPrompt>,
    mode: PromptMode,
    forget_chat: bool,
    use_tui: bool,
) -> Result<()> {
    let manifest_path = fs_utils::ensure_path_from_home(&[OWL_DIR], Some(MANIFEST))?;

    if !manifest_path.exists() {
        eprintln!("manifest doesn't exist...");
        eprintln!("run 'owlgo update'");
        return Err(OwlError::FileError(
            "manifest does not exist".into(),
            "".into(),
        ));
    }

    let prog_str = fs::read_to_string(prog).map_err(|e| {
        OwlError::FileError(
            format!("could not read program '{}'", prog.to_string_lossy()),
            e.to_string(),
        )
    })?;

    let check_prompt = match check_prompt {
        Some(review_prompt) => match review_prompt {
            ReviewPrompt::IsFile(path) => {
                let prompt_str = fs::read_to_string(&path).map_err(|e| {
                    OwlError::FileError(
                        format!("could not read prompt '{}'", path.to_string_lossy()),
                        e.to_string(),
                    )
                })?;

                Some(prompt_str)
            }
            ReviewPrompt::InStash(prompt_name) => {
                let prompt_path = fs_utils::ensure_path_from_home(
                    &[OWL_DIR, STASH_DIR, PROMPT_DIR],
                    Some(&prompt_name),
                )?;

                let prompt_str = fs::read_to_string(&prompt_path).map_err(|e| {
                    OwlError::FileError(
                        format!("could not read prompt '{}'", prompt_path.to_string_lossy()),
                        e.to_string(),
                    )
                })?;

                Some(prompt_str)
            }
            ReviewPrompt::InQuest(quest_name) => {
                let prompt_path = fs_utils::ensure_path_from_home(
                    &[OWL_DIR, STASH_DIR, &quest_name],
                    Some(PROMPT_FILE),
                )?;

                let prompt_str = fs::read_to_string(&prompt_path).map_err(|e| {
                    OwlError::FileError(
                        format!("could not read prompt '{}'", prompt_path.to_string_lossy()),
                        e.to_string(),
                    )
                })?;

                Some(prompt_str)
            }
            ReviewPrompt::UserPrompt(prompt_str) => Some(prompt_str),
        },
        None => None,
    };

    let (ai_sdk, client) = llm_utils::try_llm_client(&manifest_path)?;

    let response = if use_tui {
        tui_utils::enter_raw_mode()?;
        let response_text = LlmApp::default()
            .run(
                &ai_sdk,
                &client,
                Some(&prog_str),
                check_prompt.as_deref(),
                mode,
            )
            .await?;
        tui_utils::exit_raw_mode()?;

        response_text
    } else {
        llm_utils::llm_review_with_client(
            &ai_sdk,
            &client,
            Some(&prog_str),
            check_prompt.as_deref(),
            mode,
        )
        .await?
    };

    let now: DateTime<Local> = Local::now();
    let timestamp = now.format("%Y-%m-%d-%H-%M-%S").to_string();

    let chat_file_stem = format!("{}_{}.md", ai_sdk, timestamp);

    let chat_path =
        fs_utils::ensure_path_from_home(&[OWL_DIR, STASH_DIR, CHAT_DIR], Some(&chat_file_stem))?;

    let mut chat_file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&chat_path)
        .map_err(|e| {
            OwlError::FileError(
                format!(
                    "could not truncate chat record '{}'",
                    chat_path.to_string_lossy()
                ),
                e.to_string(),
            )
        })?;

    chat_file
        .write_all(response.as_bytes())
        .map_err(|e| {
            OwlError::FileError(
                format!(
                    "could not write chat record to '{}'",
                    chat_path.to_string_lossy()
                ),
                e.to_string(),
            )
        })
        .map(|_| {
            if cmd_utils::glow_file(&chat_path).is_err() {
                println!("{}", response);
            }
        })?;

    if forget_chat {
        fs_utils::remove_path(&chat_path)?;
    }

    Ok(())
}
