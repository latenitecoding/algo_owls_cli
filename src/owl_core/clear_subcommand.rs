use crate::common::{OwlError, Result};
use crate::owl_utils::fs_utils;
use crate::{CHAT_DIR, GIT_DIR, OWL_DIR, PROMPT_DIR, STASH_DIR};
use std::ffi::OsStr;
use std::fs;

pub fn clear_programs() -> Result<()> {
    let stash_dir = fs_utils::ensure_path_from_home(&[OWL_DIR], Some(STASH_DIR))?;

    if !stash_dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(&stash_dir)
        .map_err(|e| OwlError::FileError("could not read stash dir".into(), e.to_string()))?
    {
        let path = entry
            .map_err(|e| {
                OwlError::FileError("could not read entry in stash dir".into(), e.to_string())
            })?
            .path();

        let stem = path
            .file_stem()
            .and_then(OsStr::to_str)
            .ok_or(OwlError::UriError(
                format!("'{}': has no file stem", path.to_string_lossy()),
                "".into(),
            ))?;

        if path.is_dir() && (stem == PROMPT_DIR || stem == GIT_DIR) {
            continue;
        }

        fs_utils::remove_path(&path)?;
    }

    Ok(())
}

pub fn clear_quests() -> Result<()> {
    let owl_dir = fs_utils::ensure_path_from_home(&[OWL_DIR], None)?;

    if !owl_dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(&owl_dir)
        .map_err(|e| OwlError::FileError("could not read owlgo dir".into(), e.to_string()))?
    {
        let path = entry
            .map_err(|e| {
                OwlError::FileError("could not read entry in owlgo dir".into(), e.to_string())
            })?
            .path();

        let stem = path
            .file_stem()
            .and_then(OsStr::to_str)
            .ok_or(OwlError::UriError(
                format!("'{}': has no file stem", path.to_string_lossy()),
                "".into(),
            ))?;

        if path.is_file() || (stem == CHAT_DIR || stem == STASH_DIR) {
            continue;
        }

        fs_utils::remove_path(&path)?;
    }

    Ok(())
}
