use crate::common::OwlError;
use crate::owl_utils::{fs_utils, git_utils};
use crate::{GIT_DIR, OWL_DIR, STASH_DIR};

pub fn set_git_remote(remote: &str, use_force: bool) -> Result<(), OwlError> {
    let git_path = fs_utils::ensure_path_from_home(&[OWL_DIR, STASH_DIR], Some(GIT_DIR))?;

    if git_path.exists() && !use_force {
        return Err(OwlError::FileError(
            ".git directory already exists in stash".into(),
            "".into(),
        ));
    }

    if git_path.exists() && use_force {
        fs_utils::remove_path(&git_path)?;
    }

    let stash_dir = git_path.parent().expect("stash directory to exist");

    git_utils::git_init(stash_dir)
        .and_then(|stdout| {
            println!("{}", stdout);

            git_utils::git_remote_add(stash_dir, "origin", remote)
        })
        .and_then(|stdout| {
            println!("{}", stdout);

            git_utils::git_checkout(stash_dir, "main")
        })
        .and_then(|stdout| {
            println!("{}", stdout);

            git_utils::git_status(stash_dir)
        })
        .map(|stdout| println!("{}", stdout))
}
