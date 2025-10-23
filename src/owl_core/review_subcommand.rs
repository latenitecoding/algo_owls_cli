use crate::common::{OwlError, Result};
use crate::owl_utils::{LlmApp, PromptMode, cmd_utils, fs_utils, llm_utils};
use crate::{CHAT_DIR, MANIFEST, OWL_DIR, PROMPT_DIR, PROMPT_FILE, STASH_DIR};
use chrono::{DateTime, Local};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

pub async fn review_program(
    prog: &Path,
    check_prompt: Option<String>,
    in_stash: bool,
    in_quest: bool,
    is_file: bool,
    forget_chat: bool,
    mode: PromptMode,
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
        Some(prompt_entry) => {
            if is_file {
                let prompt_str = fs::read_to_string(Path::new(&prompt_entry)).map_err(|e| {
                    OwlError::FileError(
                        format!("could not read prompt '{}'", prompt_entry),
                        e.to_string(),
                    )
                })?;

                Some(prompt_str)
            } else if in_stash {
                let prompt_path = fs_utils::ensure_path_from_home(
                    &[OWL_DIR, STASH_DIR, PROMPT_DIR],
                    Some(&prompt_entry),
                )?;

                let prompt_str = fs::read_to_string(&prompt_path).map_err(|e| {
                    OwlError::FileError(
                        format!("could not read prompt '{}'", prompt_path.to_string_lossy()),
                        e.to_string(),
                    )
                })?;

                Some(prompt_str)
            } else if in_quest {
                let prompt_path = fs_utils::ensure_path_from_home(
                    &[OWL_DIR, STASH_DIR, &prompt_entry],
                    Some(PROMPT_FILE),
                )?;

                let prompt_str = fs::read_to_string(&prompt_path).map_err(|e| {
                    OwlError::FileError(
                        format!("could not read prompt '{}'", prompt_path.to_string_lossy()),
                        e.to_string(),
                    )
                })?;

                Some(prompt_str)
            } else {
                Some(prompt_entry)
            }
        }
        None => None,
    };

    let (ai_sdk, client) = llm_utils::try_llm_client(&manifest_path)?;

    let response = match mode {
        PromptMode::Chat => {
            LlmApp::default()
                .run(
                    &ai_sdk,
                    &client,
                    Some(&prog_str),
                    check_prompt.as_deref(),
                    mode,
                )
                .await?
        }
        _ => {
            llm_utils::llm_review_with_client(
                &ai_sdk,
                &client,
                Some(&prog_str),
                check_prompt.as_deref(),
                mode,
            )
            .await?
        }
    };

    let mut chat_path = fs_utils::ensure_path_from_home(&[OWL_DIR, CHAT_DIR], None)?;

    let now: DateTime<Local> = Local::now();
    let timestamp = now.format("%Y-%m-%d-%H-%M-%S").to_string();

    let chat_file_stem = format!("{}_{}.md", ai_sdk, timestamp);
    chat_path.push(&chat_file_stem);

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
