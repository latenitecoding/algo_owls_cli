use crate::common::{OwlError, Result};
use crate::owl_utils::{Uri, fs_utils, toml_utils};
use crate::{MANIFEST, OWL_DIR, PROMPT_DIR, STASH_DIR, TMP_ARCHIVE};
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

    let ext_uri_key = format!("{}.uri", ext_name);

    let uri = match manifest_doc["ext_uri"].get(&ext_uri_key) {
        Some(uri_item) => {
            let uri_str = uri_item.as_str().ok_or(OwlError::TomlError(
                format!("Invalid entry '{}' in manifest", &ext_uri_key),
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
        Uri::Local(path) => toml_utils::read_toml(&path)?,
        Uri::Remote(url) => toml_utils::request_toml(&url).await?,
    };

    let tmp_archive = Path::new(TMP_ARCHIVE);

    if let Some(quests_table) = ext_doc["quests"].as_table() {
        let mut quest_path = manifest_path
            .parent()
            .expect("owlgo directory to exist")
            .to_path_buf();

        for (quest_name, quest_uri) in quests_table.iter() {
            quest_path.push(quest_name);

            let quest_uri_str = quest_uri.as_str().ok_or(OwlError::TomlError(
                format!("Invalid entry '{}' in extension '{}'", quest_name, ext_name),
                "None".into(),
            ))?;

            match Uri::try_from(quest_uri_str)? {
                Uri::Local(path) => {
                    fs_utils::extract_archive(&path, tmp_archive)?;
                    fs_utils::remove_path(tmp_archive)?
                }
                Uri::Remote(url) => {
                    fs_utils::download_archive(&url, tmp_archive, &quest_path).await?
                }
            };

            quest_path.pop();
        }
    }

    if let Some(prompt_table) = ext_doc["prompts"].as_table() {
        let mut prompt_path =
            fs_utils::ensure_path_from_home(&[OWL_DIR, STASH_DIR, PROMPT_DIR], None)?;

        for (prompt_name, prompt_uri) in prompt_table.iter() {
            let prompt_uri_str = prompt_uri.as_str().ok_or(OwlError::TomlError(
                format!(
                    "Invalid entry '{}' in extension '{}'",
                    prompt_name, ext_name
                ),
                "None".into(),
            ))?;

            prompt_path.push(prompt_name);

            match Uri::try_from(prompt_uri_str)? {
                Uri::Local(path) => fs_utils::copy_file(&path, &prompt_path)?,
                Uri::Remote(url) => fs_utils::download_file(&url, &prompt_path).await?,
            };

            prompt_path.pop();
        }
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
        Uri::Local(path) => fs_utils::extract_archive(&path, &quest_dir),
        Uri::Remote(url) => {
            fs_utils::download_archive(&url, Path::new(TMP_ARCHIVE), &quest_dir).await
        }
    }
}
