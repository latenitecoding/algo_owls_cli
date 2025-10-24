use crate::common::{OwlError, Result};
use crate::owl_utils::{Uri, fs_utils, toml_utils};
use crate::{MANIFEST, OWL_DIR, PROMPT_DIR, STASH_DIR, TMP_ARCHIVE};
use futures::prelude::*;
use std::path::Path;

pub async fn fetch_extension(ext_name: &str) -> Result<()> {
    let manifest_path = fs_utils::ensure_path_from_home(&[OWL_DIR], Some(MANIFEST))?;

    if !manifest_path.exists() {
        return Err(OwlError::FileError(
            "The manifest does not exist".into(),
            "".into(),
        ));
    }

    let manifest_doc = toml_utils::read_toml(&manifest_path)?;

    let uri = match manifest_doc["ext_uri"].get(ext_name) {
        Some(uri_item) => {
            let uri_str = uri_item.as_str().ok_or(OwlError::TomlError(
                format!("Invalid URI entry '{}' in manifest", ext_name),
                "None".into(),
            ))?;
            Uri::try_from(uri_str)?
        }
        None => {
            return Err(OwlError::TomlError(
                format!("'{}': no such entry found manifest", ext_name),
                "None".into(),
            ));
        }
    };

    let ext_doc = match uri {
        Uri::Local(path) => {
            eprintln!(
                "reading extension '{}' at '{}'",
                ext_name,
                path.to_string_lossy()
            );
            toml_utils::read_toml(&path)?
        }
        Uri::Remote(url) => {
            eprintln!(">>> requesting extension '{}' from '{}' ...", ext_name, url);
            toml_utils::request_toml(&url).await?
        }
    };

    let owl_path = manifest_path.parent().expect("owlgo directory to exist");

    let tmp_archive = Path::new(TMP_ARCHIVE);

    let quest_futures = ext_doc["quests"]
        .as_table()
        .into_iter()
        .flat_map(|quests_table| quests_table.iter())
        .map(|(quest_name, quest_uri)| async move {
            let mut quest_path = owl_path.to_path_buf();
            quest_path.push(quest_name);

            let quest_uri_str = quest_uri.as_str().ok_or(OwlError::TomlError(
                format!("Invalid entry '{}' in extension '{}'", quest_name, ext_name),
                "None".into(),
            ))?;

            match Uri::try_from(quest_uri_str)? {
                Uri::Local(path) => {
                    eprintln!(
                        ">>> extracting quest '{}' at '{}' ...",
                        quest_name,
                        path.to_string_lossy()
                    );
                    fs_utils::extract_archive(&path, tmp_archive, true).await
                }
                Uri::Remote(url) => {
                    eprintln!(">>> downloading quest '{}' from '{}' ...", quest_name, url);
                    fs_utils::download_archive(&url, tmp_archive, &quest_path).await
                }
            }
        });

    let prompt_futures = ext_doc["prompts"]
        .as_table()
        .into_iter()
        .flat_map(|prompts_table| prompts_table.iter())
        .map(|(prompt_name, prompt_uri)| async move {
            let mut prompt_path = owl_path.to_path_buf();
            prompt_path.push(STASH_DIR);
            prompt_path.push(PROMPT_DIR);
            prompt_path.push(prompt_name);

            let prompt_uri_str = prompt_uri.as_str().ok_or(OwlError::TomlError(
                format!(
                    "Invalid entry '{}' in extension '{}'",
                    prompt_name, ext_name
                ),
                "None".into(),
            ))?;

            match Uri::try_from(prompt_uri_str)? {
                Uri::Local(path) => {
                    eprintln!(
                        ">>> copying prompt '{}' from '{}' ...",
                        prompt_name,
                        path.to_string_lossy()
                    );
                    fs_utils::copy_file_async(&path, &prompt_path).await
                }
                Uri::Remote(url) => {
                    eprintln!(
                        ">>> downloading prompt '{}' from '{}' ...",
                        prompt_name, url
                    );
                    fs_utils::download_file(&url, &prompt_path).await
                }
            }
        });

    let quest_stream = futures::stream::iter(quest_futures).buffer_unordered(8);
    let prompt_stream = futures::stream::iter(prompt_futures).buffer_unordered(8);

    for result in quest_stream.collect::<Vec<_>>().await {
        result?;
    }

    for result in prompt_stream.collect::<Vec<_>>().await {
        result?;
    }

    Ok(())
}

pub async fn fetch_prompt(prompt_name: &str) -> Result<()> {
    let manifest_path = fs_utils::ensure_path_from_home(&[OWL_DIR], Some(MANIFEST))?;
    let prompt_path =
        fs_utils::ensure_path_from_home(&[OWL_DIR, STASH_DIR, PROMPT_DIR], Some(prompt_name))?;

    if !manifest_path.exists() {
        return Err(OwlError::FileError(
            "The manifest does not exist".into(),
            "".into(),
        ));
    }

    let manifest_doc = toml_utils::read_toml(&manifest_path)?;

    let prompt_entry = manifest_doc["personal_prompts"]
        .get(prompt_name)
        .or(manifest_doc["prompts"].get(prompt_name));

    let uri = match prompt_entry {
        Some(uri_item) => {
            let uri_str = uri_item.as_str().ok_or(OwlError::TomlError(
                format!("Invalid entry '{}' in manifest", prompt_name),
                "None".into(),
            ))?;
            Uri::try_from(uri_str)?
        }
        None => {
            return Err(OwlError::TomlError(
                format!("'{}': no such entry found manifest", prompt_name),
                "None".into(),
            ));
        }
    };

    match uri {
        Uri::Local(path) => fs_utils::copy_file(&path, &prompt_path),
        Uri::Remote(url) => fs_utils::download_file(&url, &prompt_path).await,
    }
}

pub async fn fetch_quest(quest_name: &str) -> Result<()> {
    let manifest_path = fs_utils::ensure_path_from_home(&[OWL_DIR], Some(MANIFEST))?;
    let quest_dir = fs_utils::ensure_path_from_home(&[OWL_DIR], Some(quest_name))?;

    if !manifest_path.exists() {
        return Err(OwlError::FileError(
            "The manifest does not exist".into(),
            "".into(),
        ));
    }

    let manifest_doc = toml_utils::read_toml(&manifest_path)?;
    let quest_entry = manifest_doc["personal_quests"]
        .get(quest_name)
        .or(manifest_doc["quests"].get(quest_name));

    let uri = match quest_entry {
        Some(uri_item) => {
            let uri_str = uri_item.as_str().ok_or(OwlError::TomlError(
                format!("Invalid entry '{}' in manifest", quest_name),
                "None".into(),
            ))?;
            Uri::try_from(uri_str)?
        }
        None => {
            return Err(OwlError::TomlError(
                format!("'{}': no such entry found manifest", quest_name),
                "None".into(),
            ));
        }
    };

    match uri {
        Uri::Local(path) => fs_utils::extract_archive(&path, &quest_dir, false).await,
        Uri::Remote(url) => {
            fs_utils::download_archive(&url, Path::new(TMP_ARCHIVE), &quest_dir).await
        }
    }
}
