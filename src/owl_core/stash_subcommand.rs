use crate::common::{OwlError, Result};
use crate::owl_utils::fs_utils;
use crate::{OWL_DIR, PROMPT_DIR, STASH_DIR, TEMPLATE_STEM};
use std::ffi::OsStr;
use std::path::Path;

pub fn stash_file(prog: &Path, as_templ: bool, as_prompt: bool) -> Result<()> {
    let prog_file_name = prog
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or(OwlError::UriError(
            format!("'{}': has no filename", prog.to_string_lossy()),
            "".into(),
        ))?;

    if as_prompt {
        let stash_path = fs_utils::ensure_path_from_home(
            &[OWL_DIR, STASH_DIR, PROMPT_DIR],
            Some(prog_file_name),
        )?;

        fs_utils::copy_file(prog, &stash_path)
    } else {
        let stash_path = if as_templ {
            let prog_ext = prog
                .extension()
                .and_then(OsStr::to_str)
                .ok_or(OwlError::UriError(
                    format!("'{}': has no file extension", prog.to_string_lossy()),
                    "".into(),
                ))?;
            let stash_file = format!("{}.{}", TEMPLATE_STEM, prog_ext);

            fs_utils::ensure_path_from_home(&[OWL_DIR, STASH_DIR], Some(&stash_file))?
        } else {
            fs_utils::ensure_path_from_home(&[OWL_DIR, STASH_DIR], Some(prog_file_name))?
        };

        fs_utils::copy_file(prog, &stash_path)
    }
}
