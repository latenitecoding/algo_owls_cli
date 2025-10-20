use crate::common::OwlError;
use crate::owl_utils::{fs_utils, git_utils};
use crate::{GIT_DIR, OWL_DIR, STASH_DIR};

pub fn sync_git_remote(use_force: bool) -> Result<(), OwlError> {
    let git_path = fs_utils::ensure_path_from_home(&[OWL_DIR, STASH_DIR], Some(GIT_DIR))?;

    if git_path.exists() && !use_force {
        return Err(OwlError::FileError(
            ".git directory already exists in stash".into(),
            "".into(),
        ));
    }

    let stash_dir = git_path.parent().expect("stash directory to exist");

    let mut git_cmd = git_utils::git_fetch(stash_dir, "origin", "main");

    if use_force {
        git_cmd = git_cmd.and_then(|stdout| {
            println!("{}", stdout);

            git_utils::git_reset(stash_dir, "origin", "main")
        })
    }

    git_cmd
        .and_then(|stdout| {
            println!("{}", stdout);

            git_utils::git_pull(stash_dir, "origin", "main")
        })
        .and_then(|stdout| {
            println!("{}", stdout);

            git_utils::git_status(stash_dir)
        })
        .map(|stdout| println!("{}", stdout))
}
